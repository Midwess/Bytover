# macOS Code Signing

Bytover's desktop app has two signing paths: a **local** path for developers, and a **CI** path for distribution. Each produces a differently-signed artifact.

## Who consumes what

| Artifact | Produced by | Signing identity | Who can run it |
|---|---|---|---|
| Local ad-hoc `.app` | `pnpm tauri build` on your Mac | `-` (null) | You, on your own Mac |
| Local Apple-Dev `.app` | `pnpm tauri:build:dev` on your Mac | `Apple Development: <you>` | You, on your own Mac — required for cert-gated features like StoreKit / iCloud / Push |
| CI release `.app` / `.dmg` | GitHub Actions `desktop-release` workflow, `platform: macos-dmg` | `Developer ID Application: Midwess LLC` + notarized | Anyone on any Mac, including beta testers and public users |
| CI App Store `.app` / `.pkg` | GitHub Actions `desktop-release` workflow, `platform: macos-appstore` | `Apple Distribution: Midwess LLC` (inner `.app`) + `3rd Party Mac Developer Installer: Midwess LLC` (outer `.pkg`) | Mac App Store reviewers, then App Store users |

Apple Development signing is **local-only**. CI has no `development` path because an Apple-Dev-signed artifact can't be distributed to other machines without per-Mac manual approval — building it in CI would burn minutes for an artifact nobody can use.

## Identifier and team

- Bundle identifier: `com.midwess.bytover` (lowercase — matches the team provisioning profile)
- Team ID: `BUJKWCX7F4` (Midwess LLC)

## Local builds

### Ad-hoc (default, fastest)

```bash
cd desktop
pnpm tauri build          # same as pnpm tauri:build:adhoc
```

Uses `signingIdentity: "-"` from `src-tauri/tauri.conf.json`. Result: `target/release/bundle/macos/Bytover.app` with `flags=0x10002(adhoc,runtime)`. Launches on your Mac without warnings. Won't pass Gatekeeper on other Macs.

### Apple Development (for cert-gated feature work)

```bash
cd desktop
pnpm tauri:build:dev
```

Merges `src-tauri/tauri.conf.dev.json` on top → signs with your Apple Development cert. Required when testing StoreKit, iCloud containers, Push Notifications, App Groups, or anything keyed to the team's `TeamIdentifier`.

One-time keychain setup:

```bash
security import ~/Downloads/AppleDevelopment.p12 -k ~/Library/Keychains/login.keychain-db -P "<p12-password>" -T /usr/bin/codesign
security unlock-keychain -p "<login-password>" ~/Library/Keychains/login.keychain-db
security find-identity -v -p codesigning    # should list the dev cert
```

**macOS 26 / Tahoe note**: `entitlements.dev.plist` must not contain `com.apple.security.get-task-allow` when combined with `disable-library-validation` + `allow-unsigned-executable-memory` on a non-Developer-ID cert — AMFI rejects that combo at launch. If you need lldb-attach to a dev-signed build, run on macOS 15 Sonoma or earlier.

## CI build (production)

### Secrets (in GitHub environment `production`)

| Secret | Source |
|---|---|
| `APPLE_CERTIFICATE` | `base64 -i DeveloperID.p12` (no line wrapping). |
| `APPLE_CERTIFICATE_PASSWORD` | Password chosen when exporting the `.p12`. |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Midwess LLC (BUJKWCX7F4)` — match what `security find-identity -v -p codesigning` prints. |
| `APPLE_ID` | Apple ID email on the developer account. |
| `APPLE_PASSWORD` | **App-specific** password from appleid.apple.com (format `abcd-efgh-ijkl-mnop`). Not the Apple ID login password. |
| `APPLE_TEAM_ID` | `BUJKWCX7F4` |
| `TAURI_SIGNING_PRIVATE_KEY` | Minisign key for updater. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passphrase for minisign key. |

Configure a **deployment-branch restriction** on the `production` environment so only the `production` branch can access these secrets.

### Exporting the Developer ID cert as .p12

1. Keychain Access → `login` keychain → `My Certificates`.
2. Find `Developer ID Application: Midwess LLC (BUJKWCX7F4)`.
3. Right-click → Export → File Format `Personal Information Exchange (.p12)` → set password.
4. `base64 -i DeveloperID.p12 | pbcopy`.
5. Paste into the GitHub secret value.

### Triggering

Actions → `desktop-release` → Run workflow → pick `platform`:

- `macos-dmg` — Developer ID-signed `.dmg`, notarized (without stapling), attached to a draft GitHub release
- `macos-appstore` — Apple-Distribution-signed `.app` wrapped in a `3rd Party Mac Developer Installer`-signed `.pkg`, uploaded to App Store Connect (no GitHub release). **Opt-in only**; not part of `all`.
- `windows` / `linux` — unsigned, attached to a draft release
- `all` — `macos-dmg` + `windows` + `linux`. Excludes `macos-appstore` because App Store uploads consume App Store Connect version slots and are visible to Apple even when not promoted to review.

For the DMG and Windows/Linux paths:

- From the `production` branch → tagged `v__VERSION__`, release title `Bytover v__VERSION__`
- From any other branch → tagged `v__VERSION__-beta`, release title suffix `(Beta)`

Beta testers and release users get the same signature guarantees on the DMG path; only the version number and release naming differ. The App Store path produces no GitHub release artifact — the `.pkg` is exported as a workflow artifact (`Bytover-appstore-pkg`, 14-day retention) for smoke testing, and uploaded to App Store Connect Activity for review promotion.

## Verification

CI runs these automatically. To reproduce locally on any `.app`:

```bash
APP=target/universal-apple-darwin/release/bundle/macos/Bytover.app   # or target/release/... for local builds

