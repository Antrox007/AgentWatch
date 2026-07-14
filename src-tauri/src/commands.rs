//! Tauri-Commands, die das Frontend per `invoke` aufrufen kann.

use crate::aggregator;
use crate::model::Snapshot;
use crate::settings::{self, AppSettings, SettingsState};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

/// Ein Monitor zur Auswahl in den Einstellungen (fuer die Island-Position).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    /// Stabiler System-Name (wird als island_monitor gespeichert).
    pub name: String,
    /// Anzeige-Label fuer das Auswahlmenue.
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Liste der verfuegbaren Monitore (fuer das Auswahlmenue der Pill-Position).
#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    let Some(window) = app.get_webview_window("main") else {
        return vec![];
    };
    let primary_name = window
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().cloned());
    let Ok(monitors) = window.available_monitors() else {
        return vec![];
    };
    monitors
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            let name = m.name().cloned().unwrap_or_default();
            let size = m.size();
            let is_primary = primary_name.as_ref() == Some(&name) || (primary_name.is_none() && i == 0);
            let label = format!(
                "Monitor {} ({}×{}){}",
                i + 1,
                size.width,
                size.height,
                if is_primary { " — primär" } else { "" }
            );
            MonitorInfo {
                name,
                label,
                width: size.width,
                height: size.height,
                is_primary,
            }
        })
        .collect()
}

/// Vom Island-Frontend aufgerufen, sobald die gemessene Pill-Groesse bekannt ist
/// (Breite x Hoehe in CSS-Pixeln — Hoehe variiert, wenn die Pill aufklappt).
#[tauri::command]
pub fn position_island(app: AppHandle, state: State<SettingsState>, width: f64, height: f64) {
    let Some(window) = app.get_webview_window(crate::island::ISLAND_LABEL) else {
        return;
    };
    let monitor = state
        .inner()
        .0
        .lock()
        .ok()
        .and_then(|s| s.island_monitor.clone());
    crate::island::position(&window, monitor.as_deref(), width, height);
}

/// Dashboard-Flyout oben mittig unter der Pill auf-/zuklappen (Klick auf die Pill).
#[tauri::command]
pub fn toggle_dashboard_top(app: AppHandle) {
    crate::toggle_dashboard(&app, crate::Anchor::Top);
}

/// Dashboard-Flyout schliessen (Esc im Dashboard).
#[tauri::command]
pub fn hide_dashboard(app: AppHandle) {
    crate::hide_dashboard(&app);
}

/// Liefert den aktuellen Snapshot (fuer den initialen Load des Frontends).
#[tauri::command]
pub fn get_snapshot() -> Snapshot {
    aggregator::build_snapshot()
}

/// Aktuelle Einstellungen.
#[tauri::command]
pub fn get_settings(state: State<SettingsState>) -> AppSettings {
    state.inner().0.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Einstellungen speichern, persistieren und Autostart anwenden.
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<SettingsState>,
    mut settings: AppSettings,
) -> Result<(), String> {
    // statusLine-Integration nur bei Aenderung an-/abschalten (Opt-in, mit Backup).
    let prev_statusline = state
        .inner()
        .0
        .lock()
        .map(|s| s.statusline_integration)
        .unwrap_or(false);
    if settings.statusline_integration != prev_statusline {
        let result = if settings.statusline_integration {
            crate::statusline::enable()
        } else {
            crate::statusline::disable()
        };
        if let Err(e) = result {
            eprintln!("[agentwatch] statusLine-Integration fehlgeschlagen: {e}");
            // Persistierten Zustand an die Realitaet anpassen (Schalter zuruecksetzen).
            settings.statusline_integration = prev_statusline;
        }
    }

    apply_autostart(&app, settings.autostart);
    // Island-Fenster gemaess Einstellung herstellen/schliessen + positionieren.
    crate::island::ensure(&app, settings.island_enabled, settings.island_monitor.as_deref());

    settings::save(&app, &settings)?;
    if let Ok(mut guard) = state.inner().0.lock() {
        *guard = settings;
    }
    Ok(())
}

/// Autostart ueber das Plugin setzen (Fehler werden geloggt, nicht propagiert).
pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    // Nur aendern, wenn noetig (verhindert "Datei nicht gefunden" beim disable()).
    if manager.is_enabled().unwrap_or(false) == enabled {
        return;
    }
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(e) = result {
        eprintln!("[agentwatch] Autostart setzen fehlgeschlagen: {e}");
    }
}
