FROM rust:1.93-alpine AS builder
WORKDIR /build
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
COPY gateway-registration/ ./gateway-registration/
COPY libs/ ./libs/
RUN cargo build --release --manifest-path gateway-registration/Cargo.toml

FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder /build/gateway-registration/target/release/gateway-registration /usr/local/bin/
ENTRYPOINT ["gateway-registration"]
