use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

pub const SLIVER_WIDTH_LOGICAL: f64 = 24.0;
pub const EDGE_SNAP_PX: f64 = 40.0;
pub const UNDOCK_THRESHOLD_PX: f64 = 80.0;
pub const ANIM_DURATION_MS: u64 = 160;
pub const ANIM_FRAMES: u32 = 10;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DockEdge {
    Left,
    Right,
}

impl DockEdge {
    pub fn as_str(&self) -> &'static str {
        match self {
            DockEdge::Left => "left",
            DockEdge::Right => "right",
        }
    }

    pub fn parse(raw: &str) -> Option<DockEdge> {
        match raw {
            "left" => Some(DockEdge::Left),
            "right" => Some(DockEdge::Right),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DockState {
    pub edge: DockEdge,
    pub pre_dock_size: PhysicalSize<u32>,
    pub pre_dock_pos: PhysicalPosition<i32>,
}

struct DockRegistry {
    docked: HashMap<String, DockState>,
    animating: HashSet<String>,
}

static DOCK_REGISTRY: LazyLock<Mutex<DockRegistry>> = LazyLock::new(|| {
    Mutex::new(DockRegistry {
        docked: HashMap::new(),
        animating: HashSet::new(),
    })
});

pub fn is_docked(label: &str) -> bool {
    DOCK_REGISTRY
        .lock()
        .map(|reg| reg.docked.contains_key(label))
        .unwrap_or(false)
}

pub fn is_animating(label: &str) -> bool {
    DOCK_REGISTRY
        .lock()
        .map(|reg| reg.animating.contains(label))
        .unwrap_or(false)
}

pub fn dock_state(label: &str) -> Option<DockState> {
    DOCK_REGISTRY
        .lock()
        .ok()
        .and_then(|reg| reg.docked.get(label).cloned())
}

pub fn release_dock(label: &str) {
    if let Ok(mut reg) = DOCK_REGISTRY.lock() {
        reg.docked.remove(label);
        reg.animating.remove(label);
    }
}

fn mark_animating(label: &str) -> bool {
    let Ok(mut reg) = DOCK_REGISTRY.lock() else {
        return false;
    };
    if reg.animating.contains(label) {
        return false;
    }
    reg.animating.insert(label.to_string());
    true
}

fn clear_animating(label: &str) {
    if let Ok(mut reg) = DOCK_REGISTRY.lock() {
        reg.animating.remove(label);
    }
}

fn store_dock(label: &str, state: DockState) {
    if let Ok(mut reg) = DOCK_REGISTRY.lock() {
        reg.docked.insert(label.to_string(), state);
    }
}

fn ease_out_cubic(t: f64) -> f64 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn is_shelf_label(label: &str) -> bool {
    label.starts_with("send-") && label != "send"
}

pub fn animate_geometry<R: Runtime>(
    app: AppHandle<R>,
    label: String,
    start_pos: PhysicalPosition<i32>,
    start_size: PhysicalSize<u32>,
    end_pos: PhysicalPosition<i32>,
    end_size: PhysicalSize<u32>,
    on_complete: Option<Box<dyn FnOnce(&AppHandle<R>) + Send>>,
) {
    if !mark_animating(&label) {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let frame_delay = Duration::from_millis(ANIM_DURATION_MS / ANIM_FRAMES as u64);

        for frame in 1..=ANIM_FRAMES {
            let Some(window) = app.get_webview_window(&label) else {
                break;
            };

            let t = ease_out_cubic(frame as f64 / ANIM_FRAMES as f64);
            let x = lerp(start_pos.x as f64, end_pos.x as f64, t) as i32;
            let y = lerp(start_pos.y as f64, end_pos.y as f64, t) as i32;
            let w = lerp(start_size.width as f64, end_size.width as f64, t) as u32;
            let h = lerp(start_size.height as f64, end_size.height as f64, t) as u32;

            let _ = window.set_size(tauri::Size::Physical(PhysicalSize { width: w.max(1), height: h.max(1) }));
            let _ = window.set_position(tauri::Position::Physical(PhysicalPosition { x, y }));

            tokio::time::sleep(frame_delay).await;
        }

        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.set_size(tauri::Size::Physical(end_size));
            let _ = window.set_position(tauri::Position::Physical(end_pos));
        }

        clear_animating(&label);

        if let Some(cb) = on_complete {
            cb(&app);
        }
    });
}

