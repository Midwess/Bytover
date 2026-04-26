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

- Apple Developer Identifier `com.midwess.bytover` has the **App Sandbox** capability enabled (already configured on the App ID).
- App Store Connect app record exists for `com.midwess.bytover` with at least one draft version slot (Apple rejects uploads with no slot).
- The `bytover-ci-notarization` API key (already provisioned for notarization) has at least the `Developer` role in App Store Connect → Users and Access → Integrations.
- The `APPLE_CERTIFICATE` `.p12` (reused from the Developer ID path) **must contain both** the `Apple Distribution` and `3rd Party Mac Developer Installer` identities. Keychain Access exports a multi-identity `.p12` if you select all three certs (Developer ID Application + Apple Distribution + 3rd Party Mac Developer Installer) before `File → Export`.

### Files in the repo

| Path | Purpose |
|---|---|
| `desktop/src-tauri/tauri.conf.appstore.json` | Overlay applied via `--config`. Picks Apple Distribution identity, sandbox entitlements, the embedded provisioning profile, and forces `bundle.targets: ["app"]` (no `.dmg`). |
| `desktop/src-tauri/entitlements.appstore.plist` | Sandbox-compliant entitlements. Strict subset of the Developer ID set: enables `app-sandbox`, `network.client/server`, `files.user-selected.read-write`, `files.bookmarks.app-scope`. **Excludes** `cs.disable-library-validation` and `cs.allow-unsigned-executable-memory` (App Store rejects). |
| `desktop/src-tauri/Bytover_Production.provisionprofile` | Mac App Store distribution provisioning profile. Committed to the repo; carries no private key — only Team ID, App ID, expiration, and the public DER cert. |

### Secrets (in GitHub environment `production`)

The App Store path **introduces no new secrets**. It reuses the existing Developer ID secrets:

| Secret | How it's used on App Store path |
|---|---|
| `APPLE_CERTIFICATE` | Decoded and imported into a temp keychain on the App Store runner. The `.p12` must contain `Apple Distribution` and `3rd Party Mac Developer Installer` identities (multi-identity `.p12`). |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the multi-identity `.p12`. |
| `APPLE_TEAM_ID` | Passed to the build for `team-identifier` propagation. |
| `APPLE_API_ISSUER` / `APPLE_API_KEY_ID` / `APPLE_API_KEY_BASE64` | Reused from the notarization path — the same `.p8` works for both `notarytool submit` and `xcrun altool --upload-app`. |

The signing identity strings (`Apple Distribution`, `3rd Party Mac Developer Installer`) are **hardcoded** as prefixes in the workflow because `codesign`/`productbuild` substring-match against the keychain's identity CNs. Since the App Store run uses a fresh temp keychain populated only from `APPLE_CERTIFICATE`, the prefix uniquely picks the right identity without needing a separate `*_IDENTITY` secret.

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
| `ITMS-91065: Missing signing certificate` | `.app` and `.pkg` signed with mismatched identities, or the `APPLE_CERTIFICATE` `.p12` lacks the `Apple Distribution` or `3rd Party Mac Developer Installer` identity. | Inspect the `Import App Store certificate` step's `security find-identity` output — both identities must be listed. Re-export `APPLE_CERTIFICATE` with all required certs selected in Keychain Access. |
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

## Compliance metadata

The App Store build ships three pieces of compliance metadata beyond the entitlements file:

1. `desktop/src-tauri/Info.appstore.plist` — App Store-only `Info.plist` overlay merged into the bundled `Info.plist` by the CI step `Inject App Store Info.plist keys & re-sign`.
2. `desktop/src-tauri/PrivacyInfo.xcprivacy` — Apple Privacy Manifest. Bundled into `Bytover.app/Contents/Resources/` via `bundle.resources` in `tauri.conf.appstore.json`.
3. Localized usage strings in `desktop/src-tauri/Info.plist` (`NSAccessibilityUsageDescription`, `NSInputMonitoringUsageDescription`, `NSAppleEventsUsageDescription`).

### Export Compliance — first-time questionnaire answers

`Info.appstore.plist` declares `ITSAppUsesNonExemptEncryption=true` because Bytover uses TLS (reqwest+rustls), DTLS-SRTP (webrtc-rs), and standard hashing (SHA-256). All qualify under the U.S. EAR §740.17(b)(1) mass-market exemption — no Encryption Registration Number (ERN) required.

The first build with this declaration triggers a one-time questionnaire in App Store Connect. Answer with these literal values:

1. **"Does your app use encryption?"** → **Yes**
2. **"Does your app qualify for any of the exemptions provided in Category 5, Part 2?"** → **Yes**
3. Tick the exemption that begins:
   > "Your app uses or accesses encryption only for authentication, digital signature, or the decryption of data or files… **or** your app uses encryption only for the purpose of supporting the secure connections that are integral to the device or operating system, such as TLS, HTTPS, or DTLS"
