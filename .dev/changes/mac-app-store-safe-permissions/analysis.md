# Codebase Analysis: Mac App Store Safe Permissions

## Current Behavior

- `desktop/src-tauri/src/lib.rs` calls `check_accessibility_permission(true)` and `check_input_monitoring_permission(true)` during every startup.
- When access is absent, the same startup block invokes `open` for the Accessibility and Input Monitoring System Settings panes.
- Startup then calls `start_mouse_monitor` and the macOS drag pasteboard monitor without a Mac App Store distribution guard.
- `desktop/src-tauri/src/mouse_tracking.rs` implements process-wide event monitoring through `rdev`, Accessibility trust checks, and IOKit input access requests.

## Similar Features and Conventions

- `.github/workflows/desktop-build.yml` already separates `build-macos-dmg` and `build-macos-appstore` jobs.
- `desktop/src-tauri/tauri.conf.appstore.json`, `entitlements.appstore.plist`, and `Info.appstore.plist` demonstrate the existing pattern of App Store-only build overlays.
- `desktop/src-tauri/Cargo.toml` uses Cargo features for optional behavior (`log-file`), giving a natural compile-time distribution boundary.
- `desktop/SIGNING.md` already treats the App Store artifact as a separately verified product.

## Affected Files

| Path | Finding |
|---|---|
| `desktop/src-tauri/src/lib.rs` | Launch-time prompts, settings launches, and monitor startup are unconditional |
| `desktop/src-tauri/src/mouse_tracking.rs` | Permission and global-monitor APIs need a safe distribution policy boundary |
| `desktop/src-tauri/Cargo.toml` | No `mac-app-store` feature exists |
| `.github/workflows/desktop-build.yml` | App Store build passes only `log-file`; no assertion identifies the behavioral variant |
| `desktop/SIGNING.md` | Smoke tests cover signing and sandboxing but not clean-launch permission behavior |

## Architecture Notes

Distribution policy belongs at the Tauri shell/build boundary. Shared transfer and shelf modules should not need to know why system-wide monitoring is unavailable. The shell should expose capabilities or simply omit global-only behavior while leaving ordinary window events intact.

A compile-time feature is preferable to a mutable runtime environment switch because reviewers must receive a binary whose prohibited path cannot be enabled accidentally after packaging.

## Testing Conventions to Follow

- Extract a pure startup-policy function and unit-test direct versus App Store outcomes.
- Compile/check both the default and App Store feature sets.
- Add workflow assertions that the App Store job supplies the feature and the DMG job does not.
- Smoke-test a signed App Store artifact with Accessibility and Input Monitoring reset or on a clean macOS account.

## Risks and Dependencies

- `rdev` remains linked even when monitoring is disabled; code-level non-execution is the approval requirement, while dependency removal can be evaluated separately.
- Some cross-window drag affordances may rely on the global monitor and must degrade without blocking local drag/drop.
- A source-level feature is ineffective if CI can package the wrong feature set; artifact verification is required.

## OpenSpec Integration

Create an added `macos-permissions` domain covering non-privileged App Store launch, disabled global monitoring, preserved local interaction, and distribution-feature verification.

## Local Interaction Verification

Task 1.5 verified the existing local resource-selection paths after the global monitor was gated:

- `desktop/src/send/shelf.tsx` subscribes directly to Tauri `window.onDragDropEvent`; a drop with paths invokes `add_resources` without consulting the global mouse monitor.
- `desktop/src-tauri/src/lib.rs::add_resources` converts the Tauri-provided paths directly into `ResourceSelection` values and dispatches `ShelfEvent::AddResources`.
- The HTML drag/drop fallback invokes `add_resources_from_drag_pasteboard`, which reads the current drag pasteboard directly and does not require `start_mouse_monitor` or `start_macos_drag_pasteboard_monitor` to be running.
- No user-facing system file-picker implementation exists in the current desktop source, so there is no existing picker path coupled to the removed monitoring infrastructure. This proposal preserves the existing Tauri drag/drop selection behavior and does not add a new picker.
