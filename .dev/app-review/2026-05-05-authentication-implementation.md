# App Review Authentication Implementation

This note addresses the May 5, 2026 review findings for Guideline 4.8 and Guideline 4.

## Implemented Review Path

- The macOS sign-in window presents equivalent **Sign in with Apple** and **Sign in with Google** controls.
- The Mac App Store feature routes both providers through `ASWebAuthenticationSession` and presents the session from Bytover's authentication window.
- Only one authenticated web session can be active at a time. Completion, user cancellation, invalid callback, provider failure, and session-start failure return typed recoverable outcomes.
- The accepted callback is exactly `bytover://app/oauth/callback`; mismatched hosts and paths are rejected before token persistence.
- The Mac App Store UI has no manual access-token input, auth-URL copy control, or generic-browser fallback.
- Missing, partial, or invalid Apple server configuration returns a recoverable provider-unavailable state. It does not crash Bytover or app-gateway and does not disable Google.
- No Apple Team ID, Service ID, Key ID, private key, or other provider secret is embedded in the application.

The shared schema compatibility pointer is `883cbad146eed9889b92eaf881d1af1f68924850`. Google and Apple now finish through the same app-bound callback contract; the corresponding app-gateway change is `9a06ffdd1d9fbf05949b9a465d1be5694571fddf`.

## Release Gate Before Resubmission

Do not send the review build while Sign in with Apple reports unavailable. Supply all six app-gateway values through the existing deployment environment:

- `APPLE_SIGN_IN_SERVICE_ID`
- `APPLE_SIGN_IN_TEAM_ID`
- `APPLE_SIGN_IN_KEY_ID`
- `APPLE_SIGN_IN_PRIVATE_KEY_BASE64`
- `APPLE_SIGN_IN_REDIRECT_URI`
- `APPLE_SIGN_IN_ALLOWED_RETURN_URIS` containing the exact `bytover://app/oauth/callback`

Then verify the signed Mac App Store artifact against the deployed gateway:

1. Sign in with Google and confirm the app closes the authenticated session and loads the account.
2. Sign out, sign in with Apple, select **Hide My Email**, and confirm full account access.
3. Sign out and return with the same Apple account; confirm sign-in succeeds when Apple omits name and email.
4. Cancel each provider once; confirm the app returns to provider selection without creating a session.
5. Attempt a second sign-in while one is open; confirm the app rejects it without opening another window.
6. Confirm no default browser opens and no token-paste instruction is visible.

## App Store Connect Reply Draft

> We revised the macOS app to offer Sign in with Apple as an equivalent option alongside Sign in with Google. In the Mac App Store build, both options use `ASWebAuthenticationSession` from AuthenticationServices with the app-bound `bytover://app/oauth/callback` callback. The app no longer opens the default browser for these login flows and no longer offers manual access-token entry or copy/paste fallback. Sign in with Apple requests only name and email, supports Hide My Email, and does not use login interactions for advertising. To verify, open the sign-in window and select either provider; the secure system authentication session appears within the app's presentation context.
