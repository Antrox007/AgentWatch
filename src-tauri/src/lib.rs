//! AgentWatch — Live-Monitor fuer aktive Claude-Code-Sessions.
//! Verdrahtet Plugins, Tray-Icon, Einstellungen und den Datei-Watcher.

mod aggregator;
mod claude_paths;
mod commands;
mod history;
mod island;
mod model;
mod notifications;
mod pricing;
mod processes;
mod sessions;
mod settings;
mod statusline;
mod subagents;
mod transcripts;
mod tray;
mod watcher;

use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

/// Verhindert die Flyout-Race: Ein Klick auf Pill/Tray laesst zuerst das
/// Dashboard den Fokus verlieren (-> Auto-Hide), bevor der Klick-Handler laeuft.
/// Ohne Schutz wuerde derselbe Klick das gerade geschlossene Fenster sofort
/// wieder oeffnen. Wir merken uns den Zeitpunkt des Auto-Hide und ignorieren
/// einen Toggle, der innerhalb dieses Fensters eintrifft.
const AUTO_HIDE_GUARD_MS: u64 = 300;
/// Logischer Abstand des Dashboards unter der Pill (zusaetzlich zur Pill-Hoehe).
const DASHBOARD_TOP_GAP: f64 = 8.0;
/// Logische Marge zur rechten/unteren Bildschirmkante beim Tray-Anker ohne Cursor.
const DASHBOARD_BR_MARGIN_X: f64 = 12.0;
/// Logische Marge nach oben (haelt die Windows-Taskleiste frei).
const DASHBOARD_BR_MARGIN_Y: f64 = 56.0;

/// Merker fuer den Zeitpunkt des letzten Auto-Hide (Hide bei Fokusverlust).
pub(crate) struct DashboardGuard(pub Mutex<Option<Instant>>);

/// Wo das Dashboard-Flyout verankert wird.
#[derive(Clone, Copy)]
pub(crate) enum Anchor {
    /// Oben mittig direkt unter der Pill (Pill-Klick).
    Top,
    /// Unten rechts beim Mauszeiger (Tray-Linksklick).
    BottomRightAt(PhysicalPosition<f64>),
    /// Unten rechts der Monitor-Flaeche (Tray-Menue, kein Cursor verfuegbar).
    BottomRightMonitor,
}

/// Oeffnet/schliesst das Dashboard-Flyout am gewuenschten Anker. Schliesst bei
/// erneutem Aufruf (Toggle) und konsumiert einen Klick, der das Fenster soeben
/// per Fokusverlust geschlossen hat.
pub(crate) fn toggle_dashboard(app: &AppHandle, anchor: Anchor) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    // Wurde das Fenster gerade (< Guard-Fenster) per Blur geschlossen, ist dieser
    // Klick das Schliessen — nicht erneut oeffnen.
    if let Some(state) = app.try_state::<DashboardGuard>() {
        if let Ok(mut guard) = state.inner().0.lock() {
            if let Some(t) = *guard {
                if t.elapsed() < Duration::from_millis(AUTO_HIDE_GUARD_MS) {
                    *guard = None;
                    return;
                }
            }
        }
    }

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }

    position_dashboard(&window, app, anchor);
    let _ = window.show();
    let _ = window.set_focus();
}