4. **Do not** enter an `ITSEncryptionExportComplianceCode` — Bytover does not have an ERN and does not need one under this exemption.

Once answered, App Store Connect remembers the answer for every future build that carries `ITSAppUsesNonExemptEncryption=true`. No per-build interaction is needed.

**When this changes:** if a future feature ships proprietary cryptography, end-to-end key exchange beyond DTLS-SRTP, or cryptographic operations as a primary product feature (encrypted-at-rest storage, advertised E2E messaging), the §740.17(b)(1) exemption may no longer apply. Consult Legal before merging such a feature; an ERN filing with U.S. BIS may be required (~2 week turnaround).

### Privacy Manifest — when to update `PrivacyInfo.xcprivacy`

`PrivacyInfo.xcprivacy` declares which "required reason" APIs Bytover's bundle uses, per Apple's privacy manifest spec (enforced since 2024-05-01).

Currently declared:

| Category | Reason | Source |
|---|---|---|
| `NSPrivacyAccessedAPICategoryFileTimestamp` | `C617.1` | `desktop/src-tauri/src/thumbnail.rs:313`, `libs/core-services/src/local_storage/file_system.rs:21` |
| `NSPrivacyAccessedAPICategorySystemBootTime` | `35F9.1` | `libs/core-services/src/utils/time.rs:64`, `libs/core-services/src/local_storage/file_system.rs:149` |
| `NSPrivacyAccessedAPICategoryUserDefaults` | `CA92.1` | Implicit via Tauri 2 WebKit framework linkage |

**Update the manifest when a PR introduces:**

- New `\.modified\(\)|\.created\(\)|\.accessed\(\)` calls on file metadata → confirm `FileTimestamp` is declared.
- New `SystemTime::now()` paired with `duration_since(UNIX_EPOCH)` → `SystemBootTime` already covers this; only add a new entry if the call site is in App Store-reachable code.
- New disk-space probes (`available_space` from `fs2::`, `statfs`, `attributesOfFileSystem`) → add `NSPrivacyAccessedAPICategoryDiskSpace` with reason `E174.1` (display info to user) or `B728.1` (free space for app).
- New direct `NSUserDefaults` access from Rust (via `objc2-foundation`) → already covered, but verify reason `CA92.1` still applies (use `1C8F.1` if accessing across an App Group).
- A third-party SDK (analytics, crash reporting) that ships its own `PrivacyInfo.xcprivacy` → no manifest update needed; Apple stitches it in. But re-run upload validation locally to confirm the merge.

The CI step `Privacy manifest drift check (warning-only)` runs on every App Store build and emits `::warning::` annotations if a PR adds required-reason API symbols without modifying the manifest. Treat the warnings as a yellow flag — they are intentionally non-blocking so emergency releases can still ship.

**App Privacy storefront answers** (App Store Connect → My Apps → Bytover → "App Privacy") are managed in the storefront UI and are independent of this bundled manifest. They must be kept consistent by humans. The bundled manifest currently declares `NSPrivacyTracking=false`, no tracking domains, and no collected data types; the storefront must reflect the same. If a future feature collects user data (email for account, telemetry), the PR must update both the storefront answers and the bundle manifest.

### Sandbox usage descriptions — App Review rejection log

Apple App Review §5.1.1 rejects vague usage strings. Past rejections we have already fixed:

- Old `NSAccessibilityUsageDescription` = "Required to control windows and system interactions." → too vague.
- New: "Bytover uses accessibility to position the file-transfer shelf next to the active window and to detect drag-and-drop targets across applications."

- Old `NSInputMonitoringUsageDescription` = "Required to detect global mouse and keyboard input." → too vague.
- New: "Bytover uses global keyboard and mouse input to trigger the shelf with the configured shortcut and to detect shift-drag for cross-app file transfers."

When adding a new usage string: name the user-visible feature (not the abstract OS capability) and reference how the user triggers it. Reviewers manually test the named flow; vague strings draw rejections even when the build is otherwise correct.

### CI verification (what runs automatically)

The `Inject App Store Info.plist keys & re-sign` step asserts:

- `plutil -lint` against both `Info.plist` (post-merge) and `PrivacyInfo.xcprivacy`.
- `[ -f "$APP/Contents/Resources/PrivacyInfo.xcprivacy" ]` — fails if the manifest is missing from the bundle.
- `plutil -extract ITSAppUsesNonExemptEncryption raw "$INFO"` — fails if the export-compliance key did not land.
- `codesign --verify --deep --strict --verbose=2 "$APP"` — fails if the post-`plutil` re-sign produced an invalid signature.
- `codesign -d --entitlements - --xml "$APP"` — sanity check on the re-signed entitlements.

If any of these fail, the build halts before `productbuild` and `altool --upload-app`. No malformed bundle reaches App Store Connect.
