use std::path::Path;

use codex_protocol::protocol::AgentMessageEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;
use codex_protocol::protocol::SessionMetaLine;
use codex_protocol::protocol::TokenUsageInfo;
use codex_protocol::protocol::TurnContextItem;
use codex_protocol::user_input::TextElement;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ExportMarkdownOptions {
    pub include_tool_io: bool,
    pub include_reasoning_summaries: bool,
    pub include_token_usage: bool,
    pub include_turn_context: bool,
    pub redact_secrets: bool,
    /// Number of recent user/assistant messages to include as "recent reviewed"
    /// around compaction markers.
    pub recent_review_messages: usize,
}

impl Default for ExportMarkdownOptions {
    fn default() -> Self {
        Self {
            include_tool_io: true,
            include_reasoning_summaries: true,
            include_token_usage: true,
            include_turn_context: false,
            redact_secrets: true,
            recent_review_messages: 8,
        }
    }
}

#[derive(Clone, Debug)]
struct RecentMessage {
    role: &'static str,
    timestamp: String,
    text: String,
}

pub async fn export_rollout_to_markdown(
    rollout_path: &Path,
    options: &ExportMarkdownOptions,
) -> std::io::Result<String> {
    let raw = tokio::fs::read_to_string(rollout_path).await?;
    let mut parsed = Vec::<RolloutLine>::new();
    let mut parse_errors = 0usize;
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<RolloutLine>(line) {
            Ok(parsed_line) => parsed.push(parsed_line),
            Err(_) => parse_errors = parse_errors.saturating_add(1),
        }
    }

    if parsed.is_empty() {
        return Err(std::io::Error::other(
            "rollout file contained no readable items",
        ));
    }

    let mut meta: Option<SessionMetaLine> = None;
    let mut last_turn_context: Option<TurnContextItem> = None;
    let mut last_token_usage: Option<TokenUsageInfo> = None;

    let mut recent_messages: Vec<RecentMessage> = Vec::new();

    let mut out = String::new();
    out.push_str("# Codex Conversation Export\n\n");
    out.push_str(&format!(
        "- Exported: {}\n",
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    ));
    out.push_str(&format!(
        "- Rollout: `{}`\n",
        rollout_path.display().to_string().replace('`', "\\`")
    ));
    if parse_errors > 0 {
        out.push_str(&format!("- Parse errors: {parse_errors}\n"));
    }
    out.push('\n');

    for line in &parsed {
        match &line.item {
            RolloutItem::SessionMeta(session_meta) => {
                if meta.is_none() {
                    meta = Some(session_meta.clone());
                }
            }
            RolloutItem::TurnContext(ctx) => last_turn_context = Some(ctx.clone()),
            RolloutItem::EventMsg(EventMsg::TokenCount(ev)) => last_token_usage = ev.info.clone(),
            _ => {}
        }
    }

    if let Some(meta) = meta.as_ref() {
        out.push_str("## Session\n\n");
        out.push_str(&format!("- Thread: `{}`\n", meta.meta.id));
        if !meta.meta.timestamp.is_empty() {
            out.push_str(&format!(
                "- Started: `{}`\n",
                md_escape_inline(&meta.meta.timestamp)
            ));
        }
        if !meta.meta.cwd.as_os_str().is_empty() {
            out.push_str(&format!(
                "- CWD: `{}`\n",
                md_escape_inline(meta.meta.cwd.display().to_string().as_str())
            ));
        }
        if !meta.meta.originator.is_empty() {
            out.push_str(&format!(
                "- Originator: `{}`\n",
                md_escape_inline(&meta.meta.originator)
            ));
        }
        if !meta.meta.cli_version.is_empty() {
            out.push_str(&format!(
                "- CLI: `{}`\n",
                md_escape_inline(&meta.meta.cli_version)
            ));
        }
        out.push_str(&format!(
            "- Source: `{}`\n",
            md_escape_inline(&format!("{:?}", meta.meta.source))
        ));
        if let Some(provider) = &meta.meta.model_provider
            && !provider.is_empty()
        {
            out.push_str(&format!("- Provider: `{}`\n", md_escape_inline(provider)));
        }
        out.push('\n');
    }

    if options.include_turn_context
        && let Some(ctx) = last_turn_context.as_ref()
    {
        out.push_str("## Turn Context (Latest)\n\n");
        out.push_str(&format!(
            "- Model: `{}`\n",
            md_escape_inline(ctx.model.as_str())
        ));
        out.push_str(&format!(
            "- Approval policy: `{}`\n",
            md_escape_inline(&format!("{:?}", ctx.approval_policy))
        ));
        out.push_str(&format!(
            "- Sandbox policy: `{}`\n",
            md_escape_inline(&format!("{:?}", ctx.sandbox_policy))
        ));
        out.push_str(&format!(
            "- CWD: `{}`\n",
            md_escape_inline(ctx.cwd.display().to_string().as_str())
        ));
        out.push('\n');
    }

    if options.include_token_usage
        && let Some(usage) = last_token_usage.as_ref()
    {
        out.push_str("## Token Usage (Latest)\n\n");
        out.push_str(&format!(
            "- Total tokens: `{}`\n",
            usage.total_token_usage.total_tokens
        ));
        out.push_str(&format!(
            "- Input tokens: `{}`\n",
            usage.total_token_usage.input_tokens
        ));
        out.push_str(&format!(
            "- Output tokens: `{}`\n",
            usage.total_token_usage.output_tokens
        ));
        if usage.total_token_usage.reasoning_output_tokens > 0 {
            out.push_str(&format!(
                "- Reasoning tokens: `{}`\n",
                usage.total_token_usage.reasoning_output_tokens
            ));
        }
        out.push('\n');
    }

    out.push_str("## Transcript\n\n");

    for line in parsed {
        match line.item {
            RolloutItem::SessionMeta(_) => {}
            RolloutItem::TurnContext(ctx) => {
                if options.include_turn_context {
                    out.push_str(&format!(
                        "<details>\n<summary>Turn context ({})</summary>\n\n",
                        md_escape_inline(&line.timestamp)
                    ));
                    out.push_str("```json\n");
                    out.push_str(&pretty_json(&ctx)?);
                    out.push_str("\n```\n\n</details>\n\n");
                }
            }
            RolloutItem::Compacted(compacted) => {
                out.push_str(&format!(
                    "### Compaction ({})\n\n",
                    md_escape_inline(&line.timestamp)
                ));

                if !compacted.message.trim().is_empty() {
                    let msg = maybe_redact(&compacted.message, options);
                    out.push_str("<details>\n<summary>Compaction summary</summary>\n\n");
                    out.push_str("```text\n");
                    out.push_str(&msg);
                    if !msg.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("```\n\n</details>\n\n");
                } else if compacted.replacement_history.is_some() {
                    out.push_str(
                        "_Remote compaction: replacement history recorded in rollout._\n\n",
                    );
                } else {
                    out.push_str("_Compaction recorded (no summary text)._\\\n\n");
                }

                let n = options.recent_review_messages;
                if n > 0 && !recent_messages.is_empty() {
                    out.push_str("#### Recent Reviewed\n\n");
                    for msg in recent_messages.iter().rev().take(n).rev() {
                        out.push_str(&format!(
                            "- **{}** ({})\n",
                            msg.role,
                            md_escape_inline(&msg.timestamp)
                        ));
                        out.push_str(&blockquote(&maybe_redact(&msg.text, options)));
                        out.push('\n');
                    }
                    out.push('\n');
                }
            }
            RolloutItem::EventMsg(ev) => match ev {
                EventMsg::UserMessage(user) => {
                    let message = maybe_redact(&user.message, options);
                    out.push_str(&format!(
                        "### User ({})\n\n",
                        md_escape_inline(&line.timestamp)
                    ));
                    if !message.trim().is_empty() {
                        out.push_str(&blockquote(&message));
                        out.push('\n');
                    }

                    if let Some(urls) = user.images.as_ref() {
                        for url in urls {
                            out.push_str(&blockquote(&format!("[image url] {url}")));
                            out.push('\n');
                        }
                    }
                    for path in &user.local_images {
                        out.push_str(&blockquote(&format!("[image file] {}", path.display())));
                        out.push('\n');
                    }
                    if !user.text_elements.is_empty() {
                        out.push_str("<details>\n<summary>text elements</summary>\n\n```json\n");
                        out.push_str(&pretty_json(&user.text_elements)?);
                        out.push_str("\n```\n\n</details>\n\n");
                    }

                    push_recent(&mut recent_messages, "user", line.timestamp, user.message);
                }
                EventMsg::AgentMessage(AgentMessageEvent { message }) => {
                    let message = maybe_redact(&message, options);
                    out.push_str(&format!(
                        "### Assistant ({})\n\n",
                        md_escape_inline(&line.timestamp)
                    ));
                    if !message.trim().is_empty() {
                        out.push_str(&message);
                        if !message.ends_with('\n') {
                            out.push('\n');
                        }
                        out.push('\n');
                    }
                    push_recent(&mut recent_messages, "assistant", line.timestamp, message);
                }
                EventMsg::AgentReasoning(ev) => {
                    if !options.include_reasoning_summaries {
                        continue;
                    }
                    let text = maybe_redact(&ev.text, options);
                    if text.trim().is_empty() {
                        continue;
                    }
                    out.push_str(&format!(
                        "<details>\n<summary>Reasoning summary ({})</summary>\n\n",
                        md_escape_inline(&line.timestamp)
                    ));
                    out.push_str("```text\n");
                    out.push_str(&text);
                    if !text.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("```\n\n</details>\n\n");
                }
                // Never export raw chain-of-thought. Include a placeholder so the absence is explicit.
                EventMsg::AgentReasoningRawContent(_)
                | EventMsg::AgentReasoningRawContentDelta(_) => {
                    out.push_str(&format!(
                        "_Raw reasoning omitted ({})_\n\n",
                        md_escape_inline(&line.timestamp)
                    ));
                }
                EventMsg::TokenCount(_) => {
                    // Already captured as "latest"; omit per-event noise for exports.
                }
                EventMsg::ViewImageToolCall(ev) => {
                    out.push_str(&format!(
                        "### Image Attached ({})\n\n",
                        md_escape_inline(&line.timestamp)
                    ));
                    out.push_str(&blockquote(&format!("path: {}", ev.path.display())));
                    out.push_str("\n\n");
                }
                _ => {}
            },
            RolloutItem::ResponseItem(item) => {
                if options.include_tool_io
                    && let Some(md) = render_response_item(&line.timestamp, &item, options)?
                {
                    out.push_str(&md);
                }
            }
        }
    }

    Ok(out)
}