/// Versteckt das Dashboard-Flyout (Esc / direktes Schliessen).
pub(crate) fn hide_dashboard(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Positioniert das `main`-Fenster gemaess Anker.
fn position_dashboard(window: &WebviewWindow, app: &AppHandle, anchor: Anchor) {
    match anchor {
        Anchor::Top => position_top_center(window, app),
        Anchor::BottomRightAt(click) => position_popup(window, click),
        Anchor::BottomRightMonitor => position_bottom_right_monitor(window),
    }
}

/// Liest den in den Einstellungen gewaehlten Pill-Monitor (None = Primaer).
fn island_monitor_name(app: &AppHandle) -> Option<String> {
    app.try_state::<settings::SettingsState>()
        .and_then(|s| s.inner().0.lock().ok().and_then(|g| g.island_monitor.clone()))
}

/// Oben mittig direkt unter der Pill — auf demselben Monitor wie die Pill.
fn position_top_center(window: &WebviewWindow, app: &AppHandle) {
    let monitor_name = island_monitor_name(app);
    let Some(monitor) = island::resolve_monitor(window, monitor_name.as_deref()) else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let scale = monitor.scale_factor();
    let mpos = monitor.position();
    let msize = monitor.size();
    let x = top_center_x(mpos.x, msize.width as i32, size.width as i32);
    let gap = ((island::ISLAND_TOP_MARGIN + island::ISLAND_HEIGHT + DASHBOARD_TOP_GAP) * scale)
        .round() as i32;
    let y = mpos.y + gap;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Unten rechts der Monitor-Flaeche (Primaermonitor), Taskleiste freihaltend.
fn position_bottom_right_monitor(window: &WebviewWindow) {
    let Some(monitor) = island::resolve_monitor(window, None) else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let scale = monitor.scale_factor();
    let mpos = monitor.position();
    let msize = monitor.size();
    let margin_x = (DASHBOARD_BR_MARGIN_X * scale).round() as i32;
    let margin_y = (DASHBOARD_BR_MARGIN_Y * scale).round() as i32;
    let (x, y) = bottom_right_origin(
        mpos.x,
        mpos.y,
        msize.width as i32,
        msize.height as i32,
        size.width as i32,
        size.height as i32,
        margin_x,
        margin_y,
    );
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// Reine X-Berechnung fuer den oben-zentrierten Anker (testbar).
fn top_center_x(monitor_x: i32, monitor_w: i32, win_w: i32) -> i32 {
    monitor_x + ((monitor_w - win_w) / 2).max(0)
}

/// Reine Ursprungs-Berechnung fuer den Bottom-Right-Anker (testbar). Klemmt so,
/// dass das Fenster nicht ueber die obere/linke Monitorkante hinausragt.
fn bottom_right_origin(
    monitor_x: i32,
    monitor_y: i32,
    monitor_w: i32,
    monitor_h: i32,
    win_w: i32,
    win_h: i32,
    margin_x: i32,
    margin_y: i32,
) -> (i32, i32) {
    let x = (monitor_x + monitor_w - win_w - margin_x).max(monitor_x);
    let y = (monitor_y + monitor_h - win_h - margin_y).max(monitor_y);
    (x, y)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::get_settings,
            commands::save_settings,
            commands::list_monitors,
            commands::position_island,
            commands::toggle_dashboard_top,
            commands::hide_dashboard
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Einstellungen laden, in den State legen und Autostart anwenden.
            let loaded = settings::load(&handle);
            commands::apply_autostart(&handle, loaded.autostart);
            // WICHTIG: SettingsState registrieren, BEVOR ein Fenster erzeugt wird.
            // Das Erstellen des Island-Fensters kann die Event-Loop pumpen, wodurch
            // das Hauptfenster sonst `get_settings` aufruft, bevor der State da ist
            // -> Command-Fehler -> Frontend faellt in den Demo-Modus.
            let island_enabled = loaded.island_enabled;
            let island_monitor = loaded.island_monitor.clone();
            app.manage(settings::SettingsState(Mutex::new(loaded)));
            // Merker fuer das Auto-Hide des Dashboards — ebenfalls VOR der ersten
            // Fenster-/Event-Loop-pumpenden Operation registrieren.
            app.manage(DashboardGuard(Mutex::new(None)));
            // Island-Pill gemaess Einstellung herstellen.
            island::ensure(&handle, island_enabled, island_monitor.as_deref());
            // Cursor-Durchklick der Pill: faengt Klicks nur ueber der sichtbaren Pill
            // ab, laesst den transparenten Rand (und alles ueber/neben der Pill)
            // durch -> Hintergrundfenster bleiben dort greif- und klickbar.
            island::start_cursor_passthrough(handle.clone());

            // Dashboard-Flyout bei Fokusverlust automatisch schliessen (Klick
            // daneben / anderes Fenster). Zeitpunkt merken, damit der ausloesende
            // Klick das Fenster nicht sofort wieder oeffnet (siehe toggle_dashboard).
            if let Some(main) = handle.get_webview_window("main") {
                let app_for_blur = handle.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        if let Some(w) = app_for_blur.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                                if let Some(state) = app_for_blur.try_state::<DashboardGuard>() {
                                    if let Ok(mut guard) = state.inner().0.lock() {
                                        *guard = Some(Instant::now());
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // Tray-Kontextmenue.
            let show = MenuItem::with_id(&handle, "show", "Fenster anzeigen", true, None::<&str>)?;
            let quit = MenuItem::with_id(&handle, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(&handle, &[&show, &quit])?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(handle.default_window_icon().unwrap().clone())
                .tooltip("AgentWatch")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    // Kein Cursor verfuegbar -> unten rechts der Monitor-Flaeche.
                    "show" => toggle_dashboard(app, Anchor::BottomRightMonitor),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // Linksklick auf das Icon klappt das Dashboard unten rechts
                    // beim Mauszeiger auf bzw. wieder zu.
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        toggle_dashboard(tray.app_handle(), Anchor::BottomRightAt(position));
                    }
                })
                .build(&handle)?;

            // Echtzeit-Watcher starten.
            watcher::start(handle.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Positioniert das Popup-Fenster bei einem Tray-Klick nahe dem Mauszeiger
/// (Fenster-Unterkante leicht ueber dem Klick, rechte Kante am Klick).
fn position_popup(window: &tauri::WebviewWindow, click: tauri::PhysicalPosition<f64>) {
    if let Ok(size) = window.outer_size() {
        let x = (click.x - size.width as f64 + 24.0).max(0.0) as i32;
        let y = (click.y - size.height as f64 - 12.0).max(0.0) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::{bottom_right_origin, top_center_x};

    #[test]
    fn top_center_x_zentriert_auf_primaermonitor() {
        // 1920 breit, 440 Fenster -> (1920-440)/2 = 740, Offset 0.
        assert_eq!(top_center_x(0, 1920, 440), 740);
    }

    #[test]
    fn top_center_x_beruecksichtigt_monitor_offset() {
        // Zweiter Monitor beginnt bei x=1920.
        assert_eq!(top_center_x(1920, 2560, 440), 1920 + (2560 - 440) / 2);
    }

    #[test]
    fn top_center_x_klemmt_bei_zu_breitem_fenster() {
        // Fenster breiter als Monitor -> nie links der Monitorkante.
        assert_eq!(top_center_x(100, 400, 800), 100);
    }

    #[test]
    fn bottom_right_origin_setzt_unten_rechts_mit_marge() {
        // 1920x1080, 440x700 Fenster, Marge 12/56.
        let (x, y) = bottom_right_origin(0, 0, 1920, 1080, 440, 700, 12, 56);
        assert_eq!(x, 1920 - 440 - 12);
        assert_eq!(y, 1080 - 700 - 56);
    }

    #[test]
    fn bottom_right_origin_klemmt_an_monitorkante() {
        // Fenster groesser als Monitor -> Ursprung bleibt an der Monitorkante.
        let (x, y) = bottom_right_origin(200, 300, 400, 400, 800, 900, 12, 56);
        assert_eq!(x, 200);
        assert_eq!(y, 300);
    }
}
