#[cfg(all(target_os = "macos", feature = "mac-app-store"))]
mod macos;

#[cfg(all(target_os = "macos", feature = "mac-app-store"))]
pub(crate) use macos::authenticate;

#[cfg(test)]
mod tests {
    #[test]
    fn app_store_authentication_uses_one_native_session_owner() {
        let native = include_str!("macos.rs");

        assert!(native.contains("ASWebAuthenticationSession"));
        assert!(native.contains("setPresentationContextProvider"));
        assert!(native.contains("ACTIVE_SESSION"));
        assert!(native.contains("session_already_active"));
    }

    #[test]
    fn authentication_ui_offers_both_providers_without_manual_token_fallback() {
        let ui = include_str!("../../../src/auth/window.tsx");

        assert!(ui.contains("Sign in with Apple"));
        assert!(ui.contains("Sign in with Google"));
        assert!(!ui.contains("Paste access token"));
        assert!(!ui.contains("browser didn&apos;t open"));
    }
}