fn push_recent(
    recent: &mut Vec<RecentMessage>,
    role: &'static str,
    timestamp: String,
    text: String,
) {
    recent.push(RecentMessage {
        role,
        timestamp,
        text,
    });

    // Keep the buffer bounded even if the caller requested a very large review size.
    const HARD_MAX: usize = 128;
    if recent.len() > HARD_MAX {
        let drain = recent.len() - HARD_MAX;
        recent.drain(0..drain);
    }
}

fn render_response_item(
    timestamp: &str,
    item: &codex_protocol::models::ResponseItem,
    options: &ExportMarkdownOptions,
) -> std::io::Result<Option<String>> {
    use codex_protocol::models::ResponseItem as RI;

    let (summary, body) = match item {
        RI::Message { .. } | RI::Reasoning { .. } => return Ok(None),
        RI::FunctionCall {
            name,
            arguments,
            call_id,
            ..
        } => (
            format!("tool call: {name} ({call_id})"),
            Some(format_json_or_text(arguments.as_str(), options)),
        ),
        RI::FunctionCallOutput { call_id, output } => (
            format!("tool output ({call_id})"),
            Some(format_json_or_text(
                &serde_json::to_string(output)
                    .unwrap_or_else(|_| "<unserializable tool output>".to_string()),
                options,
            )),
        ),
        RI::LocalShellCall {
            status,
            action,
            call_id,
            ..
        } => {
            let call_id = call_id.as_deref().unwrap_or("unknown");
            (
                format!("local shell: {status:?} ({call_id})"),
                Some(format!(
                    "```json\n{}\n```",
                    maybe_redact_json(
                        &serde_json::to_value(action).unwrap_or(Value::Null),
                        options
                    )
                )),
            )
        }
        RI::CustomToolCall {
            name,
            input,
            call_id,
            status,
            ..
        } => {
            let status = status.as_deref().unwrap_or("unknown");
            (
                format!("custom tool call: {name} ({call_id}, {status})"),
                Some(format_json_or_text(input.as_str(), options)),
            )
        }
        RI::CustomToolCallOutput { call_id, output } => (
            format!("custom tool output ({call_id})"),
            Some(format_json_or_text(output.as_str(), options)),
        ),
        RI::WebSearchCall { status, action, .. } => (
            format!("web search ({})", status.as_deref().unwrap_or("unknown")),
            action.as_ref().map(|action| {
                format!(
                    "```json\n{}\n```",
                    maybe_redact_json(
                        &serde_json::to_value(action).unwrap_or(Value::Null),
                        options
                    )
                )
            }),
        ),
        RI::GhostSnapshot { .. } => ("ghost snapshot".to_string(), None),
        RI::Compaction { .. } => ("compaction artifact".to_string(), None),
        RI::Other => return Ok(None),
    };

    let mut out = String::new();
    out.push_str(&format!(
        "<details>\n<summary>{} ({})</summary>\n\n",
        md_escape_inline(&summary),
        md_escape_inline(timestamp)
    ));
    if let Some(body) = body {
        out.push_str(&body);
        out.push('\n');
    }
    out.push_str("\n</details>\n\n");
    Ok(Some(out))
}

