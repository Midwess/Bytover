use std::sync::atomic::{AtomicBool, Ordering};

use crate::extensions::AppHandleExt;
#[cfg(target_os = "macos")]
use rdev::{set_is_main_thread, Button, EventType, Key};
#[cfg(not(target_os = "macos"))]
use rdev::{Button, EventType, Key};
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, PhysicalPosition};

/// Check if the app has accessibility permission on macOS.
/// If `prompt` is true, it will show the system dialog asking user to grant permission.
/// Returns true if permission is granted, false otherwise.
#[cfg(target_os = "macos")]
pub fn check_accessibility_permission(prompt: bool) -> bool {
    use std::ptr;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFDictionaryCreate(
            allocator: *const std::ffi::c_void,
            keys: *const *const std::ffi::c_void,
            values: *const *const std::ffi::c_void,
            num_values: isize,
            key_callbacks: *const std::ffi::c_void,
            value_callbacks: *const std::ffi::c_void,
        ) -> *const std::ffi::c_void;
        fn CFRelease(cf: *const std::ffi::c_void);

        static kCFBooleanTrue: *const std::ffi::c_void;
        static kCFBooleanFalse: *const std::ffi::c_void;
        static kCFTypeDictionaryKeyCallBacks: *const std::ffi::c_void;
        static kCFTypeDictionaryValueCallBacks: *const std::ffi::c_void;
    }

    // kAXTrustedCheckOptionPrompt key
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
    }

    unsafe {
        let prompt_value = if prompt { kCFBooleanTrue } else { kCFBooleanFalse };

        let keys: [*const std::ffi::c_void; 1] = [kAXTrustedCheckOptionPrompt];
        let values: [*const std::ffi::c_void; 1] = [prompt_value];

        let options = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            kCFTypeDictionaryKeyCallBacks,
            kCFTypeDictionaryValueCallBacks,
        );

        let is_trusted = AXIsProcessTrustedWithOptions(options);

        if !options.is_null() {
            CFRelease(options);
        }

        is_trusted
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission(_prompt: bool) -> bool {
    true // No accessibility permission needed on other platforms
}

