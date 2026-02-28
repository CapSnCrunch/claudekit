use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead};
use std::path::Path;

/// A single line parsed from a Claude Code `.jsonl` session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    #[serde(rename = "type")]
    pub entry_type: String, // "user" | "assistant" | "system" | "summary"
    pub uuid: String,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub timestamp: String,
    pub message: Option<AnthropicMessage>,
    #[serde(rename = "costUSD")]
    pub cost_usd: Option<f64>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub id: Option<String>,
    pub role: Option<String>,
    pub content: Option<Value>, // kept as raw Value — frontend parses the content blocks
    pub model: Option<String>,
    #[serde(rename = "stop_reason")]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
}

/// Parses all valid entries from a `.jsonl` file.
/// Malformed lines are skipped with a warning; the parse continues.
pub fn parse_file(path: &Path) -> io::Result<Vec<JournalEntry>> {
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut entries = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<JournalEntry>(trimmed) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                log::warn!(
                    "Skipping malformed line {} in {}: {}",
                    line_num + 1,
                    path.display(),
                    e
                );
            }
        }
    }

    Ok(entries)
}

/// Extracts the plain text from the first user-role entry, for use as a session title.
pub fn extract_title(entries: &[JournalEntry]) -> Option<String> {
    entries
        .iter()
        .find(|e| {
            e.message
                .as_ref()
                .and_then(|m| m.role.as_deref())
                == Some("user")
        })
        .and_then(|e| e.message.as_ref())
        .and_then(|m| m.content.as_ref())
        .and_then(|content| {
            // content may be a string or an array of blocks
            if let Some(s) = content.as_str() {
                return Some(truncate(s, 80));
            }
            if let Some(arr) = content.as_array() {
                for block in arr {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            return Some(truncate(text, 80));
                        }
                    }
                }
            }
            None
        })
}

