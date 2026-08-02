# Proposal: Mac App Store Safe Permissions

**Status**: approved

## Summary

Introduce an explicit Mac App Store distribution boundary that prevents launch-time Accessibility and Input Monitoring requests and disables global input/mouse monitoring in that build while preserving ordinary in-app drag and drop and the existing direct-download behavior.

## Motivation

App Review rejected version 1.0.3 under Guideline 2.4.5(v) because the app requests administrator-like access during launch. The current startup path calls both permission APIs with prompting enabled, opens System Settings when permission is absent, and starts global monitoring regardless of distribution channel.

## Scope

### In Scope

- Add a compile-time Mac App Store distribution feature used by code and CI.
- Skip Accessibility and Input Monitoring prompts in the Mac App Store build.
- Skip global input/mouse monitoring and permission-related System Settings launches in that build.
- Preserve ordinary in-window drag/drop and non-privileged file selection.
- Preserve current monitoring behavior for direct-download builds.
- Add CI/static checks and clean-profile manual verification.

### Out of Scope

- Requesting special approval or privileged entitlements from Apple.
- Replacing the removed global monitoring behavior with another system-wide capture mechanism.
- Removing the feature from non-App-Store distributions.
- Broad refactoring of shelf or transfer behavior unrelated to monitoring.

## Affected Areas

| Area | Impact |
|---|---|
| `desktop/src-tauri/Cargo.toml` | Add an explicit App Store distribution feature |
| `desktop/src-tauri/src/lib.rs` | Gate startup permission requests and monitor startup |
| `desktop/src-tauri/src/mouse_tracking.rs` | Expose safe, testable distribution behavior |
| `.github/workflows/desktop-build.yml` | Compile the App Store artifact with the feature and verify it |
| `desktop/SIGNING.md` | Document feature selection and smoke tests |

## Dependencies

- Existing distinct `macos-appstore` GitHub Actions matrix entry
- Existing `tauri.conf.appstore.json` and sandbox entitlements
- A clean macOS test account or VM for approval-path verification

## Risks

| Risk | Mitigation |
|---|---|
| App Store CI accidentally omits the distribution feature | Add a build-time marker and CI assertion before packaging |
| Direct-download behavior regresses | Compile and test both feature configurations |
| Disabled monitoring breaks core drag/drop | Test ordinary Tauri window drag/drop independently from global monitoring |
| Another startup path still opens privacy settings | Add repository checks and clean-profile launch testing |