codesign -dv --verbose=4 "$APP"                              # signature details
codesign -d --entitlements - --xml "$APP" | plutil -p -      # entitlements
codesign --verify --deep --strict --verbose=2 "$APP"         # integrity
spctl --assess --type exec --verbose=4 "$APP"                # Gatekeeper (production only)
xcrun stapler validate "$APP"                                # notarization ticket (production only)
```

A production-signed build shows `Authority=Developer ID Application: Midwess LLC` + `flags=0x10000(runtime)` (no `adhoc`), and both `spctl` + `stapler` succeed.

## Troubleshooting

### `errSecInternalComponent` during local sign
Usually: signing identity missing from keychain, keychain locked, or cert expired.

```bash
security find-identity -v -p codesigning
security unlock-keychain -p "<login-password>" ~/Library/Keychains/login.keychain-db
```

### "App is damaged and can't be opened" on another Mac
Quarantine attribute is set but the signature didn't validate. For a Developer-ID build this means notarization failed silently or the DMG was edited after signing. Check the `Verify macOS signing` step's CI log. Temporary workaround only:

```bash
xattr -cr /Applications/Bytover.app
```

### "Bytover.app was not opened because it contains malware"
macOS 26's generic rejection for: ad-hoc signed binary, Apple-Dev signed binary, or unsigned binary. A Developer ID + notarized + stapled build will not trigger this. If you see it on a supposed production build, the artifact is not actually production-signed — verify with `codesign -dv`.

### Notarization rejects `get-task-allow`
Developer ID builds must not carry `com.apple.security.get-task-allow`. That entitlement is only in `entitlements.dev.plist`. Confirm the prod path didn't accidentally load the dev overlay.

### `APPLE_PASSWORD` auth failure
Must be an **app-specific** password from appleid.apple.com → Sign-In and Security → App-Specific Passwords. The Apple ID login password will not work with `notarytool`.

### Certificate rotation

1. Generate a new `.p12` from Keychain Access.
2. Update `APPLE_CERTIFICATE` + `APPLE_CERTIFICATE_PASSWORD` in the `production` GitHub environment.
3. If the cert's common name changed, update `APPLE_SIGNING_IDENTITY` too.
4. Re-run the workflow — the verify step confirms the new identity.

## CI build (App Store)

The App Store path produces a sandboxed, Apple-Distribution-signed bundle wrapped in a `3rd Party Mac Developer Installer`-signed `.pkg`, then uploads it to App Store Connect via `xcrun altool --upload-app`.

### Prerequisites

- Apple Developer Identifier `com.midwess.bytover` has the **App Sandbox** capability enabled.
- App Store Connect app record exists for `com.midwess.bytover` with at least one draft version slot (Apple rejects uploads with no slot).
- The `bytover-ci-notarization` API key (already provisioned for notarization) has at least the `Developer` role in App Store Connect → Users and Access → Integrations.

### Files in the repo

| Path | Purpose |
|---|---|
| `desktop/src-tauri/tauri.conf.appstore.json` | Overlay applied via `--config`. Picks Apple Distribution identity, sandbox entitlements, the embedded provisioning profile, and forces `bundle.targets: ["app"]` (no `.dmg`). |
| `desktop/src-tauri/entitlements.appstore.plist` | Sandbox-compliant entitlements. Strict subset of the Developer ID set: enables `app-sandbox`, `network.client/server`, `files.user-selected.read-write`, `files.bookmarks.app-scope`. **Excludes** `cs.disable-library-validation` and `cs.allow-unsigned-executable-memory` (App Store rejects). |
| `desktop/src-tauri/Bytover_Production.provisionprofile` | Mac App Store distribution provisioning profile. Committed to the repo; carries no private key — only Team ID, App ID, expiration, and the public DER cert. |

### Secrets (in GitHub environment `production`)

In addition to the Developer ID secrets above:

| Secret | Source |
|---|---|
| `APPLE_DIST_CERTIFICATE` | `base64 -i AppleDistribution.p12 \| tr -d '\n'` |
| `APPLE_DIST_CERTIFICATE_PASSWORD` | Password from the `.p12` export. |
| `APPLE_DIST_INSTALLER_CERTIFICATE` | `base64 -i MacInstaller.p12 \| tr -d '\n'` |
| `APPLE_DIST_INSTALLER_CERTIFICATE_PASSWORD` | Password from the `.p12` export. |
| `APPLE_DIST_SIGNING_IDENTITY` | `Apple Distribution: Midwess LLC (BUJKWCX7F4)` — match `security find-identity -v -p codesigning`. |
| `APPLE_DIST_INSTALLER_IDENTITY` | `3rd Party Mac Developer Installer: Midwess LLC (BUJKWCX7F4)`. |

The App Store Connect API key (`APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, `APPLE_API_KEY_BASE64`) is reused from the Developer ID notarization path — the same `.p8` works for both `notarytool submit` and `xcrun altool --upload-app`.