/// Check if the app has Input Monitoring permission on macOS.
/// This is required for CGEventTap (used by rdev) on macOS 10.15+.
/// If `prompt` is true, it will request access and show the system dialog.
/// Returns true if permission is granted, false otherwise.
#[cfg(target_os = "macos")]
pub fn check_input_monitoring_permission(prompt: bool) -> bool {
    #[repr(u32)]
    #[allow(dead_code)]
    enum IOHIDRequestType {
        ListenEvent = 1,
        PostEvent = 2,
    }

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOHIDCheckAccess(request_type: u32) -> u32;
        fn IOHIDRequestAccess(request_type: u32) -> bool;
    }

    // IOHIDAccessType values from IOKit
    const KIOHID_ACCESS_TYPE_GRANTED: u32 = 0;
    // const KIOHID_ACCESS_TYPE_DENIED: u32 = 1;
    // const KIOHID_ACCESS_TYPE_UNKNOWN: u32 = 2;

    unsafe {
        let access_status = IOHIDCheckAccess(IOHIDRequestType::ListenEvent as u32);

        if access_status == KIOHID_ACCESS_TYPE_GRANTED {
            return true;
        }

        if prompt {
            // This will show the system dialog if permission hasn't been requested before
            IOHIDRequestAccess(IOHIDRequestType::ListenEvent as u32);
            // Check again after requesting
            IOHIDCheckAccess(IOHIDRequestType::ListenEvent as u32) == KIOHID_ACCESS_TYPE_GRANTED
        } else {
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_input_monitoring_permission(_prompt: bool) -> bool {
    true // No input monitoring permission needed on other platforms
}

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

pub fn detect_drag(_start: &PhysicalPosition<f64>, _current: &PhysicalPosition<f64>) -> bool {
    if !DRAG_ACTIVE.load(Ordering::SeqCst) {
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        MACOS_DRAG_HAS_ITEMS.load(Ordering::SeqCst)
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, distinguishing between "mouse down move" (selection)
        // and "actual item drag" is done by checking for the ghost drag image window.
        // This window class is standard for File Explorer and OLE drags.
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

        let is_dragging = unsafe {
            let h1 = FindWindowW(w!("SysDragImage"), None);
            let h2 = FindWindowW(w!("DragImage"), None);

            let s1 = h1.map(|h| !h.0.is_null()).unwrap_or_else(|e| {
                log::error!("FindWindowW(SysDragImage) failed: {:?}", e);
                false
            });
            let s2 = h2.map(|h| !h.0.is_null()).unwrap_or_else(|e| {
                log::error!("FindWindowW(DragImage) failed: {:?}", e);
                false
            });

            s1 || s2
        };

        if !is_dragging {
            return false;
        }

        const THRESHOLD: f64 = 10f64;
        let dx = (_current.x - _start.x).abs();
        let dy = (_current.y - _start.y).abs();
        dx > THRESHOLD || dy > THRESHOLD
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // For other platforms, we use the distance threshold as a fallback
        const THRESHOLD: f64 = 10f64;
        let dx = (_current.x - _start.x).abs();
        let dy = (_current.y - _start.y).abs();
        dx > THRESHOLD || dy > THRESHOLD
    }
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

            let is_dragging = is_active
                && queue.exec_sync(move || {
                    let pb = Pasteboard::named(PasteboardName::Drag);
                    pb.get_file_urls().map(|it| !it.is_empty()).unwrap_or_default()
                });

            if is_dragging != current_dragging {
                current_dragging = is_dragging;
                MACOS_DRAG_HAS_ITEMS.store(current_dragging, Ordering::SeqCst);
            }

            sleep(Duration::from_millis(250));
        }
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
        #[cfg(target_os = "windows")]
        let min_changed = 30.0;
        #[cfg(not(target_os = "windows"))]
        let min_changed = 40.0;

        Self {
            required_shakes: 2,
            min_changed,
            shake_timeout: Duration::from_millis(2000),
        }
    }
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
        // Track the shelf window opened during current drag gesture
        let mut opened_shelf_label: Option<String> = None;

        // During the drag gesture, if the user shake the second time, we will ignored.
        let mut is_handled_shown = false;
        let mut shift_pressed = false;
        let _ = rdev::listen(move |event| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match event.event_type {
                    EventType::ButtonPress(Button::Left) => {
                        USER_DID_DROP.store(false, Ordering::SeqCst);

                        is_handled_shown = false;
                        opened_shelf_label = None;
                        start_mouse_position = current_mouse_position.clone();
                        drag_start_gesture();
                    }
                    EventType::ButtonRelease(Button::Left) => {
                        sleep(Duration::from_millis(400));
                        let is_dropped = USER_DID_DROP.load(Ordering::SeqCst);
                        if !is_dropped {
                            if let Some(label) = opened_shelf_label.take() {
                                if let Some(window) = app_handle.get_webview_window(&label) {
                                    let _ = window.close();
                                }
                            }
                        }
                        opened_shelf_label = None;

                        drag_end_gesture();
                        shake_count = 0;
                        last_direction = 0;
                    }
                    EventType::MouseMove { x, y } => {
                        if is_handled_shown {
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
                            let scale_factor = app_handle.primary_monitor().ok().flatten().map(|m| m.scale_factor()).unwrap_or(1.0);

                            current_mouse_position.x = x * scale_factor;
                            current_mouse_position.y = y * scale_factor;
                        }

                        if detect_drag(&start_mouse_position, &current_mouse_position) {
                            if shift_pressed && !is_handled_shown {
                                log::info!("Shift+drag detected, creating new shelf window");
                                let start_pos = start_mouse_position.clone();
                                let win = app_handle.open_new_shelf_gated(Some(start_pos));
                                is_handled_shown = true;

                                let label = win.label().to_string();
                                if !label.starts_with("fake-shelf") {
                                    opened_shelf_label = Some(label);
                                }
                                let _ = win.set_focus();
                                return;
                            }

                            let dx = current_mouse_position.x - previous_mouse.x;
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
                                log::info!("Shaking detected, creating new shelf window");
                                let start_pos = start_mouse_position.clone();
                                let win = app_handle.open_new_shelf_gated(Some(start_pos));
                                is_handled_shown = true;

                                let label = win.label().to_string();
                                if !label.starts_with("fake-shelf") {
                                    opened_shelf_label = Some(label);
                                }
                                let _ = win.set_focus();

                                shake_count = 0;
                            }

                            if last_shake_time.elapsed() > config.shake_timeout {
                                last_shake_time = Instant::now();
                                shake_count = 0;
                            }
                        }
                    }
                    EventType::KeyPress(key) => {
                        if matches!(key, Key::ShiftLeft | Key::ShiftRight) {
                            shift_pressed = true;
                        }
                    }
                    EventType::KeyRelease(key) => {
                        if matches!(key, Key::ShiftLeft | Key::ShiftRight) {
                            shift_pressed = false;
                        }
                    }
                    _ => {}
                }
            }));

            if let Err(e) = result {
                log::error!("Mouse tracking event handler panicked: {:?}", e);
            }
        });
    });
}
