# macOS Code Signing

Bytover's desktop app has two signing paths: a **local** path for developers, and a **CI** path for distribution. Each produces a differently-signed artifact.

## Who consumes what

| Artifact | Produced by | Signing identity | Who can run it |
|---|---|---|---|
| Local ad-hoc `.app` | `pnpm tauri build` on your Mac | `-` (null) | You, on your own Mac |
| Local Apple-Dev `.app` | `pnpm tauri:build:dev` on your Mac | `Apple Development: <you>` | You, on your own Mac — required for cert-gated features like StoreKit / iCloud / Push |
| CI release `.app` / `.dmg` | GitHub Actions `desktop-release` workflow, `production` environment | `Developer ID Application: Midwess LLC` + notarized + stapled | Anyone on any Mac, including beta testers and public users |

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

Actions → `desktop-release` → Run workflow → pick `platform` (`macos` / `windows` / `linux` / `all`). There is no profile input — it's always Developer ID + notarize.

- From the `production` branch → tagged `v__VERSION__`, release title `Bytover v__VERSION__`
- From any other branch → tagged `v__VERSION__-beta`, release title suffix `(Beta)`

Both use the same Developer ID signing path. Beta testers and release users get the same signature guarantees; only the version number and release naming differ.

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