fn truncate(s: &str, max_chars: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}…", truncated.trim_end())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Helper: write lines to a temp file and return the path.
    fn tmp_jsonl(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{}", line).unwrap();
        }
        f
    }

    // Minimal valid JSONL lines.
    const USER_STR: &str = r#"{"type":"user","uuid":"u1","sessionId":"s1","timestamp":"2025-01-15T10:00:00Z","message":{"role":"user","content":"hello world"}}"#;
    const ASST_STR: &str = r#"{"type":"assistant","uuid":"u2","sessionId":"s1","timestamp":"2025-01-15T10:00:01Z","message":{"role":"assistant","content":"hi there"}}"#;
    const USER_BLOCKS: &str = r#"{"type":"user","uuid":"u3","sessionId":"s1","timestamp":"2025-01-15T10:00:02Z","message":{"role":"user","content":[{"type":"text","text":"block message"}]}}"#;
    const USER_TOOL_RESULT: &str = r#"{"type":"user","uuid":"u4","sessionId":"s1","timestamp":"2025-01-15T10:00:03Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"done"}]}}"#;

    // ── parse_file ────────────────────────────────────────────────────────────

    #[test]
    fn parse_file_returns_all_valid_entries() {
        let f = tmp_jsonl(&[USER_STR, ASST_STR]);
        let entries = parse_file(f.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_type, "user");
        assert_eq!(entries[0].uuid, "u1");
        assert_eq!(entries[1].entry_type, "assistant");
    }

    #[test]
    fn parse_file_skips_malformed_lines_and_continues() {
        let f = tmp_jsonl(&[USER_STR, "not json {{{{", ASST_STR]);
        let entries = parse_file(f.path()).unwrap();
        assert_eq!(entries.len(), 2, "malformed line must be skipped, rest parsed");
        assert_eq!(entries[0].uuid, "u1");
        assert_eq!(entries[1].uuid, "u2");
    }

    #[test]
    fn parse_file_empty_file_returns_empty_vec() {
        let f = tmp_jsonl(&[]);
        assert!(parse_file(f.path()).unwrap().is_empty());
    }

    #[test]
    fn parse_file_skips_blank_and_whitespace_lines() {
        let f = tmp_jsonl(&["", "   ", USER_STR, "\t"]);
        let entries = parse_file(f.path()).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn parse_file_deserialises_content_as_string() {
        let f = tmp_jsonl(&[USER_STR]);
        let e = &parse_file(f.path()).unwrap()[0];
        let content = e.message.as_ref().unwrap().content.as_ref().unwrap();
        assert!(content.is_string());
        assert_eq!(content.as_str().unwrap(), "hello world");
    }

    #[test]
    fn parse_file_deserialises_content_as_array() {
        let f = tmp_jsonl(&[USER_BLOCKS]);
        let e = &parse_file(f.path()).unwrap()[0];
        let content = e.message.as_ref().unwrap().content.as_ref().unwrap();
        assert!(content.is_array());
    }

    #[test]
    fn parse_file_preserves_optional_fields_as_none_when_absent() {
        let f = tmp_jsonl(&[USER_STR]);
        let e = &parse_file(f.path()).unwrap()[0];
        assert!(e.parent_uuid.is_none());
        assert!(e.cost_usd.is_none());
        assert!(e.usage.is_none());
    }

    // ── extract_title ─────────────────────────────────────────────────────────

    fn e(json: &str) -> JournalEntry {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn extract_title_from_plain_string_content() {
        let entries = vec![e(USER_STR)];
        assert_eq!(extract_title(&entries).unwrap(), "hello world");
    }

    #[test]
    fn extract_title_from_text_block_array() {
        let entries = vec![e(USER_BLOCKS)];
        assert_eq!(extract_title(&entries).unwrap(), "block message");
    }

    #[test]
    fn extract_title_skips_tool_result_only_entry() {
        // tool_result-only entry has no text block → returns None
        let entries = vec![e(USER_TOOL_RESULT)];
        assert!(extract_title(&entries).is_none());
    }

    #[test]
    fn extract_title_uses_first_user_entry() {
        // assistant comes first, then user — should use the user entry
        let entries = vec![e(ASST_STR), e(USER_STR)];
        assert_eq!(extract_title(&entries).unwrap(), "hello world");
    }

    #[test]
    fn extract_title_returns_none_for_no_user_entries() {
        let entries = vec![e(ASST_STR)];
        assert!(extract_title(&entries).is_none());
    }

    #[test]
    fn extract_title_returns_none_for_empty_entries() {
        assert!(extract_title(&[]).is_none());
    }

    // ── truncate ──────────────────────────────────────────────────────────────

    #[test]
    fn truncate_short_string_returned_as_is() {
        assert_eq!(truncate("hello", 80), "hello");
    }

    #[test]
    fn truncate_exactly_max_chars_returned_as_is() {
        let s = "a".repeat(80);
        assert_eq!(truncate(&s, 80), s);
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        let s = "a".repeat(81);
        let result = truncate(&s, 80);
        assert!(result.ends_with('…'), "should end with ellipsis");
        // char count of result body (without ellipsis) should be 80
        let body: String = result.chars().take(80).collect();
        assert_eq!(body.chars().count(), 80);
    }

    #[test]
    fn truncate_trims_leading_and_trailing_whitespace() {
        assert_eq!(truncate("  hello  ", 80), "hello");
    }

    #[test]
    fn extract_title_long_content_is_truncated() {
        let long = "x".repeat(100);
        let json = format!(
            r#"{{"type":"user","uuid":"u","sessionId":"s","timestamp":"2025-01-01T00:00:00Z","message":{{"role":"user","content":"{long}"}}}}"#
        );
        let entries = vec![e(&json)];
        let title = extract_title(&entries).unwrap();
        assert!(title.ends_with('…'));
        assert!(title.chars().count() <= 81); // 80 chars + ellipsis
    }
}
