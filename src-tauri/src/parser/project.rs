use std::path::Path;

/// Decodes a Claude Code project directory name back to a human-readable path.
///
/// Claude Code encodes project paths by replacing all `/` with `-`.
/// e.g. "-Users-alice-Code-myproject" → "/Users/alice/Code/myproject"
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

    #[test]
    fn test_decode_project_path() {
        assert_eq!(
            decode_project_path("-Users-alice-Code-myproject"),
            "/Users/alice/Code/myproject"
        );
        assert_eq!(
            decode_project_path("-Users-alice-Desktop-my-project"),
            "/Users/alice/Desktop/my/project"
        );
    }

    #[test]
    fn test_display_name() {
        assert_eq!(display_name("/Users/alice/Code/myproject"), "myproject");
        assert_eq!(display_name("/Users/alice/Desktop"), "Desktop");
    }
}
