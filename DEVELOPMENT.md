## All platforms:
- 
    ```bash
    # if on macOS
    brew install openssl@3
    ```
-   ```bash
    git submodule update --init --recursive
    ```
- [Protocol Buffer Compiler Installation](https://protobuf.dev/installation/)

- 
    ```bash
    # generate types for language of your choice

    # all
    cargo build -p shared_types --target wasm32-unknown-unknown

    # typescript only
    cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript

    # swift and java
    cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features swift,java
    ```
// TODO: back-end, auth-gateway, kong gateway
- 
    ```bash
    cd backend && cargo run
    ```

## Web

- [Deno installation](https://docs.deno.com/runtime/getting_started/installation/)

- [pnpm installation](https://pnpm.io/installation)

- [wasm-pack installation](https://drager.github.io/wasm-pack/installer/)

-
    ```bash
    cd web-next
    pnpm install
    deno run wasm:dev
    deno task dev
    ```

## Android
- `Android NDK 28`
-
    ```bash
    rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
    ```

```bash
export DEVLOG_GOOGLE_CLIENT_ID=
export DEVLOG_GOOGLE_CLIENT_SECRET=
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
Build the rust binary by selecting the `shared` module, and
`press build in Android Studio`
## iOS

## Desktop
```bash
cd Desktop; cargo build
```
