# Rust builder stage
FROM devlogstudio/rust:builder1.87.0 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p app

# Runtime stage
FROM devlogstudio/rust:runner1.87.0 as runner
WORKDIR /app

# Copy binary and content
COPY --from=builder /app/target/release/app ./

EXPOSE 3000
CMD ["./app"]
