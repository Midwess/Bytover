use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};
use tauri::webview::Color;
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{Emitter, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_positioner::{Position, WindowExt};

// ── Shelf slot registry ───────────────────────────────────────────────────────
//
// Each monitor has a full-screen grid of (col, row) slots that tiles its entire
// surface.  The grid dimensions are computed at runtime from the screen size so
// the mouse always lands near a slot.  Slots are keyed by
// (monitor_hash, col, row); col 0 is the rightmost column, row 0 the top.
//
// At most MAX_SHELVES windows are open globally.  When a new shelf would exceed
// that limit the oldest one is closed first (front of `creation_order`).

const MAX_SHELVES: usize = 8;

struct ShelfRegistry {
    /// (monitor_hash, col, row) → window label
    slots: HashMap<(u64, usize, usize), String>,
    /// Labels in creation order; front = oldest, back = newest
    creation_order: VecDeque<String>,
}

static SHELF_REGISTRY: LazyLock<Mutex<ShelfRegistry>> = LazyLock::new(|| {
    Mutex::new(ShelfRegistry {
        slots: HashMap::new(),
        creation_order: VecDeque::new(),
    })
});

/// Stable identity for a monitor derived from its physical position and size.
fn monitor_hash(monitor: &tauri::Monitor) -> u64 {
    let pos = monitor.position();
    let size = monitor.size();
    let mut h: u64 = pos.x as u64;
    h = h.wrapping_mul(1_000_003).wrapping_add(pos.y as u64);
    h = h.wrapping_mul(1_000_003).wrapping_add(size.width as u64);
    h = h.wrapping_mul(1_000_003).wrapping_add(size.height as u64);
    h
}

// Grid layout in logical pixels ───────────────────────────────────────────────
const WIN_WIDTH: f64 = 245.0;
const WIN_HEIGHT: f64 = 270.0;
const CELL_W: f64 = WIN_WIDTH * 1.1; // 10 % horizontal padding
const CELL_H: f64 = WIN_HEIGHT * 1.1; // 10 % vertical padding
const MARGIN: f64 = 50.0; // uniform margin on every edge (logical px)

/// How many columns and rows fit inside `monitor`, covering the full screen.
/// Uses the same `MARGIN` on all four sides so the grid is always fully visible.
fn grid_dimensions(monitor: &tauri::Monitor) -> (usize, usize) {
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let p_margin = MARGIN * scale;
    let cols = ((size.width as f64 - 2.0 * p_margin) / (CELL_W * scale)).floor() as usize;
    let rows = ((size.height as f64 - 2.0 * p_margin) / (CELL_H * scale)).floor() as usize;
    (cols.max(1), rows.max(1))
}

/// Returns `(center_x, center_y, win_top_x, win_top_y)` in **physical pixels**.
///
/// `col 0` is the rightmost column, `row 0` is the top row.
/// The grid fills the screen; use `grid_dimensions()` to get valid col/row ranges.
fn slot_physics(monitor: &tauri::Monitor, col: usize, row: usize) -> (f64, f64, f64, f64) {
    let scale = monitor.scale_factor();
    let pos = monitor.position();
    let size = monitor.size();
    let p_cell_w = CELL_W * scale;
    let p_cell_h = CELL_H * scale;
    let p_win_w = WIN_WIDTH * scale;
    let p_win_h = WIN_HEIGHT * scale;
    let p_margin = MARGIN * scale;

    let sx = pos.x as f64;
    let sy = pos.y as f64;
    let sw = size.width as f64;

    // Top-left of the cell
    let cell_x = sx + sw - p_margin - ((col as f64 + 1.0) * p_cell_w);
    let cell_y = sy + p_margin + (row as f64 * p_cell_h);

    let cx = cell_x + p_cell_w / 2.0;
    let cy = cell_y + p_cell_h / 2.0;

    (cx, cy, cx - p_win_w / 2.0, cy - p_win_h / 2.0)
}

/// Called from `on_window_event(Destroyed)` to free the slot.
fn release_registry_slot(label: &str) {
    if let Ok(mut reg) = SHELF_REGISTRY.lock() {
        reg.slots.retain(|_, v| v != label);
        reg.creation_order.retain(|l| l != label);
    }
}

// ── Monitor helper ────────────────────────────────────────────────────────────

// ── Trait definition ──────────────────────────────────────────────────────────

pub trait AppHandleExt<R: Runtime> {
    fn close_all_windows(&self, whitelist: Vec<&str>);
    fn show_auth(&self) -> WebviewWindow<R>;
    fn create_receive(&self) -> WebviewWindow<R>;
    fn show_send(&self) -> WebviewWindow<R>;
    fn show_shelf(&self, shelf_id: u64, mouse_pos: Option<tauri::PhysicalPosition<f64>>) -> WebviewWindow<R>;
    fn open_new_shelf_window(&self, mouse_pos: Option<tauri::PhysicalPosition<f64>>) -> WebviewWindow<R>;
    fn show_settings(&self) -> WebviewWindow<R>;
    fn show_settings_with_tab(&self, tab: &str) -> WebviewWindow<R>;
    fn hide_auth(&self);
    fn show_intro(&self) -> WebviewWindow<R>;
    fn hide_intro(&self);
    fn toggle_receive(&self);
    fn is_shelf_window_open(&self, id: u64) -> bool;
    fn is_any_shelf_window_open(&self) -> bool;
    fn get_visible_shelf_windows(&self) -> Vec<WebviewWindow<R>>;
    fn hide_send(&self);
    fn close_all_shelves(&self);
    fn show_toast(&self, message: &str) -> WebviewWindow<R>;
}

fn animate_window<R: Runtime>(window: WebviewWindow<R>) {
    let _ = window.show();
    let _ = window.set_focus();
}

// ── Trait implementation ──────────────────────────────────────────────────────

impl<R: Runtime> AppHandleExt<R> for tauri::AppHandle<R> {
    fn close_all_windows(&self, whitelist: Vec<&str>) {
        for (_, window) in self.webview_windows() {
            if whitelist.contains(&window.label()) {
                continue;
            }

            let _ = window.close();
        }
    }

    fn show_auth(&self) -> WebviewWindow<R> {
        self.close_all_windows(vec!["auth", "intro"]);

        let window = match self.get_webview_window("auth") {
            Some(window) => window,
            None => WebviewWindowBuilder::new(self, "auth", WebviewUrl::App("auth.html".into()))
                .title("Bytover")
                .inner_size(600.0, 600.0)
                .decorations(true)
                .transparent(true)
                .focused(true)
                .skip_taskbar(false)
                .resizable(false)
                .shadow(true)
                .devtools(true)
                .build()
                .expect("failed to create auth window"),
        };

        let _ = window.show();
        window
    }

    fn create_receive(&self) -> WebviewWindow<R> {
        let window = match self.get_webview_window("receive") {
            Some(window) => window,
            None => {
                let window = WebviewWindowBuilder::new(self, "receive", WebviewUrl::App("receive.html".into()))
                    .title("receive")
                    .inner_size(490.0, 300.0)
                    .decorations(false)
                    .transparent(true)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(false)
                    .shadow(true)
                    .devtools(true)
                    .build()
                    .expect("failed to create receive window");

                let _ = window.set_effects(
                    EffectsBuilder::new()
                        .effect(Effect::HudWindow)
                        .effect(Effect::Blur)
                        .state(EffectState::Active)
                        .radius(25.0)
                        .color(Color(0, 0, 0, 0))
                        .build(),
                );

                window
            }
        };

        window
    }

    fn toggle_receive(&self) {
        if let Some(window) = self.get_webview_window("receive") {
            if !window.is_visible().unwrap_or_default() {
                let _ = window.show();
                let _ = window.move_window(Position::TrayBottomCenter);
            } else {
                let _ = window.hide();
            }
        } else {
            let window = self.create_receive();
            let _ = window.show();
            let _ = window.move_window(Position::TrayBottomCenter);
        }
    }

    fn show_send(&self) -> WebviewWindow<R> {
        self.close_all_windows(vec!["send"]);
        let window = match self.get_webview_window("send") {
            Some(window) => window,
            None => WebviewWindowBuilder::new(self, "send", WebviewUrl::App("send.html".into()))
                .title("send")
                .inner_size(245.0, 270.0)
                .resizable(false)
                .decorations(false)
                .transparent(true)
                .visible_on_all_workspaces(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .shadow(false)
                .devtools(true)
                .build()
                .expect("failed to create send window"),
        };

        animate_window(window.clone());
        let _ = window.emit("window-shown", {});
        window
    }

    fn show_shelf(&self, shelf_id: u64, mouse_pos: Option<tauri::PhysicalPosition<f64>>) -> WebviewWindow<R> {
        let label = format!("send-{}", shelf_id);

        // Create the window if it does not yet exist.
        let window = match self.get_webview_window(&label) {
            Some(w) => w,
            None => {
                let w = WebviewWindowBuilder::new(self, &label, WebviewUrl::App("send.html".into()))
                    .title(&label)
                    .inner_size(WIN_WIDTH, WIN_HEIGHT)
                    .resizable(false)
                    .decorations(false)
                    .transparent(true)
                    .visible_on_all_workspaces(true)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .shadow(false)
                    .devtools(true)
                    .build()
                    .expect("failed to create shelf window");

                let label_clone = label.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::Destroyed = event {
                        release_registry_slot(&label_clone);
                    }
                });
                w
            }
        };

        // Collect all monitors.  If none are found fall back to just showing.
        let monitors = self.available_monitors().unwrap_or_default();
        if monitors.is_empty() {
            animate_window(window.clone());
            return window;
        }

        // Build the full candidate list: every (col, row) slot on every monitor.
        // grid_dimensions() fills the screen, so no matter where the mouse is a
        // nearby slot always exists.
        //
        // Tuple layout:
        //   0:mh  1:col  2:row  3:cx  4:cy  5:wx  6:wy  7:win_right  8:win_bottom
        //
        // win_right / win_bottom are the exclusive right/bottom edges of the
        // window in physical pixels — used to test whether the mouse falls inside.
        let mut candidates: Vec<(u64, usize, usize, f64, f64, f64, f64, f64, f64)> = Vec::new();
        for monitor in &monitors {
            let mh = monitor_hash(monitor);
            let scale = monitor.scale_factor();
            let win_w = WIN_WIDTH * scale;
            let win_h = WIN_HEIGHT * scale;
            let (num_cols, num_rows) = grid_dimensions(monitor);
            for col in 0..num_cols {
                for row in 0..num_rows {
                    let (cx, cy, wx, wy) = slot_physics(monitor, col, row);
                    candidates.push((mh, col, row, cx, cy, wx, wy, wx + win_w, wy + win_h));
                }
            }
        }

        // ── Phase 1: evict oldest shelves until there is room (lock held briefly) ──
        let to_evict: Vec<String> = {
            let Ok(mut reg) = SHELF_REGISTRY.lock() else {
                animate_window(window.clone());
                return window;
            };
            // Re-showing an existing shelf: remove it first so it gets a fresh slot.
            reg.slots.retain(|_, v| v != &label);
            reg.creation_order.retain(|l| l != &label);

            let mut evicted = Vec::new();
            while reg.creation_order.len() >= MAX_SHELVES {
                if let Some(oldest) = reg.creation_order.pop_front() {
                    reg.slots.retain(|_, v| v != &oldest);
                    evicted.push(oldest);
                } else {
                    break;
                }
            }
            evicted
        };

        // ── Phase 2: close evicted windows (outside lock) ─────────────────────────
        for evict_label in &to_evict {
            if let Some(w) = self.get_webview_window(evict_label) {
                let _ = w.close();
            }
        }

        // ── Phase 3: pick the best free slot and record it ────────────────────────
        let primary_hash = self.primary_monitor().ok().flatten().map(|m| monitor_hash(&m));

        let chosen_pos: Option<(f64, f64)> = {
            let Ok(mut reg) = SHELF_REGISTRY.lock() else {
                animate_window(window.clone());
                return window;
            };

            let chosen = if let Some(pos) = mouse_pos {
                // Mouse-triggered: find the closest FREE slot whose window would
                // NOT overlap the current mouse cursor position.
                candidates
                    .iter()
                    .filter(|c| {
                        // Slot must be unoccupied …
                        let free = !reg.slots.contains_key(&(c.0, c.1, c.2));
                        // … and the window rectangle must not contain the mouse.
                        // c.5=wx  c.6=wy  c.7=win_right  c.8=win_bottom
                        let under_mouse = pos.x >= c.5 && pos.x < c.7 && pos.y >= c.6 && pos.y < c.8;
                        free && !under_mouse
                    })
                    .min_by(|a, b| {
                        let da = (pos.x - a.3).powi(2) + (pos.y - a.4).powi(2);
                        let db = (pos.x - b.3).powi(2) + (pos.y - b.4).powi(2);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|c| (c.0, c.1, c.2, c.5, c.6))
            } else {
                // Button-triggered: primary monitor first; within each monitor
                // col 0 (rightmost) before col 1, row 0 (top) before row 1, …
                // This ensures the top-right slot fills first.
                let mut sorted = candidates.clone();
                sorted.sort_by(|a, b| {
                    let a_primary = primary_hash.map_or(false, |ph| ph == a.0);
                    let b_primary = primary_hash.map_or(false, |ph| ph == b.0);
                    b_primary.cmp(&a_primary)  // primary first
                        .then(a.1.cmp(&b.1))   // col ascending (0 = rightmost)
                        .then(a.2.cmp(&b.2)) // row ascending (0 = top)
                });
                sorted.iter().find(|c| !reg.slots.contains_key(&(c.0, c.1, c.2))).map(|c| (c.0, c.1, c.2, c.5, c.6))
            };

            if let Some((mh, col, row, wx, wy)) = chosen {
                reg.slots.insert((mh, col, row), label.clone());
                reg.creation_order.push_back(label.clone());
                Some((wx, wy))
            } else {
                None
            }
        };

        if let Some((win_x, win_y)) = chosen_pos {
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: win_x as i32,
                y: win_y as i32,
            }));
        }

        animate_window(window.clone());
        window
    }

    fn open_new_shelf_window(&self, mouse_pos: Option<tauri::PhysicalPosition<f64>>) -> WebviewWindow<R> {
        let shelf_id = shared::gen_shelf_id();
        self.show_shelf(shelf_id, mouse_pos)
    }

    fn show_settings_with_tab(&self, tab: &str) -> WebviewWindow<R> {
        let window = match self.get_webview_window("settings") {
            Some(window) => window,
            None => {
                let window = WebviewWindowBuilder::new(self, "settings", WebviewUrl::App(format!("settings.html?tab={}", tab).into()))
                    .title("Settings")
                    .inner_size(560.0, 373.0)
                    .decorations(true)
                    .transparent(true)
                    .resizable(false)
                    .shadow(true)
                    .devtools(true)
                    .build()
                    .expect("failed to create settings window");

                let _ = window.set_effects(
                    EffectsBuilder::new()
                        .effect(Effect::HudWindow)
                        .state(EffectState::Active)
                        .radius(10.0)
                        .color(Color(30, 30, 30, 220))
                        .build(),
                );

                window
            }
        };

        let _ = window.show();
        let _ = window.set_focus();
        window
    }

    fn show_settings(&self) -> WebviewWindow<R> {
        let window = match self.get_webview_window("settings") {
            Some(window) => window,
            None => {
                let window = WebviewWindowBuilder::new(self, "settings", WebviewUrl::App("settings.html".into()))
                    .title("Settings")
                    .inner_size(560.0, 373.0)
                    .decorations(true)
                    .transparent(true)
                    .resizable(false)
                    .shadow(true)
                    .devtools(true)
                    .build()
                    .expect("failed to create settings window");

                let _ = window.set_effects(
                    EffectsBuilder::new()
                        .effect(Effect::HudWindow)
                        .state(EffectState::Active)
                        .radius(10.0)
                        .color(Color(30, 30, 30, 220))
                        .build(),
                );

                window
            }
        };

        let _ = window.show();
        let _ = window.set_focus();
        window
    }

    fn hide_auth(&self) {
        if let Some(window) = self.get_webview_window("auth") {
            let _ = window.hide();
        }
    }

    fn show_intro(&self) -> WebviewWindow<R> {
        match self.get_webview_window("intro") {
            Some(window) => {
                let _ = window.show();
                let _ = window.set_focus();
                window
            }
            None => {
                let window = WebviewWindowBuilder::new(self, "intro", WebviewUrl::App("intro.html".into()))
                    .title("Welcome to Bytover")
                    .inner_size(690.0, 690.0)
                    .decorations(true)
                    .transparent(true)
                    .focused(true)
                    .skip_taskbar(false)
                    .resizable(false)
                    .shadow(true)
                    .devtools(true)
                    .build()
                    .expect("failed to create intro window");

                let _ = window.show();
                window
            }
        }
    }

    fn hide_intro(&self) {
        if let Some(window) = self.get_webview_window("intro") {
            let _ = window.close();
        }
    }

    fn is_shelf_window_open(&self, id: u64) -> bool {
        self.get_webview_window(&format!("send-{id}"))
            .map(|it| it.is_visible().unwrap_or_default())
            .unwrap_or_default()
    }

    fn is_any_shelf_window_open(&self) -> bool {
        self.webview_windows()
            .iter()
            .any(|(label, window)| label.starts_with("send-") && window.is_visible().unwrap_or_default())
    }

    fn get_visible_shelf_windows(&self) -> Vec<WebviewWindow<R>> {
        self.webview_windows()
            .into_iter()
            .filter(|(label, window)| label.starts_with("send-") && window.is_visible().unwrap_or_default())
            .map(|(_, window)| window)
            .collect()
    }

    fn hide_send(&self) {
        if let Some(window) = self.get_webview_window("send") {
            let _ = window.hide();
        }
    }

    fn close_all_shelves(&self) {
        for (label, window) in self.webview_windows() {
            if label.starts_with("send-") {
                let _ = window.close();
            }
        }
    }

    fn show_toast(&self, message: &str) -> WebviewWindow<R> {
        let window = match self.get_webview_window("toast") {
            Some(window) => window,
            None => {
                let window = WebviewWindowBuilder::new(self, "toast", WebviewUrl::App("toast.html".into()))
                    .title("toast")
                    .inner_size(300.0, 44.0)
                    .decorations(false)
                    .transparent(true)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(false)
                    .shadow(false)
                    .focused(false)
                    .build()
                    .expect("failed to create toast window");

                let _ = window.set_effects(
                    EffectsBuilder::new()
                        .effect(Effect::HudWindow)
                        .effect(Effect::Blur)
                        .state(EffectState::Active)
                        .radius(22.0)
                        .color(Color(0, 0, 0, 0))
                        .build(),
                );

                window
            }
        };

        if let Some(monitor) = window.current_monitor().ok().flatten() {
            let screen_size = monitor.size();
            let screen_position = monitor.position();
            let scale = monitor.scale_factor();

            // Logical dimensions
            let window_width = 300.0;
            let window_height = 44.0;
            // Position above taskbar (macOS Dock ~70px, Windows taskbar ~48px) + 20px buffer
            let padding_bottom = 90.0;

            let x = (screen_size.width as f64 - window_width * scale) / 2.0;
            let y = screen_size.height as f64 - (window_height + padding_bottom) * scale;

            let _ = window.set_position(tauri::PhysicalPosition::new(
                screen_position.x + x as i32,
                screen_position.y + y as i32,
            ));
        }

        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("toast-message", message.to_string());

        window
    }
}
