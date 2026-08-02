# Tasks: mac-app-store-safe-permissions

## Progress: [5/14]

## 1. Distribution Boundary

- [x] 1.1 Add the `mac-app-store` Cargo feature and reject unsupported target combinations.
- [x] 1.2 Extract a pure startup policy that describes allowed permission and monitor behavior for each build.
- [x] 1.3 Compile permission prompting and System Settings launches out of the Mac App Store build.
- [x] 1.4 Compile global mouse/input and drag-pasteboard monitor startup out of the Mac App Store build.
- [x] 1.5 Verify ordinary Tauri window drag/drop and file-picker paths do not depend on the removed monitor startup.

## 2. Build and Artifact Enforcement

- [ ] 2.1 Enable `mac-app-store` only in the `build-macos-appstore` workflow invocation.
- [ ] 2.2 Add a verifiable App Store distribution marker to the packaged artifact.
- [ ] 2.3 Audit App Store Info.plist, configuration, and entitlements for permission descriptions or privileged capabilities.
- [ ] 2.4 Add CI assertions for the feature marker, prohibited usage descriptions, and prohibited entitlements.

## 3. Verification

- [ ] 3.1 Add unit tests for direct-download and App Store startup policies.
- [ ] 3.2 Compile/check both the default and `mac-app-store` feature configurations.
- [ ] 3.3 Launch the signed App Store artifact on a clean macOS account and record that no permission or System Settings UI appears.
- [ ] 3.4 Verify in-window drag/drop and file selection still complete a transfer setup without privileged access.

## 4. Documentation

- [ ] 4.1 Update `desktop/SIGNING.md` and the App Review checklist with the distribution behavior and captured evidence.

---

## Notes

Do not replace the removed global monitor with a different system-wide API in this change.

Task 1.5 found no existing user-facing system file picker in the desktop source. Existing selection is Tauri window drag/drop plus its drag-pasteboard fallback; both are independent of global monitor startup.
