# BitBridge

# Prequisitudes:
1. Rust `1.81`
2. Android NDK `28.0.12916984`

# Development workflow:
## Installation:

### 1. Android NDK:
Install correct Android NDK via `Android Studio > tools > Android Sdk Manager`

### 2. Environments:
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

### 3. Openssl
#### MacOS
```bash
brew install openssl@3
```

### 4. Enable target build
```
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### 4. Use cache to speed up build and compile times (optional):
Use sccache
```
brew install sccache
```
Enable sccache for cargo
```bash
export RUSTC_WRAPPER=$(which sccache)
export SCCACHE_CACHE_SIZE="50G"
sccache --stop-server
sccache --start-server
```

## Build:
### 1. Android
Build the rust binary by selecting the `shared` module, and `press build in Android Studio`
### 2. Desktop
```bash
cd Desktop; cargo build
```
### 3. iOS
Build the shared module
```bash
cd shared; cargo build
```
Option xcode and trigger run as normal

#### Output architect:
It is decided via variable `CARGO_XCODE_TARGET_ARCH`, search in shared project and adjusted it according to your choice
