//! "Dynamic Island"-Pill: ein zweites, randloses, transparentes Always-on-Top-
//! Fenster oben mittig auf einem waehlbaren Monitor. Es laedt dasselbe Frontend
//! (Query `?island`) und empfaengt denselben `snapshot`-Event wie das Hauptfenster.
//! Groesse/Position werden in Rust gesetzt, damit das Frontend nur `invoke`/`listen`
//! braucht (keine zusaetzlichen Fenster-Permissions).

use std::thread;
use std::time::Duration;
use tauri::{
    AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const ISLAND_LABEL: &str = "island";
/// Logische Hoehe der Pill — innerhalb der ueblichen Fenster-Titelleisten-Hoehe.
pub(crate) const ISLAND_HEIGHT: f64 = 34.0;
/// Startbreite, bis das Frontend die echte Inhaltsbreite meldet.
const ISLAND_DEFAULT_WIDTH: f64 = 220.0;
/// Logischer Abstand der Pill zur Bildschirm-Oberkante.
pub(crate) const ISLAND_TOP_MARGIN: f64 = 6.0;
/// Transparenter "Bleed"-Rand rund um die sichtbare Pill. Das Pill-Fenster ist
/// exakt auf den Inhalt zugeschnitten; ohne diesen Rand wuerde alles, was ueber
/// die Pill-Kante hinausgeht (aeusserer Glow, Hover-Lift, Aufklapp-Overshoot),
/// am Fensterrand abgeschnitten. Das Frontend rendert die Pill mit genau diesem
/// Innen-Padding (CSS `--bleed`), misst die Gesamtgroesse (inkl. Rand) und meldet
/// sie an `position`. MUSS mit dem CSS-Wert `--bleed` in `App.css` uebereinstimmen.
pub(crate) const ISLAND_BLEED: f64 = 20.0;
/// Poll-Intervall des Cursor-Durchklick-Threads (siehe `start_cursor_passthrough`).
const CURSOR_POLL_MS: u64 = 60;

/// Stellt das Island-Fenster gemaess Einstellung her: erzeugt/zeigt es bei
/// `enabled`, schliesst es sonst. `monitor` ist der Monitor-Name (oder None =
/// Primaermonitor).
pub fn ensure(app: &AppHandle, enabled: bool, monitor: Option<&str>) {
    if !enabled {
        if let Some(window) = app.get_webview_window(ISLAND_LABEL) {
            let _ = window.close();
        }
        return;
    }

    let window = match app.get_webview_window(ISLAND_LABEL) {
        Some(window) => window,
        None => match build(app) {
            Ok(window) => window,
            Err(e) => {
                eprintln!("[agentwatch] Island-Fenster konnte nicht erstellt werden: {e}");
                return;
            }
        },
    };

    let _ = window.show();
    position(&window, monitor, ISLAND_DEFAULT_WIDTH, ISLAND_HEIGHT);
}

/// Baut das Island-Fenster (zunaechst unsichtbar; `ensure` zeigt es an).
fn build(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let window =
        WebviewWindowBuilder::new(app, ISLAND_LABEL, WebviewUrl::App("index.html?island".into()))
            .title("AgentWatch Island")
            .inner_size(ISLAND_DEFAULT_WIDTH, ISLAND_HEIGHT)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .build()?;
    // Start: das ganze Fenster ist klick-durchlaessig. Der Cursor-Thread
    // (`start_cursor_passthrough`) schaltet das Abfangen nur ein, solange der
    // Zeiger ueber der sichtbaren Pill liegt — der transparente Bleed-Rand und
    // der Bereich ueber/neben der Pill bleiben so durchklickbar.
    let _ = window.set_ignore_cursor_events(true);
    Ok(window)
}

/// Startet einen Hintergrund-Thread, der das Pill-Fenster nur dort Klicks abfangen
/// laesst, wo die sichtbare Pill liegt. Ueberall sonst im Fensterrechteck (der
/// transparente Bleed-Rand, der Bereich ueber/neben der gerundeten Pill) wird per
/// `ignore_cursor_events` durchlaessig, damit darunterliegende Fenster — etwa
/// Titelleisten am oberen Bildschirmrand — wieder greif- und klickbar sind.
///
/// Warum Polling statt Webview-Mausevents: sobald `ignore_cursor_events(true)` aktiv
/// ist, bekommt das Webview keine Mausnachrichten mehr (auf Windows WS_EX_TRANSPARENT)
/// und koennte das Wieder-Betreten der Pill nicht erkennen. Wir fragen daher die
/// globale Cursor-Position direkt beim OS ab und vergleichen sie mit dem Pill-Rechteck.
/// Der Thread laeuft fuer die App-Lebensdauer und ruht, wenn die Pill aus ist.
pub fn start_cursor_passthrough(app: AppHandle) {
    thread::spawn(move || {
        // Zuletzt gesetzter Zustand, damit nur bei Aenderung an den Main-Thread
        // dispatcht wird (None = unbekannt, z. B. nach Aus/Ein der Pill).
        let mut ignoring: Option<bool> = None;
        loop {
            thread::sleep(Duration::from_millis(CURSOR_POLL_MS));
            let Some(window) = app.get_webview_window(ISLAND_LABEL) else {
                ignoring = None;
                continue;
            };
            let over = cursor_over_pill(&app, &window).unwrap_or(false);
            if ignoring != Some(!over) {
                let _ = window.set_ignore_cursor_events(!over);
                ignoring = Some(!over);
            }
        }
    });
}

/// True, wenn der globale Mauszeiger ueber der sichtbaren Pill liegt — das
/// Fensterrechteck abzueglich des transparenten Bleed-Rands auf allen vier Seiten.
fn cursor_over_pill(app: &AppHandle, window: &WebviewWindow) -> Option<bool> {
    if !window.is_visible().unwrap_or(false) {
        return Some(false);
    }
    let cursor = app.cursor_position().ok()?;
    let pos = window.outer_position().ok()?;
    let size = window.inner_size().ok()?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let bleed = (ISLAND_BLEED * scale).round() as i32;
    let left = pos.x + bleed;
    let top = pos.y + bleed;
    let right = pos.x + size.width as i32 - bleed;
    let bottom = pos.y + size.height as i32 - bleed;
    let cx = cursor.x.round() as i32;
    let cy = cursor.y.round() as i32;
    Some(cx >= left && cx < right && cy >= top && cy < bottom)
}

/// Loest den Zielmonitor auf: per Name, sonst Primaermonitor, sonst der erste.
pub(crate) fn resolve_monitor(window: &WebviewWindow, name: Option<&str>) -> Option<Monitor> {
    if let Some(name) = name.filter(|n| !n.is_empty()) {
        if let Ok(monitors) = window.available_monitors() {
            if let Some(found) = monitors
                .into_iter()
                .find(|m| m.name().map(|n| n.as_str() == name).unwrap_or(false))
            {
                return Some(found);
            }
        }
    }
    window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.available_monitors().ok().and_then(|v| v.into_iter().next()))
}

