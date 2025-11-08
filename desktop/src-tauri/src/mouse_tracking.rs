use std::sync::atomic::{AtomicBool, Ordering};

use std::thread;
use std::time::{Duration, Instant};
use rdev::{set_is_main_thread, Button, EventType};
use tauri::{AppHandle, LogicalPosition};
use crate::extensions::AppHandleExt;

#[cfg(target_os = "macos")]
static MACOS_DRAG_HAS_ITEMS: AtomicBool = AtomicBool::new(false);

static DRAG_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn drag_start_gesture() {
    DRAG_ACTIVE.store(true, Ordering::SeqCst);
}

pub fn drag_end_gesture() {
    DRAG_ACTIVE.store(false, Ordering::SeqCst);
}

pub fn detect_drag(start: &LogicalPosition<f64>, current: &LogicalPosition<f64>) -> bool {
    if !DRAG_ACTIVE.load(Ordering::SeqCst) {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::Mouse::DragDetect;
        use windows::Win32::Foundation::{HWND, POINT};

        let pt = POINT { x: start.0, y: start.1 };
        return unsafe { DragDetect(HWND(0), pt).as_bool() };
    }

    #[cfg(target_os = "macos")]
    {
        return MACOS_DRAG_HAS_ITEMS.load(Ordering::SeqCst)
    }

    // Linux fallback + macOS fallback
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
            // If user is not pressing, then we reset the drag state
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

            thread::sleep(Duration::from_millis(250));
        };
    });
}

#[derive(Debug, Clone)]
pub struct MouseMonitorConfig {
    pub required_shakes: u32,
    pub shake_timeout: Duration,
    pub min_changed: f64,
    pub window_close_delay: u64,
}

impl Default for MouseMonitorConfig {
    fn default() -> Self {
        Self {
            required_shakes: 3,
            min_changed: 50f64,
            shake_timeout: Duration::from_millis(2000),
            window_close_delay: 1000,
        }
    }
}

pub fn start_mouse_monitor(config: MouseMonitorConfig, app_handle: AppHandle) {
    let mut last_sampling = Instant::now();
    let sampling_interval = Duration::from_millis(100);
    let mut last_direction: i32 = 0;
    let mut shake_count = 0;

    thread::spawn(move || {
        #[cfg(target_os = "macos")]
        set_is_main_thread(false);
        let mut current_mouse_position = LogicalPosition::default();
        let mut start_mouse_position = LogicalPosition::default();
        let mut last_shake_time = Instant::now();
        let _ = rdev::listen(move |event| {
            match event.event_type {
                EventType::ButtonPress(Button::Left) => {
                    start_mouse_position = current_mouse_position.clone();
                    drag_start_gesture();
                }

                EventType::ButtonRelease(Button::Left) => {
                    drag_end_gesture();
                    shake_count = 0;
                    last_direction = 0;
                }

                EventType::MouseMove { x, y } => {
                    if last_sampling.elapsed() < sampling_interval {
                        return;
                    }

                    last_sampling = Instant::now();

                    let previous_mouse = current_mouse_position.clone();
                    current_mouse_position.x = x;
                    current_mouse_position.y = y;

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
                            log::info!("Shaking detected");
                            let win = app_handle.show_send();
                            // TODO: Apply margin
                            let _ = win.set_position(current_mouse_position.clone());
                            let _ = win.set_focus();

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