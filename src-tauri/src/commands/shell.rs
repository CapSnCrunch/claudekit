use std::process::Command;

/// Open a session/project in a named external application.
///
/// `app`          – one of: "claude_code" | "cursor" | "claude_desktop"
/// `project_path` – absolute filesystem path of the project directory
/// `session_id`   – Claude Code session UUID (used for `claude --resume`)
#[tauri::command]
pub fn open_in_app(
    app: String,
    project_path: String,
    session_id: Option<String>,
) -> Result<(), String> {
    match app.as_str() {
        "claude_code" => open_in_claude_code(&project_path, session_id.as_deref()),
        "cursor" => open_in_cursor(&project_path),
        "claude_desktop" => open_claude_desktop(),
        other => Err(format!("Unknown app: {other}")),
    }
}

// ── per-app helpers ────────────────────────────────────────────────────────────

/// Opens a new terminal window, cd's to `project_path`, and runs
/// `claude --resume SESSION_ID` (or just `claude` if no session id).
fn open_in_claude_code(project_path: &str, session_id: Option<&str>) -> Result<(), String> {
    // Single-quote the path for shell safety; escape any embedded single quotes.
    let safe_path = project_path.replace('\'', "'\\''");
    let cmd = match session_id {
        Some(id) if !id.is_empty() => {
            format!("cd '{}' && claude --resume {}", safe_path, id)
        }
        _ => format!("cd '{}' && claude", safe_path),
    };

    #[cfg(target_os = "macos")]
    {
        // AppleScript: open Terminal.app and run the command in a new window.
        // The shell command uses single-quoted paths, so no conflict with
        // the AppleScript double-quoted string literal.
        let script = format!(
            r#"tell application "Terminal"
    do script "{}"
    activate
end tell"#,
            cmd
        );
        Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| format!("osascript failed: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try common terminal emulators in order.
        let terminals = ["x-terminal-emulator", "xterm", "gnome-terminal", "konsole"];
        let mut launched = false;
        for term in terminals {
            if Command::new(term)
                .args(["-e", "bash", "-c", &cmd])
                .spawn()
                .is_ok()
            {
                launched = true;
                break;
            }
        }
        if !launched {
            return Err("No supported terminal emulator found".to_string());
        }
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "start", "cmd", "/k", &cmd])
            .spawn()
            .map_err(|e| format!("cmd failed: {e}"))?;
    }

    Ok(())
}

/// Opens `project_path` in Cursor via the `cursor` CLI.
fn open_in_cursor(project_path: &str) -> Result<(), String> {
    // Try the `cursor` CLI first (works if Cursor's bin is on PATH).
    let result = Command::new("cursor").arg(project_path).spawn();

    if result.is_ok() {
        return Ok(());
    }

    // macOS fallback: open via the .app bundle.
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Cursor", project_path])
            .spawn()
            .map_err(|e| format!("open -a Cursor failed: {e}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Cursor not found. Install Cursor and make sure the `cursor` command is on your PATH.".to_string())
}

/// Launches the Claude Desktop app.
fn open_claude_desktop() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Claude"])
            .spawn()
            .map_err(|e| format!("open -a Claude failed: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        // Claude Desktop for Linux — try the binary name that ships with the deb/AppImage.
        Command::new("claude-desktop")
            .spawn()
            .map_err(|e| format!("claude-desktop failed: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "start", "claude:"])
            .spawn()
            .map_err(|e| format!("start claude: failed: {e}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Claude Desktop launch not supported on this platform.".to_string())
}
