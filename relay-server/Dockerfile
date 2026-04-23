# Rust builder stage
FROM midwess/rust:builder1.89.0 as builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release -p relay-server

# Runtime stage
FROM midwess/rust:runner1.89.0 as runner
WORKDIR /app

# Copy binary and content
COPY --from=builder /app/target/release/relay-server ./

EXPOSE 9101
EXPOSE 3478/udp
CMD ["./relay-server"]
