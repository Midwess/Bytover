use tauri::{Emitter, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri::webview::Color;
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri_plugin_positioner::{Position, WindowExt};

pub trait AppHandleExt<R: Runtime> {
    fn close_all_windows(&self, whitelist: Vec<&str>);
    fn show_auth(&self) -> WebviewWindow<R>;
    fn create_receive(&self) -> WebviewWindow<R>;
    fn show_send(&self) -> WebviewWindow<R>;
    fn hide_auth(&self);
    fn toggle_receive(&self);
    fn is_send_window_open(&self) -> bool;
    fn hide_send(&self);
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
        self.close_all_windows(vec!["auth"]);

        let window = match self.get_webview_window("auth") {
            Some(window) => window,
            None => {
                WebviewWindowBuilder::new(
                    self,
                    "auth",
                    WebviewUrl::App("auth.html".into())
                )
                    .title("auth")
                    .inner_size(270.0, 420.0)
                    .decorations(false)
                    .transparent(true)
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
                    .skip_taskbar(false)
                    .resizable(false)
                    .shadow(true)
                    .devtools(true)
                    .build()
                    .expect("failed to create auth window");

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
                    "send", // window label
                    WebviewUrl::App("send.html".into())
                )
                    .title("send")
                    .inner_size(250.0, 260.0)
                    .resizable(false)
                    .decorations(false)
                    .transparent(true)
                    .visible_on_all_workspaces(true)
                    .always_on_top(true)
                    .skip_taskbar(false)
                    .shadow(false)
                    .devtools(true)
                    .build()
                    .expect("failed to create send window")
            }
        };

        let _ = window.show();
        let _ = window.emit("window-shown", {});
        window
    }

    fn hide_auth(&self) {
        if let Some(window) = self.get_webview_window("auth") {
            let _ = window.hide();
        }
    }

    fn is_send_window_open(&self) -> bool {
        self.get_webview_window("send").map(|it| it.is_visible().unwrap_or_default()).unwrap_or_default()
    }

    fn hide_send(&self) {
        if let Some(window) = self.get_webview_window("send") {
            let _ = window.hide();
        }
    }
}