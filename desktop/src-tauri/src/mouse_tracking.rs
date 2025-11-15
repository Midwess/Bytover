use std::sync::atomic::{AtomicBool, Ordering};

use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};
use rdev::{set_is_main_thread, Button, EventType};
use tauri::{AppHandle, LogicalPosition, Manager, PhysicalPosition, PhysicalSize};
use crate::extensions::AppHandleExt;

static USER_DID_DROP: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
static MACOS_DRAG_HAS_ITEMS: AtomicBool = AtomicBool::new(false);

static DRAG_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn notify_user_did_drop() {
    USER_DID_DROP.store(true, Ordering::SeqCst);
}

pub fn drag_start_gesture() {
    DRAG_ACTIVE.store(true, Ordering::SeqCst);
}

pub fn drag_end_gesture() {
    DRAG_ACTIVE.store(false, Ordering::SeqCst);
}

pub fn detect_drag(start: &PhysicalPosition<f64>, current: &PhysicalPosition<f64>) -> bool {
    if !DRAG_ACTIVE.load(Ordering::SeqCst) {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::Mouse::DragDetect;
        use windows::Win32::Foundation::{HWND, POINT};

        let pt = POINT { x: start.x, y: start.y };
        return unsafe { DragDetect(HWND(0), pt).as_bool() };
    }

    #[cfg(target_os = "macos")]
    {
        return MACOS_DRAG_HAS_ITEMS.load(Ordering::SeqCst)
    }

    const THRESHOLD: f64 = 30f64;
    let dx = (current.x - start.x).abs();
    let dy = (current.y - start.y).abs();
    dx > THRESHOLD || dy > THRESHOLD
}

#[cfg(target_os = "macos")]
pub fn start_macos_drag_pasteboard_monitor() {
    use cacao::pasteboard::{Pasteboard, PasteboardName};
    use dispatch::Queue;

    let queue = Queue::main();

    thread::spawn(move || {
        let mut current_dragging = true;
        loop {
            let is_active = DRAG_ACTIVE.load(Ordering::Relaxed);
            if !is_active {
                queue.exec_sync(move || {
                    // Schedule periodic checks
                    let pb = Pasteboard::named(PasteboardName::Drag);
                    pb.release_globally();
                });
            }

            let is_dragging = is_active && queue.exec_sync(move || {
                let pb = Pasteboard::named(PasteboardName::Drag);
                pb.get_file_urls().map(|it| !it.is_empty()).unwrap_or_default()
            });

            if is_dragging != current_dragging {
                current_dragging = is_dragging;
                MACOS_DRAG_HAS_ITEMS.store(current_dragging, Ordering::SeqCst);
            }

            sleep(Duration::from_millis(250));
        };
    });
}

#[derive(Debug, Clone)]
pub struct MouseMonitorConfig {
    pub required_shakes: u32,
    pub shake_timeout: Duration,
    pub min_changed: f64,
}

impl Default for MouseMonitorConfig {
    fn default() -> Self {
        Self {
            required_shakes: 3,
            min_changed: 50f64,
            shake_timeout: Duration::from_millis(2000)
        }
    }
}

fn get_monitor_at_position(physical_pos: &PhysicalPosition<f64>, app_handle: &AppHandle) -> Option<tauri::Monitor> {
    app_handle
        .available_monitors()
        .ok()
        .and_then(|monitors| {
            monitors.iter().find(|m| {
                let monitor_pos = m.position();
                let monitor_size = m.size();

                let x = physical_pos.x as i32;
                let y = physical_pos.y as i32;

                x >= monitor_pos.x
                    && x < (monitor_pos.x + monitor_size.width as i32)
                    && y >= monitor_pos.y
                    && y < (monitor_pos.y + monitor_size.height as i32)
            }).cloned()
        })
}

fn calculate_window_position(
    mouse_physical: &PhysicalPosition<f64>,
    window_physical_size: &PhysicalSize<u32>,
    monitor: &tauri::Monitor,
) -> PhysicalPosition<i32> {
    let margin = 200f64;
    let edge_margin = 50f64;
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();

    let screen_left = monitor_pos.x as f64 + edge_margin;
    let max_top = monitor_pos.y as f64 + edge_margin;
    let screen_right = monitor_pos.x as f64 + monitor_size.width as f64 - edge_margin;
    let max_bottom = monitor_pos.y as f64 + monitor_size.height as f64 - edge_margin;

    let mouse_x = mouse_physical.x;
    let mouse_y = mouse_physical.y;
    let window_width = window_physical_size.width as f64;
    let window_height = window_physical_size.height as f64;

    // Calculate desired position (top-right with margin)
    let mut desired_x = mouse_x + margin;
    let mut desired_y = mouse_y + margin;

    // Check if window would go beyond right edge
    if desired_x + window_width > screen_right {
        desired_x = mouse_x - margin - window_width;

        // Check if window would go beyond left edge too
        if desired_x < screen_left {
            desired_x = screen_left;

            // Move down if top is too close
            if desired_y < max_top {
                desired_y = mouse_y + margin;
            }
        }
    } else if desired_y - window_height < max_top {
        desired_y = mouse_y + margin;

        if desired_y + window_height > max_bottom {
            desired_y = max_bottom - window_height;
        }

        // Determine left/right position for bottom
        if desired_x + window_width > screen_right {
            desired_x = mouse_x - margin - window_width;
        }
    }

    // Clamp to screen boundaries
    desired_x = desired_x.max(screen_left).min(screen_right - window_width);
    desired_y = desired_y.max(max_top).min(max_bottom - window_height);

    PhysicalPosition::new(desired_x as i32, desired_y as i32)
}

