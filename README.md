# Bytover

Cross-platform desktop application for file sharing and synchronization.

## Product

### Build from Source

This project uses GitHub Actions for automated builds. To build:

1. Go to **Actions** → **Desktop Build**
2. Select platform from dropdown (macos/windows/linux/all)
3. Enter branch name
4. Click **Run workflow**

Build artifacts will be uploaded to GitHub Releases.

#### Build Outputs

| Platform | Artifact |
|----------|----------|
| **macOS** | `Bytover_macos_universal.dmg` |
| **Windows** | `Bytover_windows_x64.exe` |
| **Linux** | `bytover_linux_x64`, `bytover_linux_arm64` |

---

### Installation

#### macOS

1. Open the DMG file:
   ```bash
   open target/universal-apple-darwin/release/bundle/dmg/Bytover_1.0.0_universal.dmg
   ```

2. Drag `Bytover.app` to the **Applications** folder

3. Launch from **Applications** or via command line:
   ```bash
   open -a Bytover
   ```

#### Windows

1. Copy the executable to your desired location:
   ```bash
   cp target/x86_64-pc-windows-msvc/release/Bytover.exe "$HOME/Desktop/Bytover.exe"
   ```

2. Run the executable:
   ```bash
   "$HOME/Desktop/Bytover.exe"
   ```

Or double-click `Bytover.exe` in File Explorer.

#### Linux

1. Copy the binary to your system:
   ```bash
   sudo cp target/x86_64-unknown-linux-gnu/release/bytover /usr/local/bin/bytover
   sudo chmod +x /usr/local/bin/bytover
   ```

2. Run the application:
   ```bash
   bytover
   ```

For desktop integration, you can create a `.desktop` file:
```bash
sudo tee /usr/share/applications/bytover.desktop > /dev/null << 'EOF'
[Desktop Entry]
Name=Bytover
Exec=/usr/local/bin/bytover
Type=Application
Categories=Network;Utility;
EOF
```

---

## Development

### Common Setup

All platforms require:
```bash
# Install dependencies (macOS)
brew install openssl@3

# Initialize submodules
git submodule update --init --recursive

# Install Protocol Buffer Compiler
# https://protobuf.dev/installation/
```

### Backend Services

Start Kong gateway and backend database:
```bash
docker compose up
```

Run the backend:
```bash
cd backend
BYTOVER_DB_CONNECTION_STRING="postgres://bitbridge:bitbridgepass@localhost:5432/bitbridge" \
GOOGLE_CLIENT_ID="GOOGLE_CLIENT_ID" \
AWS_ACCESS_KEY_ID="AWS_ACCESS_KEY_ID" \
AWS_SECRET_ACCESS_KEY="AWS_SECRET_ACCESS_KEY" \
AWS_ENDPOINT_URL="AWS_ENDPOINT_URL" \
cargo run
```

### Generate Types

Generate types for different languages:
- `Swift` for `iOS`
- `Java` for `Android`
- `Typescript` for `Web`

```bash
# All types
cargo build -p shared_types --target wasm32-unknown-unknown

# Typescript only (requires pnpm)
cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript

# Swift and Java
cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features swift,java
```

### Web

Prerequisites:
- [Deno](https://docs.deno.com/runtime/getting_started/installation/)
- [pnpm](https://pnpm.io/installation)
- [wasm-pack](https://drager.github.io/wasm-pack/installer/)

```bash
cd web-next
pnpm wasm:build
pnpm install
KONG_GATEWAY_ADMIN_URL=http://localhost:8001 pnpm dev
```

Access your app at `http://localhost`.

### iOS (Deprecated)

iOS development has been deprecated. The codebase is preserved for reference.

### Desktop Development

#### macOS

Because macOS doesn't support deep-linking in development, authorize via environment variable:
```bash
export BYTOVER_ACCESS_TOKEN='<token>'
cd desktop
pnpm dev
```

#### Building Desktop App

See [Product](#product) section for build commands.
