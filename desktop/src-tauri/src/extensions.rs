use tauri::{Emitter, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri::webview::Color;
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri_plugin_positioner::{Position, WindowExt};

fn constrain_window_to_screen<R: Runtime>(window: &WebviewWindow<R>) {
    let Ok(Some(monitor)) = window.current_monitor() else { return; };
    let Ok(window_position) = window.outer_position() else { return; };
    let Ok(window_size) = window.outer_size() else { return; };

    let screen_size = monitor.size();
    let screen_position = monitor.position();
    let scale = monitor.scale_factor();

    let screen_width = screen_size.width as f64 / scale;
    let screen_height = screen_size.height as f64 / scale;
    let screen_x = screen_position.x as f64;
    let screen_y = screen_position.y as f64;

    let win_width = window_size.width as f64;
    let win_height = window_size.height as f64;
    let win_x = window_position.x as f64;
    let win_y = window_position.y as f64;

    let mut new_x = win_x;
    let mut new_y = win_y;

    if win_x < screen_x {
        new_x = screen_x;
    } else if win_x + win_width > screen_x + screen_width {
        new_x = screen_x + screen_width - win_width;
    }

    if win_y < screen_y {
        new_y = screen_y;
    } else if win_y + win_height > screen_y + screen_height {
        new_y = screen_y + screen_height - win_height;
    }

    if new_x != win_x || new_y != win_y {
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: new_x as i32,
            y: new_y as i32,
        }));
    }
}

pub trait AppHandleExt<R: Runtime> {
    fn close_all_windows(&self, whitelist: Vec<&str>);
    fn show_auth(&self) -> WebviewWindow<R>;
    fn create_receive(&self) -> WebviewWindow<R>;
    fn show_send(&self) -> WebviewWindow<R>;
    fn show_shelf(&self, shelf_id: u64) -> WebviewWindow<R>;
    fn open_new_shelf_window(&self) -> WebviewWindow<R>;
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
    fn hide_all_shelves(&self);
    fn show_toast(&self, message: &str) -> WebviewWindow<R>;
}

