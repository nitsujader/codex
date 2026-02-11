use std::path::Path;
use std::path::PathBuf;

use sha1::Digest as _;

use crate::path_utils;

const PROJECT_MEMORIES_DIR: &str = "project-memories";

fn normalize_cwd_for_key(cwd: &Path) -> PathBuf {
    path_utils::normalize_for_path_comparison(cwd).unwrap_or_else(|_| cwd.to_path_buf())
}

fn memory_key(cwd: &Path) -> String {
    let normalized = normalize_cwd_for_key(cwd);
    let key = normalized.display().to_string();
    let mut hasher = sha1::Sha1::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn project_memory_path(codex_home: &Path, cwd: &Path) -> PathBuf {
    let key = memory_key(cwd);
    codex_home
        .join(PROJECT_MEMORIES_DIR)
        .join(format!("{key}.md"))
}

pub async fn read_project_memory(codex_home: &Path, cwd: &Path) -> Option<String> {
    let path = project_memory_path(codex_home, cwd);
    let Ok(text) = tokio::fs::read_to_string(path).await else {
        return None;
    };
    let trimmed = text.trim();
    (!trimmed.is_empty()).then_some(trimmed.to_string())
}

pub async fn write_project_memory(
    codex_home: &Path,
    cwd: &Path,
    contents: &str,
) -> std::io::Result<PathBuf> {
    let path = project_memory_path(codex_home, cwd);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, contents).await?;
    Ok(path)
}

/// Overwrite the project memory with a new summary, but only if a memory file
/// already exists for this cwd.
///
/// This makes project memories opt-in: once the user creates/edits a memory,
/// future compactions can keep it fresh.
pub async fn maybe_update_project_memory_from_compaction_summary(
    codex_home: &Path,
    cwd: &Path,
    summary_text: &str,
) -> std::io::Result<Option<PathBuf>> {
    let path = project_memory_path(codex_home, cwd);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(None);
    }

    let updated = format!(
        "# Project Memory\n\n- cwd: `{}`\n- updated_at: `{}`\n\n---\n\n{}\n",
        cwd.display(),
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        summary_text.trim()
    );
    write_project_memory(codex_home, cwd, &updated)
        .await
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[tokio::test]
    async fn reads_and_writes_project_memory() {
        let home = TempDir::new().unwrap();
        let cwd = PathBuf::from("C:\\tmp\\proj");
        let text = "hello";
        let path = write_project_memory(home.path(), cwd.as_path(), text)
            .await
            .unwrap();
        assert!(path.exists());

        let read = read_project_memory(home.path(), cwd.as_path()).await;
        assert_eq!(read, Some(text.to_string()));
    }
}
