//! Feuert Windows-Toast-Benachrichtigungen bei relevanten Zustandsuebergaengen,
//! gated durch die App-Einstellungen. Der Tracker merkt sich den vorherigen
//! Zustand pro Session, um nur echte Uebergaenge zu melden.

use crate::model::{SessionState, Snapshot};
use crate::settings::AppSettings;
use std::collections::{HashMap, HashSet};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Cooldown fuer die "Agent fertig"-Benachrichtigung je Session. Der Marker
/// `sessions/<PID>.json` flackert am Turn-Ende kurz (z.B. idle -> busy -> idle),
/// wodurch der Aggregator mehrere `Working -> Ready`-Uebergaenge innerhalb weniger
/// Sekunden sieht und sonst zwei Toasts feuern wuerde. Innerhalb dieses Fensters
/// wird nur der erste "fertig"-Toast gezeigt. Echte, getrennte Turn-Enden liegen
/// praktisch immer deutlich weiter auseinander.
const READY_NOTIFY_COOLDOWN_MS: i64 = 8_000;

#[derive(Default)]
pub struct Tracker {
    prev_state: HashMap<String, SessionState>,
    prev_pressure: HashMap<String, bool>,
    /// Bekannte Subagent-IDs je Session (stabil, im Gegensatz zum aktiven Zaehler).
    prev_agent_ids: HashMap<String, HashSet<String>>,
    /// Zeitpunkt (Epoch-ms, = snapshot.generated_at) der letzten "fertig"-
    /// Benachrichtigung je Session — fuer den Doppel-Toast-Cooldown.
    last_ready_notify: HashMap<String, i64>,
    initialized: bool,
}

