//! Echtzeit-Datei-Watcher (Rust `notify` => ReadDirectoryChangesW auf Windows).
//! Reagiert auf Aenderungen unter `sessions/` und `projects/`, berechnet einen
//! neuen Snapshot, reichert ihn mit der Zustands-Historie an, feuert
//! Benachrichtigungen, aktualisiert das Tray und sendet den Snapshot per
//! Tauri-Event ans Frontend. Ein 2s-Heartbeat-Tick faengt zusaetzlich
//! Subagent-mtime-Aenderungen ab.

use crate::history::HistoryStore;
use crate::settings::SettingsState;
use crate::{aggregator, claude_paths, notifications, tray};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[agentwatch] Watcher-Init fehlgeschlagen: {e}");
                return;
            }
        };

        if let Some(sessions_dir) = claude_paths::sessions_dir() {
            let _ = watcher.watch(&sessions_dir, RecursiveMode::Recursive);
        }
        if let Some(projects_dir) = claude_paths::projects_dir() {
            let _ = watcher.watch(&projects_dir, RecursiveMode::Recursive);
        }

        let mut tracker = notifications::Tracker::default();
        let mut history = HistoryStore::default();

        // Initialer Snapshot direkt beim Start.
        emit_snapshot(&app, &mut tracker, &mut history);

        loop {
            match rx.recv_timeout(Duration::from_millis(2000)) {
                Ok(_) => {
                    // Kurz sammeln, um Bursts zu buendeln (Debounce).
                    std::thread::sleep(Duration::from_millis(250));
                    while rx.try_recv().is_ok() {}
                    emit_snapshot(&app, &mut tracker, &mut history);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    emit_snapshot(&app, &mut tracker, &mut history);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn emit_snapshot(app: &AppHandle, tracker: &mut notifications::Tracker, history: &mut HistoryStore) {
    let mut snapshot = aggregator::build_snapshot();
    history.record_and_enrich(&mut snapshot);

    // Aktuelle Einstellungen aus dem State lesen.
    let settings = app
        .state::<SettingsState>()
        .inner()
        .0
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();

    notifications::process(app, tracker, &snapshot, &settings);
    tray::update_tray(app, &snapshot);
    let _ = app.emit("snapshot", &snapshot);
}
