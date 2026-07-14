//! Liest die Live-Session-Marker `~/.claude/sessions/<PID>.json`.
//! Diese Dateien schreibt Claude Code pro interaktivem CLI-Prozess; das Feld
//! `status` (idle/busy/waiting) ist das primaere Live-Signal.

use crate::claude_paths;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct RawSession {
    pub pid: u32,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: i64,
    #[serde(rename = "startedAt", default)]
    pub started_at: Option<i64>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Liest alle vorhandenen Session-Marker.
pub fn read_sessions() -> Vec<RawSession> {
    let Some(dir) = claude_paths::sessions_dir() else {
        return vec![];
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return vec![];
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<RawSession>(&content) {
                out.push(session);
            }
        }
    }
    out
}
