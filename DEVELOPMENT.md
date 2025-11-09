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
    ```

    # on another shell
    ```bash
    surreal sql --endpoint http://127.0.0.1:8500 --auth-level root --username root --password root
    ```

    ```sql
    DEFINE NAMESPACE development;
    USE NS development;
    DEFINE DATABASE `app-gateway`;
    DEFINE DATABASE system;
    DEFINE DATABASE bitbridge;
    USE DB `app-gateway`;
    DEFINE USER devlog ON NAMESPACE PASSWORD 'ssh' ROLES OWNER;
    CREATE application:[ 'BitBridge', 50515741832650750 ] CONTENT {
        avatar_urls: [],
        icon_url: 'icon_url',
        maximum_device: 6,
        name: 'BitBridge',
        order_id: 50515741832650750,
        random_avatar: true,
        redirect_url: [
            { platform: 'Web', url: 'http://localhost:8000/' },
            { platform: 'Ios', url: 'BitBridge://authorize' }
        ]
    };
    ```

- Kong gateway
    ```bash
    docker compose up 
    ```
- auth-gateway
    ```bash
    cd auth-gateway && DEVLOG_GOOGLE_CLIENT_ID="DEVLOG_GOOGLE_CLIENT_ID" DEVLOG_GOOGLE_CLIENT_SECRET="DEVLOG_GOOGLE_CLIENT_SECRET" DEVLOG_KONG_GATEWAY_ADMIN_URL="http://localhost:8001" cargo run
    ```
- Back-end DB:
    ```bash
    cd backend && docker compose up
    ```
- Back-end
    ```bash
    cd backend && BITBRIDGE_DB_CONNECTION_STRING="postgres://bitbridge:bitbridgepass@localhost:5433/bitbridge" DEVLOG_GOOGLE_CLIENT_ID="DEVLOG_GOOGLE_CLIENT_ID" AWS_ACCESS_KEY_ID="AWS_ACCESS_KEY_ID" AWS_SECRET_ACCESS_KEY="AWS_SECRET_ACCESS_KEY" AWS_ENDPOINT_URL="AWS_ENDPOINT_URL" cargo run
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
- Environment variables are applied at build time
-
    ```bash
    cd web-next
    pnpm install
    deno run wasm:dev
    DEVLOG_KONG_GATEWAY_ADMIN_URL=http://localhost:8001 deno task dev
    ```
- Access your app at `http://localhost:8000`.

## Android
- `Android NDK 28`
- Environment variables are applied at build time
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
- Environment variables are applied at build time in XCode settings
- Run with XCode

## Desktop
#### MacOS only
- Because MacOS doesn't support 'deep-linking' on development, so that we cannot authorize,
Instead, we will explicitly set access token via environment variable.
- Environment variables are applied at build time
```bash
export BTIBRIDGE_ACCESS_TOKEN='<token>'
```

```bash
cd Desktop; pnpm dev 
```
