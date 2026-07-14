//! Erkennt Subagents einer Session. Claude Code legt fuer jeden Subagent ein
//! eigenes Sidechain-Transcript unter `projects/<ordner>/<sessionId>/subagents/`
//! ab (inkl. verschachtelter `workflows/<wf>/agent-*.meta.json`). Die `.meta.json`
//! liefert den `agentType`; der Zustand wird aus der Schreib-Aktualitaet des
//! zugehoerigen `.jsonl` abgeleitet (kuerzlich geschrieben => aktiv).

use crate::claude_paths;
use crate::model::{SessionState, Subagent};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Ein Subagent gilt als aktiv, wenn sein Transcript in diesem Fenster zuletzt
/// geschrieben wurde.
const ACTIVE_WINDOW: Duration = Duration::from_secs(90);

pub fn scan_subagents(cwd: &str, session_id: &str) -> Vec<Subagent> {
    let Some(projects) = claude_paths::projects_dir() else {
        return vec![];
    };
    let folder = claude_paths::encode_project_folder(cwd);
    let direct = projects.join(&folder).join(session_id).join("subagents");

    // Direkter Pfad bevorzugt; sonst (z.B. abweichendes Encoding) alle
    // Projektordner nach <sessionId>/subagents durchsuchen.
    let base = if direct.is_dir() {
        direct
    } else {
        match find_subagents_dir(&projects, session_id) {
            Some(p) => p,
            None => return vec![],
        }
    };

    let mut out = Vec::new();
    collect_meta(&base, &mut out);
    out
}

fn find_subagents_dir(projects: &Path, session_id: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(projects).ok()?;
    for entry in entries.flatten() {
        let cand = entry.path().join(session_id).join("subagents");
        if cand.is_dir() {
            return Some(cand);
        }
    }
    None
}

fn collect_meta(dir: &Path, out: &mut Vec<Subagent>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Verschachtelte Workflow-Agents (subagents/workflows/<wf>/...).
            collect_meta(&path, out);
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.ends_with(".meta.json") {
            continue;
        }
        let id = name.trim_end_matches(".meta.json").to_string();

        let mut agent_type = "agent".to_string();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(t) = value.get("agentType").and_then(serde_json::Value::as_str) {
                    agent_type = t.to_string();
                }
            }
        }

        // Zustand aus der Aktualitaet des zugehoerigen Transcripts ableiten.
        let jsonl = dir.join(format!("{id}.jsonl"));
        let state = freshness_state(&jsonl);

        out.push(Subagent {
            id,
            agent_type,
            state,
        });
    }
}

fn freshness_state(jsonl: &Path) -> SessionState {
    let Ok(meta) = std::fs::metadata(jsonl) else {
        // Kein Transcript lesbar -> Zustand unbekannt.
        return SessionState::Unknown;
    };
    let Ok(modified) = meta.modified() else {
        return SessionState::Unknown;
    };
    match SystemTime::now().duration_since(modified) {
        Ok(age) if age <= ACTIVE_WINDOW => SessionState::Working,
        Ok(_) => SessionState::Ready,
        // mtime in der Zukunft (Uhr-Drift) -> als frisch/aktiv werten.
        Err(_) => SessionState::Working,
    }
}
