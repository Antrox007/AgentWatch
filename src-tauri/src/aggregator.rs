//! Fuegt alle Datenquellen zu einem Snapshot zusammen:
//! Sessions (Live-Marker) -> Liveness-Check -> Transcript-Infos -> Subagents ->
//! Gruppierung nach Projekt + Kennzahlen.

use crate::model::*;
use crate::{claude_paths, processes, sessions, subagents, transcripts};
use std::collections::BTreeMap;

/// Aktivitaets-Fenster: ein `shell`-Status gilt nur als "arbeitet", wenn das
/// Transcript innerhalb dieser Zeitspanne zuletzt geschrieben wurde. Sonst
/// faerben zurueckgelassene Hintergrund-Shells (z.B. ein Dev-Server) die
/// Session dauerhaft als arbeitend. Bewusst grosszuegig, damit kurze
/// Denkpausen zwischen Tool-Aufrufen nicht faelschlich als "fertig" gelten.
const SHELL_ACTIVITY_WINDOW_MS: i64 = 30_000;

pub fn build_snapshot() -> Snapshot {
    let raw_sessions = sessions::read_sessions();
    let alive = processes::alive_claude_pids();
    let now = chrono::Utc::now().timestamp_millis();

    let mut session_infos: Vec<SessionInfo> = Vec::new();

    for raw in raw_sessions {
        // Nur Sessions mit lebendem claude-Prozess beruecksichtigen.
        if !alive.contains(&raw.pid) {
            continue;
        }

        let subs = subagents::scan_subagents(&raw.cwd, &raw.session_id);
        let active_subs = subs
            .iter()
            .filter(|s| s.state == SessionState::Working)
            .count();

        let transcript_path = transcripts::find_transcript(&raw.cwd, &raw.session_id);
        let (model, context_tokens, git_branch, last_text, estimated_cost_usd) =
            match &transcript_path {
                Some(path) => {
                    let ti = transcripts::read_transcript_info(path);
                    (
                        ti.model,
                        ti.context_tokens,
                        ti.git_branch,
                        ti.last_text,
                        ti.estimated_cost_usd,
                    )
                }
                None => (None, None, None, None, None),
            };

        // Aktivitaets-Signal aus der Transcript-mtime: unterscheidet echtes
        // Arbeiten von einem zurueckgelassenen Hintergrund-`shell`.
        let transcript_active = transcript_path
            .as_deref()
            .and_then(transcripts::file_mtime_millis)
            .map(|mt| now - mt <= SHELL_ACTIVITY_WINDOW_MS)
            .unwrap_or(false);
        let state = SessionState::from_status_with_activity(&raw.status, transcript_active);
        // Abgeleitetes Idle (aus stehendem shell, Marker != "idle") soll keine
        // "fertig"-Benachrichtigung ausloesen — sonst flattert ein langer
        // Vordergrund-Befehl wiederholt zwischen "arbeitet" und "bereit".
        let idle_inferred = state == SessionState::Ready && raw.status != "idle";

        let model_final = model.or_else(|| raw.model.clone());
        let context_window = model_final.as_deref().map(transcripts::context_window_for);
        let context_utilization = match (context_tokens, context_window) {
            (Some(t), Some(w)) if w > 0 => Some((t as f64 / w as f64).min(1.0)),
            _ => None,
        };

        session_infos.push(SessionInfo {
            project_name: claude_paths::project_name_from_cwd(&raw.cwd),
            session_id: raw.session_id,
            pid: raw.pid,
            cwd: raw.cwd,
            git_branch,
            state,
            model: model_final,
            updated_at: raw.updated_at,
            started_at: raw.started_at,
            context_tokens,
            context_window,
            context_utilization,
            estimated_cost_usd,
            idle_inferred,
            subagent_count: active_subs,
            subagents: subs,
            last_text,
            history: Vec::new(),
        });
    }

    // Nach Arbeitsverzeichnis gruppieren (stabile, alphabetische Reihenfolge).
    let mut groups: BTreeMap<String, ProjectGroup> = BTreeMap::new();
    for session in session_infos {
        let key = session.cwd.clone();
        let group = groups.entry(key).or_insert_with(|| ProjectGroup {
            project_name: session.project_name.clone(),
            path: session.cwd.clone(),
            sessions: Vec::new(),
        });
        group.sessions.push(session);
    }
    let projects: Vec<ProjectGroup> = groups.into_values().collect();

    // Kennzahlen.
    let mut totals = Totals::default();
    for group in &projects {
        for session in &group.sessions {
            totals.sessions += 1;
            match session.state {
                SessionState::Working => totals.working += 1,
                SessionState::Waiting => totals.waiting += 1,
                SessionState::Ready => totals.ready += 1,
                SessionState::Unknown => {}
            }
            totals.agents += session.subagent_count;
        }
    }

    Snapshot {
        generated_at: now,
        projects,
        totals,
        rate_limits: crate::statusline::read_rate_limits(),
    }
}
