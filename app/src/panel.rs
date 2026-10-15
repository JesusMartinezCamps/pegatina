use crate::state::AppState;
use crate::win;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

const CURSOR_GAP: i32 = 12;
// Windows a veces avisa de una pérdida de foco justo al mostrar la ventana.
const BLUR_GRACE_MS: u64 = 250;

#[derive(Clone, Copy)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn place_panel(cursor: (i32, i32), size: (i32, i32), area: Area) -> (i32, i32) {
    let right = area.x + area.w;
    let bottom = area.y + area.h;
    let mut x = cursor.0 + CURSOR_GAP;
    if x + size.0 > right {
        x = cursor.0 - CURSOR_GAP - size.0;
    }
    let mut y = cursor.1 + CURSOR_GAP;
    if y + size.1 > bottom {
        y = cursor.1 - CURSOR_GAP - size.1;
    }
    (
        x.min(right - size.0).max(area.x),
        y.min(bottom - size.1).max(area.y),
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn position_near_cursor(app: &AppHandle, panel: &WebviewWindow) -> tauri::Result<()> {
    let cursor = app.cursor_position()?;
    let monitor = match app.monitor_from_point(cursor.x, cursor.y)? {
        Some(m) => Some(m),
        None => app.primary_monitor()?,
    };
    let Some(monitor) = monitor else {
        return Ok(());
    };
    let work = monitor.work_area();
    let ratio = monitor.scale_factor() / panel.scale_factor()?;
    let size = panel.outer_size()?;
    let (x, y) = place_panel(
        (cursor.x as i32, cursor.y as i32),
        (
            (size.width as f64 * ratio).round() as i32,
            (size.height as f64 * ratio).round() as i32,
        ),
        Area {
            x: work.position.x,
            y: work.position.y,
            w: work.size.width as i32,
            h: work.size.height as i32,
        },
    );
    panel.set_position(PhysicalPosition::new(x, y))
}

fn show(app: &AppHandle, remember_window: bool) {
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };
    let state = app.state::<AppState>();
    let previous = if remember_window {
        win::foreground_window()
    } else {
        0
    };
    state.previous_window.store(previous, Ordering::SeqCst);
    if let Err(e) = position_near_cursor(app, &panel) {
        eprintln!("No se pudo colocar el panel junto al cursor: {e}");
    }
    let _ = panel.show();
    let _ = panel.set_focus();
    state.panel_shown_at.store(now_ms(), Ordering::SeqCst);
    let _ = app.emit_to("panel", "panel-shown", ());
}

pub fn hide(app: &AppHandle) {
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    let visible = app
        .get_webview_window("panel")
        .and_then(|p| p.is_visible().ok())
        .unwrap_or(false);
    if visible {
        hide(app);
    } else {
        show(app, true);
    }
}

/// Desde la bandeja la ventana en primer plano es la barra de tareas: no hay dónde pegar.
pub fn open_from_tray(app: &AppHandle) {
    show(app, false);
}

pub fn hide_if_blurred(app: &AppHandle) {
    let shown_at = app
        .state::<AppState>()
        .panel_shown_at
        .load(Ordering::SeqCst);
    if now_ms().saturating_sub(shown_at) > BLUR_GRACE_MS {
        hide(app);
    }
}

pub fn show_main(app: &AppHandle, section: &str) {
    hide(app);
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    let section = if section == "settings" {
        "settings"
    } else {
        "library"
    };
    let _ = main.eval(format!("window.location.hash = '#{section}'"));
    let _ = main.unminimize();
    let _ = main.show();
    let _ = main.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Area = Area {
        x: 0,
        y: 0,
        w: 1920,
        h: 1040,
    };

    #[test]
    fn opens_below_and_right_of_the_cursor() {
        assert_eq!(place_panel((500, 300), (400, 600), AREA), (512, 312));
    }

    #[test]
    fn flips_left_and_up_near_the_far_corner() {
        assert_eq!(place_panel((1800, 900), (400, 600), AREA), (1388, 288));
    }

    #[test]
    fn stays_inside_the_work_area_on_a_second_monitor() {
        let left = Area {
            x: -1920,
            y: 0,
            w: 1920,
            h: 1040,
        };
        let (x, y) = place_panel((-1900, 10), (400, 600), left);
        assert!(x >= left.x && x + 400 <= left.x + left.w);
        assert!(y >= 0 && y + 600 <= 1040);
    }

    #[test]
    fn panel_larger_than_the_area_is_pinned_to_the_corner() {
        assert_eq!(
            place_panel(
                (50, 50),
                (400, 600),
                Area {
                    x: 0,
                    y: 0,
                    w: 300,
                    h: 300
                }
            ),
            (0, 0)
        );
    }
}
