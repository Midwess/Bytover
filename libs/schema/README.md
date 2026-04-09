# Schema

Contains protobuf schema of all services

### Workflow
You must install protoc first
```sh
# MacOS
brew install protoc
```

# Usage
### Typescript
```
"@devlog/typescript": "file:../schema/typescript"
```
### Rust
```toml
[dependencies]
schema = {path = "../schema/rust"}
```
