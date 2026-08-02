# Design: Mac App Store Safe Permissions

## Overview

The Mac App Store and direct-download products intentionally have different system-integration capabilities. That distinction must be encoded at compile time because a runtime switch could accidentally ship a prohibited path to App Review.

## Key Decisions

### Decision 1: Compile-Time Distribution Feature

**Context:** The current build overlay changes signing and entitlements but not application behavior.

**Options:**

1. Inspect runtime environment/configuration — easy to change but capable of drift after packaging.
2. Add a Cargo feature — explicit, testable, and removes prohibited launch code from the artifact.

**Decision:** Add `mac-app-store` and have only the App Store workflow enable it.

### Decision 2: Remove Global Monitoring, Preserve Local Interaction

**Context:** App Review rejected launch-time access while ordinary file interaction must remain useful.

**Options:**

1. Ask later or after an explanation — still creates approval risk for a nonessential global feature.
2. Disable global monitoring in the App Store build and keep normal application-window APIs.

**Decision:** The App Store build never requests or uses Accessibility/Input Monitoring; in-window drag/drop and file pickers remain.

### Decision 3: Fail Closed in CI

**Context:** An operator can select the correct workflow but still package the wrong feature set.

**Decision:** CI verifies the distribution marker, Info.plist keys, and entitlements before upload. Missing evidence blocks packaging.

## Security and Privacy Considerations

- App Store users are not asked for unrelated system-wide observation privileges.
- The direct-download product retains its existing consent behavior and is outside this App Store policy change.
- No runtime server flag may reactivate global monitoring in the App Store artifact.

