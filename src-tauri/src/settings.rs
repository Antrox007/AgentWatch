//! App-Einstellungen: Laden/Speichern als JSON im App-Config-Verzeichnis + Default.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub notify_waiting: bool,
    pub notify_ready: bool,
    pub notify_context_pressure: bool,
    pub notify_new_subagent: bool,
    pub sound_enabled: bool,
    /// Geschaetzte USD-Kosten anzeigen (Subscription => nur informativ).
    pub show_cost: bool,
    /// Beim Windows-Login automatisch starten.
    pub autostart: bool,
    /// Opt-in: statusLine-Integration in ~/.claude/settings.json fuer Rate-Limits.
    pub statusline_integration: bool,
    /// "Dynamic Island"-Pill oben am Bildschirm anzeigen.
    #[serde(default = "default_true")]
    pub island_enabled: bool,
    /// Monitor-Name fuer die Pill (z.B. "\\\\.\\DISPLAY1"); None/leer = Primaermonitor.
    #[serde(default)]
    pub island_monitor: Option<String>,
    /// Beim Start automatisch auf eine neue Version pruefen (Updater-Plugin).
    #[serde(default = "default_true")]
    pub auto_update_check: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            // Toast-Benachrichtigungen standardmaessig AUS — die Island-Pill
            // signalisiert "wartet"/"fertig" visuell. Toggles bleiben verfuegbar.
            notify_waiting: false,
            notify_ready: false,
            notify_context_pressure: false,
            notify_new_subagent: false,
            sound_enabled: true,
            show_cost: false,
            autostart: false,
            statusline_integration: false,
            island_enabled: true,
            island_monitor: None,
            auto_update_check: true,
        }
    }
}

/// Im Tauri-State gehaltene, threadsichere Einstellungen.
pub struct SettingsState(pub Mutex<AppSettings>);

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> AppSettings {
    if let Some(path) = settings_path(app) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                return settings;
            }
        }
    }
    AppSettings::default()
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app).ok_or("Kein Config-Verzeichnis verfuegbar")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
