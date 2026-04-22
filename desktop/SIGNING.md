# macOS Code Signing

This document covers how Bytover's desktop app is signed for both local development and CI-produced releases.

## Profiles at a glance

| Aspect | `development` | `production` |
|---|---|---|
| Certificate | Apple Development | Developer ID Application |
| Provisioning profile | `Bytover.provisionprofile` (embedded) | None (Developer ID doesn't use profiles) |
| Entitlements file | `entitlements.dev.plist` (+ `get-task-allow`) | `entitlements.plist` |
| Notarization | No | Yes (`notarytool` + stapled ticket) |
| Gatekeeper (`spctl`) | Expected to reject | Must accept |
| Local debugger attach | Yes | No |
| GitHub Actions environment | `development` | `production` |
| Triggered automatically on | Any non-`production` branch | `production` branch |

## How Tauri config merges

- `src-tauri/tauri.conf.json` — base config, represents the **production** shape (signing identity `"-"` for ad-hoc, no embedded profile, prod entitlements).
- `src-tauri/tauri.conf.dev.json` — overlay applied via `tauri build --config src-tauri/tauri.conf.dev.json`. Adds the Apple Development signing identity, dev entitlements, and embeds `Bytover.provisionprofile`.

Tauri's merge is deep and one-way: the overlay can add and override keys, but not remove them. That's why the base is the minimal (prod) shape and the overlay is the dev addition.

## Identifier

Bundle identifier is `com.midwess.bytover` (lowercase). This is what the existing provisioning profile encodes as `BUJKWCX7F4.com.midwess.bytover`. Team ID: `BUJKWCX7F4` (Midwess LLC). Do **not** change the identifier casing without regenerating the provisioning profile at developer.apple.com first.

## GitHub secrets

Secrets live in GitHub Actions **environments**, scoped per profile. The workflow reads `secrets.APPLE_CERTIFICATE` (etc.) — the same name resolves to different values depending on which environment the job is running in.

### `development` environment

| Secret | Source |
|---|---|
| `APPLE_CERTIFICATE` | `base64 -i AppleDevelopment.p12` (no line wrapping). Export from Keychain Access. |
| `APPLE_CERTIFICATE_PASSWORD` | Password chosen when exporting the `.p12`. |
| `APPLE_SIGNING_IDENTITY` | `Apple Development: <Your Name> (BUJKWCX7F4)` — match exactly what `security find-identity -v -p codesigning` prints. |
| `APPLE_TEAM_ID` | `BUJKWCX7F4` |
| `TAURI_SIGNING_PRIVATE_KEY` | Minisign key for updater. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passphrase for minisign key. |

No `APPLE_ID` / `APPLE_PASSWORD` — the workflow passes empty values for dev so `notarytool` is never invoked.

### `production` environment

| Secret | Source |
|---|---|
| `APPLE_CERTIFICATE` | `base64 -i DeveloperID.p12` (no line wrapping). |
| `APPLE_CERTIFICATE_PASSWORD` | Password chosen when exporting the `.p12`. |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Midwess LLC (BUJKWCX7F4)`. |
| `APPLE_ID` | Apple ID email associated with the developer account. |
| `APPLE_PASSWORD` | **App-specific** password from appleid.apple.com (format `abcd-efgh-ijkl-mnop`). Not the Apple ID login password. |
| `APPLE_TEAM_ID` | `BUJKWCX7F4` |
| `TAURI_SIGNING_PRIVATE_KEY` | Minisign key for updater. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passphrase for minisign key. |

Also configure a **deployment-branch restriction** on the `production` environment so that only the `production` branch can pull its secrets.

## Exporting the certificate as .p12

1. Open Keychain Access → `login` keychain → `My Certificates`.
2. Find the cert (either `Apple Development: …` or `Developer ID Application: …`).
3. Right-click → Export → File Format `Personal Information Exchange (.p12)` → choose a password.
4. `base64 -i DeveloperID.p12 | pbcopy` (macOS pipes it to clipboard with no line breaks).
5. Paste into the GitHub secret value.

## Triggering a build

Actions → `desktop-release` → Run workflow:

- `platform`: `macos`, `windows`, `linux`, or `all`.
- `profile`: `auto` (default — development unless on the `production` branch), or explicitly `development` / `production`.

The resolved profile is printed in the `resolve-profile` job's log so you can confirm which environment's secrets got loaded.

## Building locally

### Quick build (unsigned / ad-hoc)

```bash
cd desktop
pnpm install
pnpm tauri:build:prod
```

This uses `signingIdentity: "-"` from the base config → produces an ad-hoc signed `.app` that runs locally but won't pass Gatekeeper or be notarizable. Use for iteration speed.

### Dev-signed build (with debugger attach)

```bash
cd desktop
pnpm tauri:build:dev
```

This merges `src-tauri/tauri.conf.dev.json` on top → requires an `Apple Development` cert in your login keychain. To set that up:

```bash
# Import the .p12 into your login keychain
security import ~/Downloads/AppleDevelopment.p12 -k ~/Library/Keychains/login.keychain-db -P "<p12-password>" -T /usr/bin/codesign

# If codesign prompts for keychain access, unlock it first
security unlock-keychain -p "<login-password>" ~/Library/Keychains/login.keychain-db

# Verify identity is visible
security find-identity -v -p codesigning
```

### Production-style local build

Not recommended. Notarization requires round-tripping with Apple's servers (2–15 min) and burns quota. Let CI do it.

## Verification

CI runs these automatically after the Tauri build step. To run locally on a produced `.app`:

```bash
APP=desktop/src-tauri/target/universal-apple-darwin/release/bundle/macos/Bytover.app

# 1. Inspect the signature
codesign -dv --verbose=4 "$APP"

# 2. Inspect the baked-in entitlements
codesign -d --entitlements - --xml "$APP" | plutil -p -

# 3. Validate signature integrity
codesign --verify --deep --strict --verbose=2 "$APP"

# 4. Ask Gatekeeper (production only)
spctl --assess --type exec --verbose=4 "$APP"

# 5. Confirm notarization ticket is stapled (production only)
xcrun stapler validate "$APP"
```

## Troubleshooting

### `errSecInternalComponent` during sign

Usually caused by: (a) the signing identity isn't in the keychain, (b) the keychain is locked, or (c) the embedded provisioning profile targets a different identifier or Team ID.

Check:
```bash
security find-identity -v -p codesigning
security cms -D -i desktop/src-tauri/Bytover.provisionprofile | plutil -p - | grep -E 'identifier|TeamIdentifier'
```

### "App is damaged and can't be opened"

This means the quarantine attribute is set but Gatekeeper can't verify the signature. Either: the build wasn't notarized, or notarization failed silently, or the DMG was edited after signing.

Check the CI verification step logs. Locally, strip quarantine to test:
```bash
xattr -d com.apple.quarantine /Applications/Bytover.app
```

### Notarization rejects `get-task-allow`

Developer ID builds must **not** have `com.apple.security.get-task-allow = true`. This is only in `entitlements.dev.plist`. If it ends up in a prod build, check you didn't accidentally pass `--config src-tauri/tauri.conf.dev.json` to a production run.

### `APPLE_PASSWORD` auth failure

Make sure it's an **app-specific** password from appleid.apple.com → Sign-In and Security → App-Specific Passwords. The regular Apple ID login password will not work with `notarytool`.

### Wrong Team ID

Team ID must be the 10-char alphanumeric from developer.apple.com → Membership. For Midwess LLC this is `BUJKWCX7F4`. Don't confuse with the Organization ID (different format).

### Certificate rotation

When the Apple cert expires or is revoked:

1. Generate a new `.p12` from Keychain Access.
2. Update `APPLE_CERTIFICATE` in the matching GitHub environment (the workflow code doesn't change).
3. If the Team changed, update `APPLE_TEAM_ID` and `APPLE_SIGNING_IDENTITY`.
4. Re-run the workflow — verification step will confirm the new identity.
