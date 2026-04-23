use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, EventTarget, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

pub const SLIVER_WIDTH_LOGICAL: f64 = 24.0;
pub const EDGE_SNAP_PX: f64 = 40.0;
pub const UNDOCK_THRESHOLD_PX: f64 = 80.0;
pub const ANIM_DURATION_MS: u64 = 160;
pub const ANIM_FRAMES: u32 = 10;
pub const DOCK_DEBOUNCE_MS: u64 = 80;
pub const RECONCILE_DURATION_MS: u64 = 500;
pub const RECONCILE_INTERVAL_MS: u64 = 40;
pub const RECONCILE_TOLERANCE_PX: i32 = 2;

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

struct PendingDock {
    edge: DockEdge,
    last_update: Instant,
    task_alive: bool,
}

enum PendingCheckResult {
    Commit(DockEdge),
    Wait(Duration),
    Empty,
}

struct DockRegistry {
    docked: HashMap<String, DockState>,
    animating: HashSet<String>,
    pending: HashMap<String, PendingDock>,
}

static DOCK_REGISTRY: LazyLock<Mutex<DockRegistry>> = LazyLock::new(|| {
    Mutex::new(DockRegistry {
        docked: HashMap::new(),
        animating: HashSet::new(),
        pending: HashMap::new(),
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
        reg.pending.remove(label);
    }
}

fn record_pending_dock(label: &str, edge: DockEdge) -> bool {
    let Ok(mut reg) = DOCK_REGISTRY.lock() else {
        return false;
    };
    let now = Instant::now();
    match reg.pending.get_mut(label) {
        Some(p) => {
            p.edge = edge;
            p.last_update = now;
            if p.task_alive {
                false
            } else {
                p.task_alive = true;
                true
            }
        }
        None => {
            reg.pending.insert(
                label.to_string(),
                PendingDock {
                    edge,
                    last_update: now,
                    task_alive: true,
                },
            );
            true
        }
    }
}

fn clear_pending_dock(label: &str) {
    if let Ok(mut reg) = DOCK_REGISTRY.lock() {
        reg.pending.remove(label);
    }
}

fn check_pending_dock(label: &str, debounce: Duration) -> PendingCheckResult {
    let Ok(mut reg) = DOCK_REGISTRY.lock() else {
        return PendingCheckResult::Empty;
    };
    let Some(p) = reg.pending.get_mut(label) else {
        return PendingCheckResult::Empty;
    };
    let elapsed = p.last_update.elapsed();
    if elapsed >= debounce {
        let edge = p.edge;
        reg.pending.remove(label);
        PendingCheckResult::Commit(edge)
    } else {
        PendingCheckResult::Wait(debounce - elapsed)
    }
}

fn mark_pending_task_ended(label: &str) {
    if let Ok(mut reg) = DOCK_REGISTRY.lock() {
        if let Some(p) = reg.pending.get_mut(label) {
            p.task_alive = false;
        }
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

    let app_for_phase2 = app.clone();
    let label_for_phase2 = label.clone();
    animate_geometry(
        app.clone(),
        label.clone(),
        current_pos,
        current_size,
        target_pos,
        target_size,
        Some(Box::new(move |_app| {
            reconcile_sliver_position(
                app_for_phase2,
                label_for_phase2,
                edge,
                target_pos,
                target_size,
            );
        })),
    );
}

fn reconcile_sliver_position<R: Runtime>(
    app: AppHandle<R>,
    label: String,
    edge: DockEdge,
    target_pos: PhysicalPosition<i32>,
    target_size: PhysicalSize<u32>,
) {
    if !mark_animating(&label) {
        return;
    }

    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_size(tauri::Size::Physical(target_size));
        let _ = win.set_position(tauri::Position::Physical(target_pos));
        let _ = win.emit_to(
            EventTarget::webview_window(win.label()),
            "shelf-docked",
            serde_json::json!({ "edge": edge.as_str() }),
        );
    }

    tauri::async_runtime::spawn(async move {
        let duration = Duration::from_millis(RECONCILE_DURATION_MS);
        let interval = Duration::from_millis(RECONCILE_INTERVAL_MS);
        let start = Instant::now();
        let mut clean_ticks = 0u32;

        while start.elapsed() < duration {
            tokio::time::sleep(interval).await;

            if !is_docked(&label) {
                break;
            }
            let Some(win) = app.get_webview_window(&label) else {
                break;
            };
            let Ok(actual_pos) = win.outer_position() else {
                continue;
            };
            let Ok(actual_size) = win.outer_size() else {
                continue;
            };

            let pos_off = (actual_pos.x - target_pos.x).abs() > RECONCILE_TOLERANCE_PX
                || (actual_pos.y - target_pos.y).abs() > RECONCILE_TOLERANCE_PX;
            let size_off = actual_size.width != target_size.width;

            if pos_off || size_off {
                let _ = win.set_size(tauri::Size::Physical(target_size));
                let _ = win.set_position(tauri::Position::Physical(target_pos));
                clean_ticks = 0;
            } else {
                clean_ticks += 1;
                if clean_ticks >= 3 {
                    break;
                }
            }
        }

        clear_animating(&label);
    });
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

    let app_clone = app.clone();
    let label_for_emit = label.clone();
    animate_geometry(
        app.clone(),
        label.clone(),
        current_pos,
        current_size,
        state.pre_dock_pos,
        state.pre_dock_size,
        Some(Box::new(move |_app| {
            if let Some(w) = app_clone.get_webview_window(&label_for_emit) {
                let _ = w.emit_to(
                    EventTarget::webview_window(w.label()),
                    "shelf-expanded",
                    serde_json::json!({}),
                );
            }
        })),
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

    let intent = if window_left - screen_left <= snap_phys {
        Some(DockEdge::Left)
    } else if screen_right - window_right <= snap_phys {
        Some(DockEdge::Right)
    } else {
        None
    };

    let Some(edge) = intent else {
        clear_pending_dock(&label);
        return;
    };

    if record_pending_dock(&label, edge) {
        spawn_pending_dock_task(app.clone(), label);
    }
}

fn spawn_pending_dock_task<R: Runtime>(app: AppHandle<R>, label: String) {
    let debounce = Duration::from_millis(DOCK_DEBOUNCE_MS);
    tauri::async_runtime::spawn(async move {
        let mut next_sleep = debounce;
        loop {
            tokio::time::sleep(next_sleep).await;
            match check_pending_dock(&label, debounce) {
                PendingCheckResult::Commit(edge) => {
                    if !is_animating(&label) && !is_docked(&label) {
                        if let Some(win) = app.get_webview_window(&label) {
                            begin_dock(&app, &win, edge);
                        }
                    }
                    return;
                }
                PendingCheckResult::Wait(remaining) => {
                    next_sleep = remaining;
                }
                PendingCheckResult::Empty => {
                    mark_pending_task_ended(&label);
                    return;
                }
            }
        }
    });
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
