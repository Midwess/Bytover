// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // GTK3 on native Wayland ignores set_keep_above, visible_on_all_workspaces,
    // and absolute window positioning — the shelf UI depends on all three, so
    // route GTK through XWayland whenever one is available. Must run before
    // tokio spawns worker threads, since env::set_var is racy once threads
    // start and is read during GTK init.
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("DISPLAY").is_some() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
        .block_on(async {
            desktop_lib::run().await;
        });
}
