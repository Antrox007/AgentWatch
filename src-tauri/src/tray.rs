//! Aktualisiert Tray-Tooltip und Status-Icon (Ampelfarbe/-glyph nach
//! Prioritaet: wartet > arbeitet > bereit > keine). Die Glyphen (Zahnrad /
//! Haekchen / Ausrufezeichen) sind dieselben wie in der Pill (`StatusIcon.tsx`)
//! — als PNG vorgerendert, damit Tray und Pill optisch einheitlich wirken statt
//! nur ein schlichter Farbpunkt im Tray.

use crate::model::{Snapshot, Totals};
use tauri::image::Image;
use tauri::AppHandle;

const WORKING_ICON: Image<'static> = tauri::include_image!("icons/tray-working.png");
const WAITING_ICON: Image<'static> = tauri::include_image!("icons/tray-waiting.png");
const READY_ICON: Image<'static> = tauri::include_image!("icons/tray-ready.png");

pub fn update_tray(app: &AppHandle, snapshot: &Snapshot) {
    let t = &snapshot.totals;
    let tooltip = format!(
        "AgentWatch — {} Sessions ({} aktiv, {} warten, {} fertig) · {} Agents",
        t.sessions, t.working, t.waiting, t.ready, t.agents
    );
    let icon = status_icon(t);

    // Tray-Operationen sicherheitshalber auf dem Main-Thread ausfuehren.
    let app_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app_handle.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(&tooltip));
            let _ = tray.set_icon(Some(icon));
        }
    });
}

/// Waehlt die zur dominanten Statusfarbe passende Glyphe (Pill-Pendant); ohne
/// aktive Sessions bleibt es beim schlichten grauen Punkt (kein Pill-Aequivalent).
fn status_icon(totals: &Totals) -> Image<'static> {
    if totals.waiting > 0 {
        WAITING_ICON
    } else if totals.working > 0 {
        WORKING_ICON
    } else if totals.ready > 0 {
        READY_ICON
    } else {
        build_none_icon()
    }
}

/// Zeichnet einen gefuellten, weich umrandeten grauen Kreis fuer "keine Sessions".
fn build_none_icon() -> Image<'static> {
    let size: u32 = 32;
    let (r, g, b) = (142u8, 142u8, 147u8);

    let mut buf = vec![0u8; (size * size * 4) as usize];
    let center = size as f32 / 2.0 - 0.5;
    let radius = size as f32 / 2.0 - 1.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;
            if dist <= radius {
                let alpha = if dist <= radius - 1.0 {
                    255.0
                } else {
                    (255.0 * (radius - dist)).clamp(0.0, 255.0)
                };
                buf[idx] = r;
                buf[idx + 1] = g;
                buf[idx + 2] = b;
                buf[idx + 3] = alpha as u8;
            }
        }
    }
    Image::new_owned(buf, size, size)
}