fn animate_window<R: Runtime>(window: WebviewWindow<R>) {
    let _ = window.show();
    let _ = window.set_focus();
}

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
            None => {
                WebviewWindowBuilder::new(
                    self,
                    "auth",
                    WebviewUrl::App("auth.html".into())
                )
                    .title("Bytover")
                    .inner_size(600.0, 600.0)
                    .decorations(true)
                    .transparent(true)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true)
                    .focused(true)
                    .skip_taskbar(false)
                    .resizable(false)
                    .shadow(true)
                    .devtools(true)
                    .build()
                    .expect("failed to create auth window")
            }
        };

        let _ = window.show();
        window
    }

    fn create_receive(&self) -> WebviewWindow<R> {
        let window = match self.get_webview_window("receive") {
            Some(window) => window,
            None => {
                let window = WebviewWindowBuilder::new(
                    self,
                    "receive",
                    WebviewUrl::App("receive.html".into())
                )
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
                        .build()
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
            }
            else {
                let _ = window.hide();
            }
        }
        else {
            let window = self.create_receive();
            let _ = window.show();
            let _ = window.move_window(Position::TrayBottomCenter);
        }
    }

    fn show_send(&self) -> WebviewWindow<R> {
        self.close_all_windows(vec!["send"]);
        let window = match self.get_webview_window("send") {
            Some(window) => window,
            None => {
                WebviewWindowBuilder::new(
                    self,
                    "send",
                    WebviewUrl::App("send.html".into())
                )
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
                    .expect("failed to create send window")
            }
        };

        animate_window(window.clone());
        let _ = window.emit("window-shown", {});
        window
    }

    fn show_shelf(&self, shelf_id: u64) -> WebviewWindow<R> {
        let label = format!("send-{}", shelf_id);
        let window = match self.get_webview_window(&label) {
            Some(window) => window,
            None => {
                WebviewWindowBuilder::new(
                    self,
                    &label,
                    WebviewUrl::App("send.html".into())
                )
                    .title(&label)
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
                    .expect("failed to create shelf window")
            }
        };

        if let Some(monitor) = window.current_monitor().ok().flatten() {
            let screen_size = monitor.size();
            let scale = monitor.scale_factor();

            const WIN_WIDTH: f64 = 245.0;
            const WIN_HEIGHT: f64 = 270.0;
            let max_offset_x = WIN_WIDTH * 1.5;
            let max_offset_y = WIN_HEIGHT * 1.5;

            let hash_x = ((shelf_id.wrapping_mul(2654435761)) & 0xFFFF) as f64 / 65535.0;
            let hash_y = ((shelf_id.wrapping_mul(2654435761).wrapping_shr(16)) & 0xFFFF) as f64 / 65535.0;

            let offset_x = (hash_x * 2.0 - 1.0) * max_offset_x;
            let offset_y = (hash_y * 2.0 - 1.0) * max_offset_y;

            let center_x = (screen_size.width as f64 / scale - WIN_WIDTH) / 2.0;
            let center_y = (screen_size.height as f64 / scale - WIN_HEIGHT) / 2.0;

            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: center_x + offset_x,
                y: center_y + offset_y,
            }));
        }

        constrain_window_to_screen(&window);
        animate_window(window.clone());
        window
    }

    fn open_new_shelf_window(&self) -> WebviewWindow<R> {
        let shelf_id = shared::gen_shelf_id();
        self.show_shelf(shelf_id)
    }

    fn show_settings_with_tab(&self, tab: &str) -> WebviewWindow<R> {
        let window = match self.get_webview_window("settings") {
            Some(window) => window,
            None => {
                let window = WebviewWindowBuilder::new(
                    self,
                    "settings",
                    WebviewUrl::App(format!("settings.html?tab={}", tab).into())
                )
                    .title("Settings")
                    .inner_size(560.0, 373.0)
                    .decorations(true)
                    .transparent(true)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true)
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
                        .build()
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
                let window = WebviewWindowBuilder::new(
                    self,
                    "settings",
                    WebviewUrl::App("settings.html".into())
                )
                    .title("Settings")
                    .inner_size(560.0, 373.0)
                    .decorations(true)
                    .transparent(true)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true)
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
                        .build()
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
            let _ = window.close();
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
                let window = WebviewWindowBuilder::new(
                    self,
                    "intro",
                    WebviewUrl::App("intro.html".into())
                )
                    .title("Welcome to Bytover")
                    .inner_size(690.0, 690.0)
                    .decorations(true)
                    .transparent(true)
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true)
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
        self.get_webview_window(&format!("send-{id}")).map(|it| it.is_visible().unwrap_or_default()).unwrap_or_default()
    }

    fn is_any_shelf_window_open(&self) -> bool {
        self.webview_windows()
            .iter()
            .any(|(label, window)| {
                label.starts_with("send-") && window.is_visible().unwrap_or_default()
            })
    }

    fn get_visible_shelf_windows(&self) -> Vec<WebviewWindow<R>> {
        self.webview_windows()
            .into_iter()
            .filter(|(label, window)| {
                label.starts_with("send-") && window.is_visible().unwrap_or_default()
            })
            .map(|(_, window)| window)
            .collect()
    }

    fn hide_send(&self) {
        if let Some(window) = self.get_webview_window("send") {
            let _ = window.hide();
        }
    }

    fn hide_all_shelves(&self) {
        for (label, window) in self.webview_windows() {
            if label.starts_with("send-") {
                let _ = window.hide();
            }
        }
    }

    fn show_toast(&self, message: &str) -> WebviewWindow<R> {
        let window = match self.get_webview_window("toast") {
            Some(window) => window,
            None => {
                let window = WebviewWindowBuilder::new(
                    self,
                    "toast",
                    WebviewUrl::App("toast.html".into())
                )
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
                        .build()
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
                screen_position.y + y as i32
            ));
        }

        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("toast-message", message.to_string());

        window
    }
}