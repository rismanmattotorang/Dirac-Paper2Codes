# Multi-stage build for Paper2Codes-Core
# Stage 1: Build stage
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/lib.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --features api && \
    rm -rf src

# Copy source code
COPY src ./src
COPY tests ./tests
COPY examples ./examples

# Build the application
RUN touch src/main.rs && \
    cargo build --release --features api

# Stage 2: Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 paper2codes && \
    mkdir -p /app/data && \
    chown -R paper2codes:paper2codes /app

# Copy binary from builder
COPY --from=builder /app/target/release/paper2codes /usr/local/bin/paper2codes

# Set working directory
WORKDIR /app

# Switch to non-root user
USER paper2codes

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Default command
CMD ["paper2codes", "api"]

