//! Opt-in: Schreibt eine `statusLine` in ~/.claude/settings.json, deren Command
//! den Statusline-Payload (inkl. `rate_limits`) in eine Datei schreibt, die
//! AgentWatch liest.
//!
//! WICHTIG (Datensicherheit): Es wird IMMER nur der `statusLine`-Key chirurgisch
//! veraendert — niemals die ganze settings.json ueberschrieben. Beim Aktivieren
//! wird der urspruengliche `statusLine`-Wert in einer Marker-Datei gesichert und
//! beim Deaktivieren exakt dieser Key wiederhergestellt; alle anderen (auch
//! zwischenzeitlich vom User geaenderten) Keys bleiben unangetastet.

use crate::claude_paths;
use crate::model::RateLimits;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Gemerkter Spitzenwert je Limit-Fenster: (resets_at-Sekunden, max used_percentage).
/// Hintergrund: Alle Claude-Code-Sessions schreiben dieselbe Statusline-Datei mit
/// ihren ZULETZT bekannten Rate-Limits. Eine inaktive Session kann die Datei mit
/// veralteten, niedrigeren Werten ueberschreiben. Da der Verbrauch innerhalb eines
/// Fensters bis zum Reset nur steigt, merken wir uns das Maximum pro Fenster und
/// zeigen dieses — ein Altwert zieht die Anzeige so nicht mehr nach unten.
static FIVE_HOUR_PEAK: OnceLock<Mutex<Option<(i64, f64)>>> = OnceLock::new();
static SEVEN_DAY_PEAK: OnceLock<Mutex<Option<(i64, f64)>>> = OnceLock::new();

fn payload_file() -> Option<PathBuf> {
    claude_paths::claude_dir().map(|d| d.join("agentwatch-statusline.json"))
}
fn helper_script() -> Option<PathBuf> {
    claude_paths::claude_dir().map(|d| d.join("agentwatch-statusline.ps1"))
}
fn settings_file() -> Option<PathBuf> {
    claude_paths::claude_dir().map(|d| d.join("settings.json"))
}
/// Sichert den urspruenglichen statusLine-Wert (oder dessen Abwesenheit).
fn prev_marker_file() -> Option<PathBuf> {
    claude_paths::claude_dir().map(|d| d.join("agentwatch-statusline-prev.json"))
}

/// Reine Maximum-Logik (testbar): nimmt den bisher gemerkten Fensterstand, die
/// aktuelle Reset-Zeit und den aktuellen Prozentwert; liefert (neuer Stand,
/// anzuzeigender Prozentwert). Gleiche Reset-Zeit => Maximum behalten; neue
/// Reset-Zeit => Fenster zuruecksetzen; ohne Reset-Zeit nicht merken.
fn peak_step(
    prev: Option<(i64, f64)>,
    resets_at: Option<i64>,
    pct: f64,
) -> (Option<(i64, f64)>, f64) {
    match (prev, resets_at) {
        (Some((prev_reset, prev_pct)), Some(reset)) if prev_reset == reset => {
            let max = prev_pct.max(pct);
            (Some((reset, max)), max)
        }
        (_, Some(reset)) => (Some((reset, pct)), pct),
        (_, None) => (prev, pct),
    }
}

/// Wendet `peak_step` auf den prozessweiten Fenster-Merker an.
fn apply_peak(cell: &OnceLock<Mutex<Option<(i64, f64)>>>, resets_at: Option<i64>, pct: f64) -> f64 {
    let mutex = cell.get_or_init(|| Mutex::new(None));
    let Ok(mut guard) = mutex.lock() else {
        return pct;
    };
    let (next, shown) = peak_step(*guard, resets_at, pct);
    *guard = next;
    shown
}

