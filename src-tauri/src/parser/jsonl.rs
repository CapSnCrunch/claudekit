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
