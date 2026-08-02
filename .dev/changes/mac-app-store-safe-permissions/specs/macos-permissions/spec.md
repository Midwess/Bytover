# Delta for macOS Permissions

## ADDED Requirements

### Requirement: Non-Privileged Mac App Store Launch

The Mac App Store build SHALL launch without requesting Accessibility, Input Monitoring, administrator, or equivalent privileged access.

#### Scenario: First launch on a clean macOS account

- GIVEN the user has never granted Bytover Accessibility or Input Monitoring access
- WHEN the user launches the Mac App Store build
- THEN Bytover does not display or trigger either permission prompt
- AND Bytover does not open a related System Settings pane

#### Scenario: Previously denied access

- GIVEN Accessibility and Input Monitoring access are denied
- WHEN the user relaunches the Mac App Store build
- THEN Bytover continues without requesting the denied permissions

### Requirement: Distribution-Bound Global Monitoring

The system SHALL prevent global input and mouse monitoring from starting in the Mac App Store build while allowing the direct-download build to retain its approved distribution behavior.

#### Scenario: Mac App Store monitor startup

- WHEN the Mac App Store build completes application setup
- THEN no global input or mouse monitor is started

#### Scenario: Direct-download monitor startup

- GIVEN the direct-download build follows its existing permission policy
- WHEN that build completes application setup
- THEN its global monitoring behavior remains available

### Requirement: Ordinary In-App Interaction

The Mac App Store build SHALL preserve ordinary in-window file selection and drag-and-drop behavior without privileged monitoring.

#### Scenario: Drag files into Bytover

- GIVEN the Mac App Store build is running without Accessibility or Input Monitoring access
- WHEN the user drags supported files into a Bytover window
- THEN Bytover accepts the files through normal application drag-and-drop APIs

#### Scenario: Select files with the system picker

- WHEN the user chooses files through Bytover's file picker
- THEN the selected files remain available to the transfer flow

### Requirement: Verifiable Distribution Variant

The build system SHALL make Mac App Store behavior explicit and verifiable before packaging.

#### Scenario: App Store workflow build

- WHEN the `macos-appstore` workflow builds the application
- THEN the Mac App Store distribution feature is enabled
- AND the workflow verifies the expected distribution marker before upload

#### Scenario: Direct-download workflow build

- WHEN the macOS direct-download workflow builds the application
- THEN the Mac App Store distribution feature is not enabled

