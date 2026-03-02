use std::path::Path;
use serde::Deserialize;

#[derive(Deserialize)]
struct SessionsIndex {
    entries: Vec<SessionEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionEntry {
    project_path: String,
}

/// Reads the actual project path from sessions-index.json.
/// This is the source of truth for the project path, as it preserves
/// directory names with dashes (e.g., "campaign-manager").
pub fn read_project_path(project_dir: &Path) -> Result<String, String> {
    let index_path = project_dir.join("sessions-index.json");

    if !index_path.exists() {
        return Err("sessions-index.json not found".to_string());
    }

    let contents = std::fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read sessions-index.json: {e}"))?;

    let index: SessionsIndex = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse sessions-index.json: {e}"))?;

    // Get the project path from the first entry (all entries should have the same project path)
    index
        .entries
        .first()
        .map(|e| e.project_path.clone())
        .ok_or_else(|| "No entries in sessions-index.json".to_string())
}

/// Decodes a Claude Code project directory name back to a human-readable path.
///
/// **Note:** This is a fallback for when sessions-index.json doesn't exist.
/// It has a known limitation: it cannot distinguish between dashes in the path
/// separator vs. dashes in directory names (e.g., "campaign-manager" becomes "campaign/manager").
/// Prefer using `read_project_path()` which reads from sessions-index.json.
pub fn decode_project_path(dir_name: &str) -> String {
    // Each `-` that starts a path segment represents a `/`.
    // The leading `-` represents the leading `/`.
    dir_name.replace('-', "/")
}

/// Returns the display name (last path component) for a decoded project path.
pub fn display_name(decoded_path: &str) -> String {
    Path::new(decoded_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(decoded_path)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_decode_project_path() {
        // This is a fallback with known limitations
        assert_eq!(
            decode_project_path("-Users-alice-Code-myproject"),
            "/Users/alice/Code/myproject"
        );
        // Note: This test shows the limitation - dashes in directory names
        // are incorrectly treated as path separators
        assert_eq!(
            decode_project_path("-Users-alice-Desktop-my-project"),
            "/Users/alice/Desktop/my/project"
        );
    }

    #[test]
    fn test_read_project_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();

        // Create a sessions-index.json with a project path containing dashes
        let index_json = r#"{
            "version": 1,
            "entries": [
                {
                    "sessionId": "test-id",
                    "projectPath": "/Users/alice/Code/campaign-manager"
                }
            ]
        }"#;

        fs::write(project_dir.join("sessions-index.json"), index_json).unwrap();

        let path = read_project_path(project_dir).unwrap();
        assert_eq!(path, "/Users/alice/Code/campaign-manager");
    }

    #[test]
    fn test_read_project_path_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = read_project_path(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_display_name() {
        assert_eq!(display_name("/Users/alice/Code/myproject"), "myproject");
        assert_eq!(display_name("/Users/alice/Desktop"), "Desktop");
        assert_eq!(
            display_name("/Users/alice/Code/campaign-manager"),
            "campaign-manager"
        );
    }
}
