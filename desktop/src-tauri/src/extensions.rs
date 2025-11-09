use tauri::{Manager, Monitor, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri::utils::config::LogicalPosition;

pub trait AppHandleExt<R: Runtime> {
    fn close_all_windows(&self, whitelist: Vec<&str>);
    fn show_auth(&self);
    fn hide_auth(&self);
    fn show_send(&self) -> WebviewWindow<R>;
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

    fn show_auth(&self) {
        self.close_all_windows(vec!["auth"]);
        let auth = self.get_webview_window("auth").expect("auth window not found");
        let _ = auth.show();
    }

    fn hide_auth(&self) {
        if let Some(window) = self.get_webview_window("auth") {
            let _ = window.hide();
        }
    }

    fn is_send_window_open(&self) -> bool {
        self.get_webview_window("send").is_some()
    }

    fn hide_send(&self) {
        if let Some(window) = self.get_webview_window("send") {
            let _ = window.close();
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
                    .inner_size(260.0, 280.0)
                    .resizable(false)
                    .decorations(false)
                    .transparent(true)
                    .always_on_top(true)
                    .skip_taskbar(false)
                    .shadow(false)
                    .devtools(true)
                    .build()
                    .expect("failed to create send window")
            }
        };

        let _ = window.show();
        window
    }
}