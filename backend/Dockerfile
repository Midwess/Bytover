# Rust builder stage
FROM midwess/rust:builder1.89.0 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p backend

# Runtime stage
FROM midwess/rust:runner1.89.0 as runner
WORKDIR /app

# Copy binary and content
COPY --from=builder /app/target/release/backend ./

EXPOSE 3000
CMD ["./backend"]
