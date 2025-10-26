# All platforms:
- 
    ```bash
    # if on macOS
    brew install openssl@3
    ```
-   ```bash
    git submodule update --init --recursive
    ```
- [Surreal DB Installation](https://surrealdb.com/docs/surrealdb/installation/linux)
    ```bash
    surreal start --bind 127.0.0.1:8500 --user root --pass root --log debug

    # on another shell
    surreal sql --endpoint http://127.0.0.1:8500 --auth-level root --username root --password root
    > DEFINE NAMESPACE development;
    > USE NS development;
    > DEFINE USER devlog ON NAMESPACE PASSWORD 'ssh' ROLES OWNER;
    ```

- Kong gateway
    ```bash
    docker compose up 
    ```
- auth-gateway
    ```bash
    cd auth-gateway && cargo run
    ```
- Back-end
    ```bash
    cd backend && AWS_ACCESS_KEY_ID="AWS_ACCESS_KEY_ID" AWS_SECRET_ACCESS_KEY="AWS_SECRET_ACCESS_KEY" AWS_ENDPOINT_URL="AWS_ENDPOINT_URL" cargo run
    ```

# Front-end & native development
- [Protocol Buffer Compiler Installation](https://protobuf.dev/installation/)

- Generate types for language of your choice
    - `Swift` for `iOS`
    - `Java` for `Android`
    - `Typescript` for `Web`
    ```bash
    # all
    cargo build -p shared_types --target wasm32-unknown-unknown

    # typescript only
    cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript

    # swift and java
    cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features swift,java
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
- Open XCode project at `ios/BitBridge.xcodeproj`
- Run with XCode

## Desktop
- Because MacOS doesn't support 'deep-linking' on development,
We will need to explicitly set access token in the environment variable.
```bash
export BTIBRIDGE_ACCESS_TOKEN='<token>'
```
```bash
cd Desktop; pnpm tauri dev
```
