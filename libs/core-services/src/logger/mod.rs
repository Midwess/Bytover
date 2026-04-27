use futures_util::lock::Mutex;
use log::info;
use n0_future::task::{spawn, JoinHandle};
use n0_future::time;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub fn setup() {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Reflect;
        use log::{Level, LevelFilter, Log, Metadata, Record};
        use wasm_bindgen::JsValue;

        fn log_flag_enabled() -> bool {
            let global = js_sys::global();
            match Reflect::get(&global, &JsValue::from_str("__midwess_log")) {
                Ok(v) if v.is_undefined() => cfg!(debug_assertions),
                Ok(v) => v.is_truthy(),
                Err(_) => cfg!(debug_assertions),
            }
        }

        struct WasmLogger;

        impl Log for WasmLogger {
            fn enabled(&self, metadata: &Metadata) -> bool {
                if !log_flag_enabled() {
                    return false;
                }
                metadata.level() <= Level::Info
            }

            fn log(&self, record: &Record) {
                if self.enabled(record.metadata()) {
                    let now = js_sys::Date::new_0().to_iso_string();
                    web_sys::console::log_1(&format!("[{}] {} - {}", now, record.level(), record.args()).into());
                }
            }

            fn flush(&self) {}
        }

        static LOGGER: WasmLogger = WasmLogger;
        log::set_logger(&LOGGER).expect("failed to set logger");
        log::set_max_level(LevelFilter::Info);

        console_error_panic_hook::set_once();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use pretty_env_logger::formatted_timed_builder;
        let mut log_builder = formatted_timed_builder();
        if let Ok(filter_env) = std::env::var("RUST_LOG") {
            log_builder.parse_filters(&filter_env);
        } else {
            log_builder.filter(None, log::LevelFilter::Info);
        }
        // Suppress noisy third-party log targets
        log_builder.filter_module("rig::completions", log::LevelFilter::Off);
        log_builder.filter_module("rig::agent_chat", log::LevelFilter::Off);
        log_builder.filter_module("sqlx::query", log::LevelFilter::Off);
        // Enable raw LLM response logging for debugging
        log_builder.filter_module("llm::raw", log::LevelFilter::Debug);

        let _ = log_builder.try_init();
    }
}

pub struct ThrottleLogger {
    messages: Arc<Mutex<HashMap<String, usize>>>,
    join_handle: JoinHandle<()>
}

impl ThrottleLogger {
    pub fn new(namespace: String, delay: Duration) -> Self {
        let messages = Arc::new(Mutex::new(HashMap::new()));
        let messages_clone = messages.clone();
        let ns = namespace.clone();

        let join_handle = spawn(async move {
            let mut interval = time::interval(delay);
            interval.tick().await;

            loop {
                interval.tick().await;

                let mut msgs = messages_clone.lock().await;
                for (content, count) in msgs.drain() {
                    info!(
                        target: ns.as_str(),
                        "{content} happens {count} times in the last {delay:?}"
                    );
                }
            }
        });

        Self { messages, join_handle }
    }

    pub async fn log(&self, content: String) {
        let mut msgs = self.messages.lock().await;
        *msgs.entry(content).or_insert(0) += 1;
    }
}

impl Drop for ThrottleLogger {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}