pub fn process(app: &AppHandle, tracker: &mut Tracker, snapshot: &Snapshot, settings: &AppSettings) {
    let now = snapshot.generated_at;
    let mut current_state: HashMap<String, SessionState> = HashMap::new();

    for group in &snapshot.projects {
        for session in &group.sessions {
            let id = &session.session_id;
            current_state.insert(id.clone(), session.state);
            let current_ids: HashSet<String> =
                session.subagents.iter().map(|s| s.id.clone()).collect();

            if tracker.initialized {
                // Zustandswechsel. `prev` kopieren, damit der Borrow auf `tracker`
                // endet und wir `last_ready_notify` darunter mutieren koennen.
                if let Some(prev) = tracker.prev_state.get(id).copied() {
                    if prev != session.state {
                        match session.state {
                            SessionState::Waiting if settings.notify_waiting => notify(
                                app,
                                "Agent wartet auf Eingabe",
                                &format!("{} wartet auf dich", session.project_name),
                                settings.sound_enabled,
                            ),
                            SessionState::Ready if settings.notify_ready => {
                                // idle_inferred (stehendes shell) und der Cooldown
                                // (Marker-Flacker am Turn-Ende) unterdruecken den Toast.
                                let due = ready_notify_due(
                                    session.idle_inferred,
                                    tracker.last_ready_notify.get(id).copied(),
                                    now,
                                );
                                if due {
                                    notify(
                                        app,
                                        "Agent fertig",
                                        &format!("{}: Turn abgeschlossen", session.project_name),
                                        settings.sound_enabled,
                                    );
                                    tracker.last_ready_notify.insert(id.clone(), now);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Neuer Subagent: eine ID, die wir fuer diese Session noch nie gesehen haben.
                if settings.notify_new_subagent {
                    let empty = HashSet::new();
                    let prev_ids = tracker.prev_agent_ids.get(id).unwrap_or(&empty);
                    let new_count = current_ids.iter().filter(|x| !prev_ids.contains(*x)).count();
                    if new_count > 0 {
                        notify(
                            app,
                            "Neuer Subagent gestartet",
                            &format!("{}: {} neue(r) Agent(s)", session.project_name, new_count),
                            settings.sound_enabled,
                        );
                    }
                }
            }

            // Context-Pressure ab 80%.
            let pressure = session.context_utilization.map_or(false, |u| u >= 0.8);
            if tracker.initialized && settings.notify_context_pressure {
                let was = tracker.prev_pressure.get(id).copied().unwrap_or(false);
                if pressure && !was {
                    notify(
                        app,
                        "Context-Pressure",
                        &format!("{}: Kontext über 80% — bald frische Session", session.project_name),
                        settings.sound_enabled,
                    );
                }
            }
            tracker.prev_pressure.insert(id.clone(), pressure);
            tracker.prev_agent_ids.insert(id.clone(), current_ids);
        }
    }

    // Zustands-Merker auf die aktuell vorhandenen Sessions reduzieren, damit die
    // Maps ueber lange Laufzeiten nicht unbegrenzt wachsen (verschwundene
    // Sessions vergessen — Session-IDs sind eindeutig, keine Wiederverwendung).
    tracker.last_ready_notify.retain(|k, _| current_state.contains_key(k));
    tracker.prev_pressure.retain(|k, _| current_state.contains_key(k));
    tracker.prev_agent_ids.retain(|k, _| current_state.contains_key(k));

    tracker.prev_state = current_state;
    tracker.initialized = true;
}

/// Entscheidet, ob bei einem (bereits erkannten) Uebergang nach `Ready` eine
/// "fertig"-Benachrichtigung faellig ist. Unterdrueckt abgeleitete Ready-Zustaende
/// (`idle_inferred` = stehendes shell, kein echtes Turn-Ende) sowie Doppel-Toasts
/// innerhalb des Cooldowns (Marker-Flacker idle<->busy am Turn-Ende).
/// `last_notify` = Zeitpunkt der letzten "fertig"-Benachrichtigung dieser Session.
fn ready_notify_due(idle_inferred: bool, last_notify: Option<i64>, now: i64) -> bool {
    if idle_inferred {
        return false;
    }
    match last_notify {
        Some(t) => now.saturating_sub(t) >= READY_NOTIFY_COOLDOWN_MS,
        None => true,
    }
}

fn notify(app: &AppHandle, title: &str, body: &str, sound: bool) {
    let mut builder = app.notification().builder().title(title).body(body);
    if sound {
        builder = builder.sound("Default");
    }
    let _ = builder.show();
}

#[cfg(test)]
mod tests {
    use super::{ready_notify_due, READY_NOTIFY_COOLDOWN_MS};

    #[test]
    fn erstes_turn_ende_feuert() {
        // Noch nie benachrichtigt -> faellig.
        assert!(ready_notify_due(false, None, 10_000));
    }

    #[test]
    fn abgeleitetes_ready_feuert_nicht() {
        // idle_inferred (stehendes shell) -> nie "fertig".
        assert!(!ready_notify_due(true, None, 10_000));
    }

    #[test]
    fn doppel_toast_innerhalb_cooldown_unterdrueckt() {
        // Zweiter Ready-Uebergang 2s nach dem ersten (Marker-Flacker) -> kein Toast.
        let last = 10_000;
        assert!(!ready_notify_due(false, Some(last), last + 2_000));
    }

    #[test]
    fn echtes_neues_turn_ende_nach_cooldown_feuert() {
        // Weit nach dem Cooldown (z.B. naechster echter Turn) -> wieder faellig.
        let last = 10_000;
        assert!(ready_notify_due(false, Some(last), last + READY_NOTIFY_COOLDOWN_MS));
        assert!(ready_notify_due(false, Some(last), last + 60_000));
    }

    #[test]
    fn rueckwaerts_springende_uhr_unterdrueckt_nicht_dauerhaft() {
        // Falls die Systemuhr zurueckspringt (NTP), keine negative Differenz ->
        // saturating_sub liefert 0 -> innerhalb Cooldown (kein Doppel-Toast).
        let last = 50_000;
        assert!(!ready_notify_due(false, Some(last), 49_000));
    }
}