fn format_json_or_text(input: &str, options: &ExportMarkdownOptions) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "```text\n\n```".to_string();
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return format!("```json\n{}\n```", maybe_redact_json(&value, options));
    }

    let redacted = maybe_redact(trimmed, options);
    format!("```text\n{redacted}\n```")
}

fn pretty_json<T: serde::Serialize + ?Sized>(value: &T) -> std::io::Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| std::io::Error::other(format!("failed to serialize json: {err}")))
}

fn maybe_redact(text: &str, options: &ExportMarkdownOptions) -> String {
    if !options.redact_secrets {
        return text.to_string();
    }

    // Best-effort redaction. Keep this intentionally conservative; callers can disable.
    let mut out = text.to_string();

    // OpenAI-style keys.
    out = redact_prefix_like(&out, "sk-");
    out = redact_prefix_like(&out, "rk-");

    // Common JSON-ish fields.
    out = redact_json_field_value(&out, "api_key");
    out = redact_json_field_value(&out, "apiKey");
    out = redact_json_field_value(&out, "access_token");
    out = redact_json_field_value(&out, "accessToken");
    out = redact_json_field_value(&out, "refresh_token");
    out = redact_json_field_value(&out, "refreshToken");

    out
}

fn redact_prefix_like(input: &str, prefix: &str) -> String {
    // Replace any contiguous non-whitespace token starting with `prefix`.
    let mut out = String::with_capacity(input.len());
    for token in input.split_inclusive(char::is_whitespace) {
        if let Some((word, suffix_ws)) = token.split_once(|c: char| c.is_whitespace()) {
            if word.starts_with(prefix) && word.len() > prefix.len() + 8 {
                out.push_str("[REDACTED]");
            } else {
                out.push_str(word);
            }
            out.push_str(suffix_ws);
        } else if token.starts_with(prefix) && token.len() > prefix.len() + 8 {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(token);
        }
    }
    out
}

