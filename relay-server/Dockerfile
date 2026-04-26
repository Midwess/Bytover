# Rust builder stage
FROM midwess/rust:builder1.89.0 as builder
ARG PROTOC_VERSION=28.3
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl unzip ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && case "$(uname -m)" in \
         x86_64)  PROTOC_ARCH=x86_64 ;; \
         aarch64) PROTOC_ARCH=aarch_64 ;; \
         *) echo "Unsupported arch: $(uname -m)" >&2; exit 1 ;; \
       esac \
    && curl -fsSL -o /tmp/protoc.zip \
         "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${PROTOC_ARCH}.zip" \
    && unzip -q /tmp/protoc.zip -d /usr/local \
    && rm /tmp/protoc.zip \
    && chmod 755 /usr/local/bin/protoc
WORKDIR /app
COPY . .
RUN cargo build --release -p relay-server

# GeoIP asset stage
FROM debian:bookworm-slim as geoip-assets
ARG MAXMIND_LICENSE_KEY=""
WORKDIR /assets
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN if [ -n "$MAXMIND_LICENSE_KEY" ]; then \
      curl -fsSL -o /tmp/geolite2.tar.gz \
        "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key=${MAXMIND_LICENSE_KEY}&suffix=tar.gz" \
      && tar --strip-components=1 -xzf /tmp/geolite2.tar.gz \
           --wildcards '*/GeoLite2-Country.mmdb' '*/LICENSE.txt' \
      && mv LICENSE.txt LICENSE-GEOLITE2.txt \
      && rm /tmp/geolite2.tar.gz; \
    else \
      echo "MAXMIND_LICENSE_KEY not provided; relay will fall back to gRPC GetRegion at runtime" > NO-GEOIP-DB.txt; \
    fi

# Runtime stage
FROM midwess/rust:runner1.89.0 as runner
WORKDIR /app

COPY --from=builder /app/target/release/relay-server ./
COPY --from=geoip-assets /assets /app/assets
COPY relay-server/assets/README.md /app/assets/README.md

EXPOSE 9101
EXPOSE 3478/udp
CMD ["./relay-server"]