pub fn begin_dock<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>, edge: DockEdge) {
    let label = window.label().to_string();
    if !is_shelf_label(&label) {
        return;
    }
    if is_docked(&label) || is_animating(&label) {
        return;
    }

    let Ok(current_pos) = window.outer_position() else {
        return;
    };
    let Ok(current_size) = window.outer_size() else {
        return;
    };

    let Some(monitor) = window.current_monitor().ok().flatten() else {
        return;
    };
    let scale = monitor.scale_factor();
    let m_pos = monitor.position();
    let m_size = monitor.size();

    let sliver_width_phys = (SLIVER_WIDTH_LOGICAL * scale).round().max(20.0) as u32;

    let clamped_pre_dock_pos = clamp_pos_to_monitor(current_pos, current_size, *m_pos, *m_size);

    let target_x = match edge {
        DockEdge::Left => m_pos.x,
        DockEdge::Right => m_pos.x + m_size.width as i32 - sliver_width_phys as i32,
    };
    let target_y = clamped_pre_dock_pos.y;

    let target_pos = PhysicalPosition { x: target_x, y: target_y };
    let target_size = PhysicalSize { width: sliver_width_phys, height: current_size.height };

    let state = DockState {
        edge,
        pre_dock_size: current_size,
        pre_dock_pos: clamped_pre_dock_pos,
    };
    store_dock(&label, state);

    let app_clone = app.clone();
    let label_for_emit = label.clone();
    let label_for_check = label.clone();
    let monitor_left = m_pos.x;
    let monitor_right = m_pos.x + m_size.width as i32;
    animate_geometry(
        app.clone(),
        label.clone(),
        current_pos,
        current_size,
        target_pos,
        target_size,
        Some(Box::new(move |_app| {
            if let Some(w) = app_clone.get_webview_window(&label_for_emit) {
                if let Ok(final_pos) = w.outer_position() {
                    if final_pos.x < monitor_left || final_pos.x >= monitor_right {
                        log::warn!(
                            "shelf_dock: post-dock position {:?} for {} is outside monitor [{}, {})",
                            final_pos, label_for_check, monitor_left, monitor_right
                        );
                    }
                }
                let _ = w.emit("shelf-docked", serde_json::json!({ "edge": edge.as_str() }));
            }
        })),
    );
}

fn clamp_pos_to_monitor(
    pos: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    m_pos: PhysicalPosition<i32>,
    m_size: PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let min_x = m_pos.x;
    let max_x = m_pos.x + m_size.width as i32 - size.width as i32;
    let min_y = m_pos.y;
    let max_y = m_pos.y + m_size.height as i32 - size.height as i32;

    let x = if max_x >= min_x { pos.x.clamp(min_x, max_x) } else { pos.x };
    let y = if max_y >= min_y { pos.y.clamp(min_y, max_y) } else { pos.y };

    PhysicalPosition { x, y }
}

pub fn begin_expand<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    let label = window.label().to_string();
    if !is_shelf_label(&label) {
        return;
    }
    if is_animating(&label) {
        return;
    }
    let Some(state) = dock_state(&label) else {
        return;
    };

    let Ok(current_pos) = window.outer_position() else {
        return;
    };
    let Ok(current_size) = window.outer_size() else {
        return;
    };

    release_dock(&label);

    let _ = window.emit("shelf-expanded", serde_json::json!({}));

    animate_geometry(
        app.clone(),
        label.clone(),
        current_pos,
        current_size,
        state.pre_dock_pos,
        state.pre_dock_size,
        None,
    );
}

pub fn maybe_detect_edge_dock<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    let label = window.label().to_string();
    if !is_shelf_label(&label) {
        return;
    }
    if is_animating(&label) {
        return;
    }

    let Ok(pos) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let Some(monitor) = window.current_monitor().ok().flatten() else {
        return;
    };
    let scale = monitor.scale_factor();
    let m_pos = monitor.position();
    let m_size = monitor.size();

    let window_left = pos.x as f64;
    let window_right = pos.x as f64 + size.width as f64;
    let screen_left = m_pos.x as f64;
    let screen_right = m_pos.x as f64 + m_size.width as f64;

    let snap_phys = EDGE_SNAP_PX * scale;
    let undock_phys = UNDOCK_THRESHOLD_PX * scale;

    if let Some(state) = dock_state(&label) {
        let distance_from_edge = match state.edge {
            DockEdge::Left => window_left - screen_left,
            DockEdge::Right => screen_right - window_right,
        };
        if distance_from_edge > undock_phys {
            begin_expand(app, window);
        }
        return;
    }

    if window_left - screen_left <= snap_phys {
        begin_dock(app, window, DockEdge::Left);
    } else if screen_right - window_right <= snap_phys {
        begin_dock(app, window, DockEdge::Right);
    }
}

#[tauri::command]
pub fn dock_shelf_edge(app: AppHandle, label: String, edge: String) {
    if !is_shelf_label(&label) {
        return;
    }
    let Some(edge) = DockEdge::parse(&edge) else {
        return;
    };
    let Some(window) = app.get_webview_window(&label) else {
        return;
    };
    begin_dock(&app, &window, edge);
}

#[tauri::command]
pub fn expand_shelf(app: AppHandle, label: String) {
    if !is_shelf_label(&label) {
        return;
    }
    let Some(window) = app.get_webview_window(&label) else {
        return;
    };
    begin_expand(&app, &window);
}