### Provisioning profile rotation

The Mac App Store distribution profile is committed to the repo (`Bytover_Production.provisionprofile`) — its contents are public-information-equivalent without the matching `Apple Distribution` private key. To rotate:

1. Apple Developer portal → Profiles → regenerate the *Mac App Store Distribution* profile bound to the Apple Distribution cert + bundle ID `com.midwess.bytover`.
2. Download the new `.provisionprofile` and replace `desktop/src-tauri/Bytover_Production.provisionprofile`.
3. Open a PR — reviewers can `security cms -D -i` the new file to verify Team ID, App ID, and expiration match expectations.

The CI step `Install App Store provisioning profile` validates the file every run: hard-fails on team-id mismatch, app-id mismatch, or expiration; warns within 30 days of expiry.

### `ITMS-*` error reference

| Error | Likely cause | Fix |
|---|---|---|
| `ITMS-90296: App sandbox not enabled` | App ID lacks the App Sandbox capability, or `app-sandbox: true` was dropped from `entitlements.appstore.plist`. | Enable the capability in Apple Developer portal; re-issue the profile if needed. |
| `ITMS-91065: Missing signing certificate` | `.app` and `.pkg` signed with mismatched identities (e.g., Developer ID for the inner .app instead of Apple Distribution). | Verify `APPLE_DIST_SIGNING_IDENTITY` and `APPLE_DIST_INSTALLER_IDENTITY` are correct identity strings. |
| `ITMS-90478: Invalid Version` | `CFBundleVersion` already uploaded for this bundle. | Bump `desktop/package.json` version and re-run. |
| `ITMS-90438: Invalid Bundle / missing LSApplicationCategoryType` | `bundle.category` not set in `tauri.conf.json`. | Confirm `bundle.category: "public.app-category.productivity"` is present in the base config. |

### Sandbox compatibility expectations

The App Store sandbox restricts:

- File system access outside the app container — custom save locations work via the file picker (`files.user-selected.read-write` + `files.bookmarks.app-scope` for persistence) but arbitrary path writes silently fail.
- Library validation — third-party `.dylib`s and sidecar binaries must be Apple-signed or co-signed by us. The Developer ID build's `cs.disable-library-validation` workaround is unavailable.
- Privileged ports — `network.server` allows server sockets but not ports below 1024 without a temporary exception entitlement.
- `macOSPrivateApi: true` (currently set in `tauri.conf.json`) gives Tauri private window-styling APIs. Apple's review has historically been inconsistent on these — submission may be rejected for "uses non-public API" even if the build itself succeeds.

Smoke-test the artifact (`Bytover-appstore-pkg` workflow artifact) on a clean Mac before promoting to review:

```bash
sudo installer -pkg Bytover.pkg -target /
open /Applications/Bytover.app
log stream --predicate 'process == "Bytover" AND subsystem == "com.apple.sandbox"'
```

Sandbox violations appear in `Console.app` filtered on `process:Bytover` + `subsystem:com.apple.sandbox`.
