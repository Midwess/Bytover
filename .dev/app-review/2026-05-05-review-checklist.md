# App Review Verification Checklist

**Submission**: `5b4d77db-c9ff-4e4d-8112-44cdde37d040`

**Status**: in progress — implementation verified locally; production-eligible signed build and clean-account UI verification pending

## Verification Build Attempt

- Branch: `mac-app-store-safe-permissions`
- Commit: `b0ff0a914fc78f29aae04a2250ef047192813037`
- GitHub Actions run: `https://github.com/Midwess/Bytover/actions/runs/30738512503`
- Workflow inputs: `platform=macos-appstore`, `upload_to_app_store=false`
- Result: failed before the deploy job started because the `production` GitHub environment only permits the `production` branch.
- Artifact: none; no signing secret was exposed and no package was produced.
- App Store Connect impact: none; the upload step was disabled and no deploy step ran.
- Next action: merge the reviewed change to `production`, run `platform=macos-appstore` with `upload_to_app_store=false`, retain the signed package, then perform every clean-account check below before an intentional upload.

## Artifact Identity

- [ ] Record replacement version and build number.
- [x] Record the attempted Git commit, GitHub Actions run URL, and environment-gate result.
- [ ] Download and retain the exact `Bytover-appstore-pkg` artifact.
- [ ] Verify App Store signature, sandbox entitlement, provisioning profile, bundle ID, and distribution marker.
- [ ] Confirm the artifact uploaded to App Store Connect matches the tested artifact.

## Guideline 2.4.5(v): Launch and Permissions

Automated artifact gates must pass before the manual checks below: compiled/package distribution markers, prohibited usage-description removal, sandbox entitlement, prohibited-entitlement absence, signing integrity, and package signature. Then install and launch the exact retained package under a clean standard macOS account. Treat any authorization requested by the package installer as installation behavior; the rejection is resolved only when Bytover itself launches and operates without requesting administrator access.

- [ ] Use a clean macOS account or reset Bytover privacy permissions.
- [ ] Launch the App Store build and confirm no Accessibility prompt.
- [ ] Confirm no Input Monitoring prompt.
- [ ] Confirm no administrator/password prompt.
- [ ] Confirm no automatic navigation to either System Settings privacy pane.
- [ ] Confirm no global shake/shift-drag monitor behavior.
- [ ] Confirm file picker and drag into an open Bytover window still work.

## Guidelines 4.8 and 4: Authentication

- [ ] Confirm Sign in with Apple and Sign in with Google are both visible and equivalent.
- [ ] Confirm Google starts `ASWebAuthenticationSession` and completes through the app-bound callback.
- [ ] Confirm Apple starts `ASWebAuthenticationSession` and completes through the app-bound callback.
- [ ] Confirm cancellation returns to provider selection without creating a session.
- [ ] Confirm an invalid/mismatched callback is rejected.
- [ ] Confirm Apple Hide My Email creates a fully functional account.
- [ ] Confirm matching Google/Apple emails are not automatically merged.
- [ ] Confirm no manual access-token or generic browser fallback is shown in the App Store build.
- [ ] Confirm production provider configuration is present and no secrets/tokens appear in logs.

## Guideline 3.1.1: In-App Purchase

- [ ] Confirm `com.midwess.bytover.premium` metadata, localization, pricing, agreements, and storefronts are complete.
- [ ] Confirm the IAP is associated with the replacement app version.
- [ ] Confirm StoreKit returns the product in the exact signed artifact.
- [ ] Confirm Settings displays StoreKit-localized price.
- [ ] Purchase Pro with a fresh sandbox/reviewer-like free account.
- [ ] Confirm server verification and Pro capability refresh complete.
- [ ] Restore Pro with a prior purchaser.
- [ ] Confirm no-prior-purchase restore result is understandable.
- [ ] Confirm cancellation and retryable failure behavior.
- [ ] Relaunch and confirm unfinished transactions recover safely.
- [ ] Audit all App Store UI for license-key fields and external purchase calls to action.
- [ ] Confirm externally entitled Pro and IAP Pro expose the same functionality.

## Guideline 5.1.1(v): Account Deletion

- [ ] Confirm Settings → Account exposes Delete Account distinctly from Sign Out.
- [ ] Cancel deletion and confirm no state changes.
- [ ] Confirm deletion with required recent authentication.
- [ ] Confirm the server durably accepts the request before the client removes credentials.
- [ ] Confirm active sessions stop and local profile/token/transfers/shelves/aliases/capabilities clear.
- [ ] Confirm deleted sessions no longer authenticate.
- [ ] Confirm backend and external identity cleanup complete or enter the documented retry workflow.
- [ ] Confirm Apple token revocation for an Apple-authenticated test account.
- [ ] Confirm missing-token fallback does not prevent Bytover data deletion.
- [ ] Confirm policy instructions and retention wording match observed behavior.

## App Store Connect Review Information

- [ ] Provide a free review account or exact account-creation steps.
- [ ] Identify both login buttons and state that `ASWebAuthenticationSession` is used.
- [ ] Identify the IAP product ID, purchase location, Restore Purchases action, and expected Pro result.
- [ ] Identify Delete Account location and expected timing.
- [ ] Map each original guideline to the implemented fix and evidence.
- [ ] Replace every placeholder in `2026-05-05-response-draft.md`.
- [ ] Have a second person reproduce the full checklist without developer assistance.

## Submission Gate

- [ ] All four proposal task lists are complete.
- [ ] All checks above pass against one exact signed artifact.
- [ ] The response draft contains no unverified claim.
- [ ] Submit the replacement app version and its IAP together.
