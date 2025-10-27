use tauri::{Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

pub trait AppHandleExt<R: Runtime> {
    fn close_all_windows(&self, whitelist: Vec<&str>);
    fn show_auth(&self);
    fn show_send(&self);
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
        let auth = self.get_webview_window("auth").expect("auth window not found");
        let _ = auth.show();
    }

    fn show_send(&self) {
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
                    .inner_size(480.0, 280.0)
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
    }
}