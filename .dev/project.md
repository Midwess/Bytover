# Bytover Project Context

## Product

Bytover is a cross-platform file-sharing and synchronization application. The current remediation scope targets the Tauri-based macOS desktop application distributed through the Mac App Store while preserving the direct-download desktop distributions.

## Technology Stack

- Rust workspace with shared application logic built around Crux-style modules, events, operations, and shell executors
- Tauri 2 desktop shell with platform-specific Rust integrations
- React 19, TypeScript, Vite, and Tailwind CSS for desktop UI
- Rust backend using Tokio, tonic gRPC, Axum, SeaORM, SQLx, and PostgreSQL
- Protocol Buffers in the `libs/schema` Git submodule with generated Rust and TypeScript bindings
- Next.js web application for OAuth callback pages, policy content, and browser file-transfer surfaces
- GitHub Actions build pipeline with distinct `macos-dmg` and `macos-appstore` paths

## Repository Areas

| Area | Responsibility |
|---|---|
| `desktop/src/` | React desktop windows and settings UI |
| `desktop/src-tauri/` | Tauri shell, macOS integrations, signing, entitlements, and StoreKit |
| `shared/src/app/` | Cross-platform application state, commands, events, and operations |
| `shared/src/protocol/` | gRPC clients and authentication/payment adapters |
| `backend/` | Bytover backend services, persistence, capabilities, and StoreKit verification bridge |
| `libs/schema/` | Shared protobuf contracts and generated bindings |
| `web-next/` | OAuth callback and policy/web surfaces |
| `.github/workflows/desktop-build.yml` | Distribution-specific desktop build and App Store upload pipeline |

## Conventions

- Keep platform and distribution-specific behavior at the shell/build boundary; keep reusable state transitions and error handling in `shared` modules.
- Represent asynchronous application behavior as typed operations and events rather than embedding business logic directly in React components.
- Add unit tests beside Rust modules with `#[cfg(test)]`; cover operation outcomes and state transitions without requiring live external services.
- Treat provider credentials and deployment values as environment configuration. Missing required production configuration must fail clearly and must not silently downgrade authentication or payment security.
- Keep the Mac App Store build explicitly selectable and isolated from the direct-download macOS build.
- Use OpenSpec delta files for proposed requirements. Proposal status remains `draft` until reviewed and manually changed to `approved`.

## External Dependencies

- Apple Developer Program and App Store Connect
- Sign in with Apple provider configuration and credentials
- Existing external app-gateway authentication service implementing the protobuf `AuthService`
- StoreKit product `com.midwess.bytover.premium`
- Apple App Store sandbox/reviewer environment

## Latest Analysis

The May 05, 2026 App Review remediation program is coordinated from `.dev/app-review/milestones.md` and split into four independent draft changes.

Key architecture findings:

- App Store behavior must use a compile-time distribution feature at the Tauri/build boundary.
- Authentication must carry provider choice end-to-end and use a typed macOS authenticated-session adapter.
- Existing StoreKit transaction architecture should be preserved; reviewer-visible product availability, localized presentation, restore behavior, and external release gates are missing.
- Account deletion spans the external identity owner and Bytover repositories and therefore requires a durable idempotent workflow.

Key conventions extracted:

- Compile prohibited App Store behavior out rather than relying on a mutable runtime toggle.
- Keep provider credentials and App Store configuration outside source control while making readiness failures explicit.
- Treat provider subject, not email, as the social identity key.
- Accept account deletion durably before clearing the client's only credential; retry server cleanup to completion.