/// Liest die Rate-Limits aus der vom statusLine-Command geschriebenen Datei.
pub fn read_rate_limits() -> Option<RateLimits> {
    let path = payload_file()?;
    let content = std::fs::read_to_string(&path).ok()?;
    // PowerShell 5.1 kann ein UTF-8-BOM voranstellen — entfernen.
    let content = content.trim_start_matches('\u{feff}');
    let value: Value = serde_json::from_str(content).ok()?;
    let rl = value.get("rate_limits")?;

    let pct = |key: &str| {
        rl.get(key)
            .and_then(|o| o.get("used_percentage"))
            .and_then(Value::as_f64)
    };
    // resets_at: aktuelle Claude-Versionen (>=2.1) schreiben Unix-Sekunden als
    // ZAHL; aeltere ggf. als String. Beides unterstuetzen (sonst wird die
    // Reset-Zeit still verworfen und die Fenster-Zuordnung greift nicht).
    let reset_secs = |key: &str| -> Option<i64> {
        let v = rl.get(key)?.get("resets_at")?;
        v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    };
    let reset_str = |key: &str| -> Option<String> {
        let v = rl.get(key)?.get("resets_at")?;
        match v {
            Value::Number(n) => Some(n.to_string()),
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    };

    // Pro Fenster den Maximalwert behalten (gegen Altwert-Ueberschreibungen).
    let five = pct("five_hour").map(|p| apply_peak(&FIVE_HOUR_PEAK, reset_secs("five_hour"), p));
    let seven = pct("seven_day").map(|p| apply_peak(&SEVEN_DAY_PEAK, reset_secs("seven_day"), p));

    let limits = RateLimits {
        five_hour_pct: five,
        seven_day_pct: seven,
        five_hour_resets_at: reset_str("five_hour"),
        seven_day_resets_at: reset_str("seven_day"),
    };
    if limits.five_hour_pct.is_some() || limits.seven_day_pct.is_some() {
        Some(limits)
    } else {
        None
    }
}

/// Laedt die settings.json als JSON-Objekt. Fehler, wenn vorhanden aber
/// nicht parsebar bzw. kein Objekt — dann wird NICHTS angetastet.
fn load_settings_object(path: &PathBuf) -> Result<serde_json::Map<String, Value>, String> {
    if !path.exists() {
        return Ok(serde_json::Map::new());
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let trimmed = content.trim_start_matches('\u{feff}');
    if trimmed.trim().is_empty() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|_| "settings.json ist nicht parsebar — Integration abgebrochen".to_string())?;
    match value {
        Value::Object(map) => Ok(map),
        _ => Err("settings.json ist kein JSON-Objekt — Integration abgebrochen".to_string()),
    }
}

fn write_settings_object(path: &PathBuf, map: &serde_json::Map<String, Value>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&Value::Object(map.clone())).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Aktiviert die Integration: sichert nur den urspruenglichen statusLine-Wert,
/// setzt unseren Command, schreibt das Helper-Script.
pub fn enable() -> Result<(), String> {
    let settings = settings_file().ok_or("kein settings.json-Pfad")?;
    let marker = prev_marker_file().ok_or("kein Marker-Pfad")?;
    let script = helper_script().ok_or("kein Script-Pfad")?;
    let payload = payload_file().ok_or("kein Payload-Pfad")?;

    let mut map = load_settings_object(&settings)?;

    // Originalen statusLine-Wert sichern (had + value), nur falls noch kein Marker.
    if !marker.exists() {
        let prev = match map.get("statusLine") {
            Some(v) => serde_json::json!({ "had": true, "value": v }),
            None => serde_json::json!({ "had": false, "value": Value::Null }),
        };
        if let Ok(s) = serde_json::to_string_pretty(&prev) {
            let _ = std::fs::write(&marker, s);
        }
    }

    // Helper-Script BOM-frei schreiben (sonst zerbricht das serde-Parsing).
    let payload_str = payload.to_string_lossy().replace('\\', "\\\\");
    let ps = format!(
        "$in = [Console]::In.ReadToEnd()\r\n[System.IO.File]::WriteAllText(\"{}\", $in, (New-Object System.Text.UTF8Encoding($false)))\r\nWrite-Output \"\"\r\n",
        payload_str
    );
    std::fs::write(&script, ps).map_err(|e| e.to_string())?;

    let command = format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
        script.to_string_lossy()
    );
    map.insert(
        "statusLine".into(),
        serde_json::json!({ "type": "command", "command": command }),
    );
    write_settings_object(&settings, &map)
}

/// Deaktiviert die Integration: stellt den urspruenglichen statusLine-Key wieder
/// her (oder entfernt ihn) — chirurgisch, ohne andere Keys anzutasten.
pub fn disable() -> Result<(), String> {
    let settings = settings_file().ok_or("kein settings.json-Pfad")?;
    let marker = prev_marker_file().ok_or("kein Marker-Pfad")?;

    // Aktuelle settings laden (Parse-Fehler => nicht anfassen, nur aufraeumen).
    if let Ok(mut map) = load_settings_object(&settings) {
        // Original aus Marker bestimmen.
        let mut restored = false;
        if let Ok(content) = std::fs::read_to_string(&marker) {
            if let Ok(prev) = serde_json::from_str::<Value>(&content) {
                if prev.get("had").and_then(Value::as_bool) == Some(true) {
                    if let Some(orig) = prev.get("value") {
                        map.insert("statusLine".into(), orig.clone());
                        restored = true;
                    }
                }
            }
        }
        if !restored {
            map.remove("statusLine");
        }
        let _ = write_settings_object(&settings, &map);
    }

    // Aufraeumen.
    let _ = std::fs::remove_file(&marker);
    if let Some(p) = payload_file() {
        let _ = std::fs::remove_file(p);
    }
    if let Some(s) = helper_script() {
        let _ = std::fs::remove_file(s);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::peak_step;

    #[test]
    fn erster_wert_wird_uebernommen() {
        assert_eq!(peak_step(None, Some(100), 17.0), (Some((100, 17.0)), 17.0));
    }

    #[test]
    fn hoeherer_wert_im_gleichen_fenster_steigt() {
        assert_eq!(
            peak_step(Some((100, 17.0)), Some(100), 22.0),
            (Some((100, 22.0)), 22.0)
        );
    }

    #[test]
    fn veralteter_niedrigerer_wert_zieht_nicht_runter() {
        // Kernfall: inaktive Session schreibt 17%, Maximum bleibt 22%.
        assert_eq!(
            peak_step(Some((100, 22.0)), Some(100), 17.0),
            (Some((100, 22.0)), 22.0)
        );
    }

    #[test]
    fn neues_fenster_setzt_zurueck() {
        // resets_at wechselt (Fenster-Reset) -> neuer, niedrigerer Wert gilt.
        assert_eq!(
            peak_step(Some((100, 22.0)), Some(200), 3.0),
            (Some((200, 3.0)), 3.0)
        );
    }

    #[test]
    fn ohne_resets_at_wird_nicht_gemerkt() {
        // Ohne Fenster-Schluessel: aktuellen Wert zeigen, Stand unveraendert.
        assert_eq!(
            peak_step(Some((100, 22.0)), None, 5.0),
            (Some((100, 22.0)), 5.0)
        );
    }
}