/// Setzt die Fenstergroesse (logische Breite x Hoehe) und zentriert das Fenster
/// oben am Zielmonitor. `content_width_logical`/`content_height_logical` kommen
/// vom Frontend (die gemessene Pill-Groesse in CSS-Pixeln). Die Hoehe ist
/// dynamisch, damit die Pill bei Ereignissen aufklappen kann.
pub fn position(
    window: &WebviewWindow,
    monitor: Option<&str>,
    content_width_logical: f64,
    content_height_logical: f64,
) {
    let width = content_width_logical.max(80.0);
    let height = content_height_logical.max(ISLAND_HEIGHT);
    let _ = window.set_size(LogicalSize::new(width, height));

    let Some(monitor) = resolve_monitor(window, monitor) else {
        return;
    };
    let scale = monitor.scale_factor();
    let mpos = monitor.position();
    let msize = monitor.size();
    let win_w_phys = (width * scale).round() as i32;
    let x = mpos.x + ((msize.width as i32 - win_w_phys) / 2).max(0);
    // Das Fenster ist um den Bleed-Rand groesser als die sichtbare Pill. Damit die
    // Pill selbst (nicht der transparente Rand) bei ISLAND_TOP_MARGIN sitzt, schieben
    // wir das Fenster um den Bleed nach oben — geklemmt auf die Monitor-Oberkante,
    // falls der obere Rand sonst aus dem Bild liefe (dann wird er einfach abgeschnitten).
    let y = mpos.y + (((ISLAND_TOP_MARGIN - ISLAND_BLEED) * scale).round() as i32).max(0);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