pub fn start_mouse_monitor(config: MouseMonitorConfig, app_handle: AppHandle) {
    let mut last_sampling = Instant::now();
    let sampling_interval = Duration::from_millis(50);
    let mut last_direction: i32 = 0;
    let mut shake_count = 0;

    thread::spawn(move || {
        #[cfg(target_os = "macos")]
        set_is_main_thread(false);
        let mut current_mouse_position = PhysicalPosition::default();
        let mut start_mouse_position = PhysicalPosition::default();
        let mut last_shake_time = Instant::now();
        let mut is_already_current_shown = app_handle.is_send_window_open();
        // During the drag gesture, if the user shake the second time, we will ignored.
        let mut is_handled_shown = false;
        let _ = rdev::listen(move |event| {
            match event.event_type {
                EventType::ButtonPress(Button::Left) => {
                    USER_DID_DROP.store(false, Ordering::SeqCst);
                    is_handled_shown = false;
                    start_mouse_position = current_mouse_position.clone();
                    is_already_current_shown = app_handle.is_send_window_open();
                    if is_already_current_shown || is_handled_shown {
                        if let (Some(monitor), Some(send_monitor)) = (get_monitor_at_position(&current_mouse_position, &app_handle), app_handle.get_webview_window("send").and_then(|it| it.current_monitor().ok().flatten())) {
                            if monitor.position() != send_monitor.position() {
                                is_already_current_shown = false;
                                is_handled_shown = false;
                            }
                        }
                    }

                    drag_start_gesture();
                }
                EventType::ButtonRelease(Button::Left) => {
                    sleep(Duration::from_millis(400));
                    let is_dropped =  USER_DID_DROP.load(Ordering::SeqCst);
                    if !is_dropped && app_handle.is_send_window_open() && !is_already_current_shown {
                        let _ = app_handle.hide_send();
                        return;
                    }

                    drag_end_gesture();
                    shake_count = 0;
                    last_direction = 0;
                }
                EventType::MouseMove { x, y } => {
                    if is_handled_shown {
                        return;
                    }

                    if is_already_current_shown {
                        return;
                    }

                    if last_sampling.elapsed() < sampling_interval {
                        return;
                    }

                    last_sampling = Instant::now();

                    let previous_mouse = current_mouse_position.clone();
                    current_mouse_position.x = x;
                    current_mouse_position.y = y;

                    #[cfg(target_os = "macos")]
                    {
                        let scale_factor = app_handle
                            .primary_monitor()
                            .ok()
                            .flatten()
                            .map(|m| m.scale_factor())
                            .unwrap_or(1.0);

                        current_mouse_position.x = x * scale_factor;
                        current_mouse_position.y = y * scale_factor;
                    }

                    if detect_drag(&start_mouse_position, &current_mouse_position) {
                        let dx = (current_mouse_position.x - previous_mouse.x);
                        if dx.abs() < config.min_changed {
                            return;
                        }

                        let direction = dx.signum() as i32;
                        if direction == 0 {
                            return;
                        }

                        if direction != last_direction {
                            last_direction = direction;
                            shake_count += 1;
                            last_shake_time = Instant::now();
                        }

                        if shake_count >= config.required_shakes {
                            log::info!("Shaking detected, showing send window");
                            let win = app_handle.show_send();
                            // Temporary hide it to avoid flickering
                            let _ = win.hide();

                            if let Ok(window_size) = win.outer_size() {
                                let window_physical_size: PhysicalSize<u32> = window_size.into();

                                if let Some(monitor) = get_monitor_at_position(&start_mouse_position, &app_handle) {
                                    let final_pos = calculate_window_position(
                                        &start_mouse_position,
                                        &window_physical_size,
                                        &monitor,
                                    );

                                    let _ = win.set_position(final_pos);
                                }
                                else {
                                    log::warn!("Could not find monitor at position, using logical fallback");
                                    let _ = win.set_position(start_mouse_position.clone());
                                }
                            }

                            let _ = win.show();
                            let _ = win.set_focus();
                            is_handled_shown = true;

                            shake_count = 0;
                        }

                        if last_shake_time.elapsed() > config.shake_timeout {
                            last_shake_time = Instant::now();
                            shake_count = 0;
                        }
                    }
                }
                _ => {}
            }
        });
    });
}
