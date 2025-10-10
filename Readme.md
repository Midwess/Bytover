# BitBridge

# Prerequisites:
1. Rust `1.89`
2. Android NDK `28`

# Development workflow:
## Installation
#### Openssl
```bash
# Mac
$ brew install openssl@3
```

### Enable target build
```
$ rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### Pull git submodules
```
$ make gsu
```

### Dependencies
See [Protocol Buffer Compiler Installation](https://protobuf.dev/installation/)
See [pnpm Installation](https://pnpm.io/installation)

### (Optional) Use cache to speed up build process:
Use sccache
```
$ brew install sccache
```
Enable sccache for cargo
```bash
$ export RUSTC_WRAPPER=$(which sccache)
$ export SCCACHE_CACHE_SIZE="50G"
$ sccache --stop-server
$ sccache --start-server
```

## Run

### Generate shared types
This will generate entity code in multiple language (swift, kotlin, typescript, etc...).
```bash
$ make gen
```

### Backend
```bash
$ cd backend
$ cargo run
```

### Web
NextJs with Deno runtime

- Deno (Runtime) version: `>= 2.0`
- Node (Build time) version: `>= 20.0`
- pnpm version: `>= 10.0.0`
```bash
$ cd web-next
# Install dependencies
$ pnpm install
# To start in dev mode
$ deno task dev
# To start in production mode
$ deno task build
$ PORT=<PORT> deno task start
```

### Android:
Install correct Android NDK via `Android Studio > tools > Android Sdk Manager`

### Environments:
```bash
export ANDROID_HOME=/Users/tiendang/Library/Android/sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/28.0.12916984
export TOOLCHAIN=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64
export AR=$TOOLCHAIN/bin/llvm-ar
export LD=$TOOLCHAIN/bin/ld
export RANLIB=$TOOLCHAIN/bin/llvm-ranlib
export STRIP=$TOOLCHAIN/bin/llvm-strip
export PATH=$ANDROID_HOME:$PATH
export PATH=$PATH:$TOOLCHAIN/bin
```

### 1. Android
Build the rust binary by selecting the `shared` module, and `press build in Android Studio`
### 2. Desktop
```bash
$ cd Desktop; cargo build
```
### 3. iOS
Open xcode and trigger run

##### Output architect:
Set variable `CARGO_XCODE_TARGET_ARCH`, search in shared project and adjusted it according to your choice

# Other commands:
```bash
# Format code
$ make ffmt
# Sync submodule
$ make gsu
```
