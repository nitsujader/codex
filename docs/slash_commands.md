# Slash commands

For an overview of Codex CLI slash commands, see [this documentation](https://developers.openai.com/codex/cli/slash-commands).

## Local additions

This repo also includes some TUI-focused commands:

- `/export`: export the current conversation rollout to a Markdown file under `CODEX_HOME/exports/`
- `/stream`: toggle streaming the active thread to `CODEX_HOME/exports/live-<threadId>.md`
- `/screenshot`: capture a screenshot and attach it (Windows-only today)
- `/theme`: switch visual themes (`default`, `fallout`, `cyberpunk`, `matrix`)
- `/memory`: view/edit project memory for the current working directory
