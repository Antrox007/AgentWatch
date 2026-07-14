//! Aktualisiert Tray-Tooltip und ein dynamisch gezeichnetes Status-Icon
//! (Ampelfarbe nach Prioritaet: wartet > arbeitet > bereit > keine).

use crate::model::{Snapshot, Totals};
use tauri::image::Image;
use tauri::AppHandle;

pub fn update_tray(app: &AppHandle, snapshot: &Snapshot) {
    let t = &snapshot.totals;
    let tooltip = format!(
        "AgentWatch — {} Sessions ({} aktiv, {} warten, {} fertig) · {} Agents",
        t.sessions, t.working, t.waiting, t.ready, t.agents
    );
    let icon = build_status_icon(t);

    // Tray-Operationen sicherheitshalber auf dem Main-Thread ausfuehren.
    let app_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app_handle.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(&tooltip));
            let _ = tray.set_icon(Some(icon));
        }
    });
}

/// Zeichnet einen gefuellten, weich umrandeten Kreis in der dominanten Statusfarbe.
fn build_status_icon(totals: &Totals) -> Image<'static> {
    let size: u32 = 32;
    let (r, g, b) = if totals.waiting > 0 {
        (255u8, 149u8, 0u8) // orange: wartet auf Eingabe (hoechste Prioritaet)
    } else if totals.working > 0 {
        (10, 132, 255) // blau: arbeitet
    } else if totals.ready > 0 {
        (52, 199, 89) // gruen: bereit
    } else {
        (142, 142, 147) // grau: keine Sessions
    };

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