fn redact_json_field_value(input: &str, field: &str) -> String {
    // Extremely small/cheap heuristic: `"field":"..."`
    // Does not attempt full JSON parsing for streaming robustness.
    let needle = format!("\"{field}\"");
    let Some(pos) = input.find(needle.as_str()) else {
        return input.to_string();
    };
    let mut out = String::new();
    out.push_str(&input[..pos]);
    out.push_str(needle.as_str());

    let rest = &input[pos + needle.len()..];
    let Some(colon_pos) = rest.find(':') else {
        out.push_str(rest);
        return out;
    };
    out.push_str(&rest[..=colon_pos]);
    let rest = &rest[colon_pos + 1..];

    // Skip whitespace.
    let rest_trimmed = rest.trim_start();
    let whitespace_len = rest.len() - rest_trimmed.len();
    out.push_str(&rest[..whitespace_len]);

    // If the value starts with a quote, redact until the next quote.
    if let Some(stripped) = rest_trimmed.strip_prefix('"') {
        out.push('"');
        out.push_str("[REDACTED]");
        if let Some(end_quote) = stripped.find('"') {
            out.push('"');
            out.push_str(&stripped[end_quote + 1..]);
        }
        return out;
    }

    out.push_str(rest_trimmed);
    out
}

fn maybe_redact_json(value: &Value, options: &ExportMarkdownOptions) -> String {
    let Ok(mut text) = serde_json::to_string_pretty(value) else {
        return "<unserializable json>".to_string();
    };
    text = maybe_redact(&text, options);
    text
}

