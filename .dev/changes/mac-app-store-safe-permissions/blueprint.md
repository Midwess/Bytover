# Blueprint: Mac App Store Safe Permissions

## Design Summary

Add a Cargo feature named `mac-app-store` and use conditional compilation so prohibited permission and global-monitor code is unreachable in the App Store binary. The existing `macos-appstore` workflow becomes the sole owner of that feature. Ordinary Tauri window drag/drop remains unchanged.

## Files to Modify or Review

- Modify `desktop/src-tauri/Cargo.toml` to declare `mac-app-store = []`.
- Modify `desktop/src-tauri/src/lib.rs` to compile out permission prompts, System Settings launches, and global monitor startup.
- Modify `desktop/src-tauri/src/mouse_tracking.rs` so privileged imports and implementations are absent where practical from the App Store feature build.
- Modify `.github/workflows/desktop-build.yml` to pass `--features log-file,mac-app-store` and assert the packaged variant.
- Modify `desktop/SIGNING.md` with behavior and clean-profile smoke tests.
- Review App Store plist/config/entitlements files and `desktop/src/send/shelf.tsx` for ordinary drag/drop preservation.

## Interface and Build Contract

- Cargo feature: `mac-app-store = []`.
- App Store-only code checks: `#[cfg(all(target_os = "macos", feature = "mac-app-store"))]` and its negation.
- App Store workflow must contain the feature; direct-download workflow must not.
- The packaged App Store artifact must expose a verifiable distribution marker and contain no Accessibility/Input Monitoring usage descriptions or privileged entitlements.
- Enabling `mac-app-store` for a non-macOS target should fail compilation or be explicitly rejected by build validation.

## Implementation Phases

1. Add the feature and isolate a testable startup policy.
2. Compile out privileged permission and monitor paths.
3. Update the App Store build and artifact assertions.
4. Verify default and App Store builds plus ordinary drag/drop.
5. Capture clean-profile evidence and update signing documentation.

## Testing Strategy

- Unit-test distribution startup policy.
- Compile/check default and `mac-app-store` feature variants.
- Assert workflow feature selection and artifact metadata.
- Launch the signed artifact with privacy permissions reset and verify no prompt/settings navigation.
- Verify drag into an existing Bytover window and file-picker selection still work.

## Risks and Mitigations

- **All drag/drop is accidentally disabled**: gate only global-monitor infrastructure and test the Tauri window event path.
- **Wrong binary is packaged**: make feature selection and artifact inspection fail closed in CI.
- **A second prompt path remains**: search and artifact-test every privacy-settings launch before submission.

