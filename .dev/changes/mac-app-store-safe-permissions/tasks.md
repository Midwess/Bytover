# Tasks: mac-app-store-safe-permissions

## Progress: [12/14]

## 1. Distribution Boundary

- [x] 1.1 Add the `mac-app-store` Cargo feature and reject unsupported target combinations.
- [x] 1.2 Extract a pure startup policy that describes allowed permission and monitor behavior for each build.
- [x] 1.3 Compile permission prompting and System Settings launches out of the Mac App Store build.
- [x] 1.4 Compile global mouse/input and drag-pasteboard monitor startup out of the Mac App Store build.
- [x] 1.5 Verify ordinary Tauri window drag/drop and file-picker paths do not depend on the removed monitor startup.

## 2. Build and Artifact Enforcement

- [x] 2.1 Enable `mac-app-store` only in the `build-macos-appstore` workflow invocation.
- [x] 2.2 Add a verifiable App Store distribution marker to the packaged artifact.
- [x] 2.3 Audit App Store Info.plist, configuration, and entitlements for permission descriptions or privileged capabilities.
- [x] 2.4 Add CI assertions for the feature marker, prohibited usage descriptions, and prohibited entitlements.

## 3. Verification

- [x] 3.1 Add unit tests for direct-download and App Store startup policies.
- [x] 3.2 Compile/check both the default and `mac-app-store` feature configurations.
- [ ] 3.3 Launch the signed App Store artifact on a clean macOS account and record that no permission or System Settings UI appears.
- [ ] 3.4 Verify in-window drag/drop and file selection still complete a transfer setup without privileged access.

## 4. Documentation

- [x] 4.1 Update `desktop/SIGNING.md` and the App Review checklist with the distribution behavior and captured evidence.

---

## Notes

Do not replace the removed global monitor with a different system-wide API in this change.

Task 1.5 found no existing user-facing system file picker in the desktop source. Existing selection was Tauri window drag/drop plus its drag-pasteboard fallback; both are independent of global monitor startup. Task 3.4 subsequently added an explicit native picker through that same resource-selection path.

Task 2.4 verifies the compiled App Store marker, absence of permission/global-input symbols, the packaged distribution marker, absence of privileged usage descriptions, sandbox enablement, and absence of prohibited entitlements after signing.

Task 3.1 covers the explicit direct-download and App Store policies plus the policy and marker selected by each compiled feature configuration.

Task 3.2 passes `cargo check -p Bytover` with and without `--features mac-app-store`. Both configurations retain four pre-existing warnings and report no errors.

Task 3.3 cannot be completed on the feature branch: GitHub Actions run 30738512503 stopped before deployment because signing secrets are correctly restricted to the `production` branch, and this Mac has no App Store signing identity. The run used `upload_to_app_store=false`, so App Store Connect was not changed. Complete the signed clean-account test after merge to `production`; do not relax the environment restriction.

Task 3.4 now has an explicit native “Choose files…” action routed through the same `ShelfEvent::AddResources` path as Tauri window drops. Its App Store-feature unit test and the production web build pass. Final end-to-end verification remains paired with the signed clean-account test in Task 3.3.