fn md_escape_inline(input: &str) -> String {
    // Escape backticks; we wrap values in inline code spans.
    input.replace('`', "\\`")
}

fn blockquote(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        out.push_str("> ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[allow(dead_code)]
fn _text_elements_to_json(elements: &[TextElement]) -> std::io::Result<String> {
    pretty_json(elements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::protocol::UserMessageEvent;
    use pretty_assertions::assert_eq;
    use tempfile::NamedTempFile;

    fn write_lines(file: &NamedTempFile, lines: &[RolloutLine]) {
        let mut buf = String::new();
        for line in lines {
            buf.push_str(&serde_json::to_string(line).unwrap());
            buf.push('\n');
        }
        std::fs::write(file.path(), buf).unwrap();
    }

    #[tokio::test]
    async fn export_includes_user_assistant_and_compaction_recent_review() {
        let tmp = NamedTempFile::new().unwrap();
        write_lines(
            &tmp,
            &[
                RolloutLine {
                    timestamp: "t1".to_string(),
                    item: RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                        message: "first".to_string(),
                        images: None,
                        local_images: Vec::new(),
                        text_elements: Vec::new(),
                    })),
                },
                RolloutLine {
                    timestamp: "t2".to_string(),
                    item: RolloutItem::EventMsg(EventMsg::AgentMessage(AgentMessageEvent {
                        message: "ok".to_string(),
                    })),
                },
                RolloutLine {
                    timestamp: "t3".to_string(),
                    item: RolloutItem::Compacted(codex_protocol::protocol::CompactedItem {
                        message: "summary".to_string(),
                        replacement_history: None,
                    }),
                },
            ],
        );

        let options = ExportMarkdownOptions {
            recent_review_messages: 2,
            redact_secrets: false,
            ..Default::default()
        };
        let md = export_rollout_to_markdown(tmp.path(), &options)
            .await
            .unwrap();

        assert!(md.contains("### User (t1)"));
        assert!(md.contains("> first"));
        assert!(md.contains("### Assistant (t2)"));
        assert!(md.contains("ok"));
        assert!(md.contains("### Compaction (t3)"));
        assert!(md.contains("#### Recent Reviewed"));
        // Ensure both messages are present in the review block.
        assert!(md.contains("**user** (t1)"));
        assert!(md.contains("**assistant** (t2)"));
    }

    #[test]
    fn md_escape_inline_escapes_backticks() {
        assert_eq!(md_escape_inline("a`b"), "a\\`b");
    }
}
