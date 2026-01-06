# Stage 1: Builder
FROM rust:slim AS builder

# Install build dependencies (g++ needed for some native dependencies)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files and build dependencies to cache them
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
# Build dependencies (this layer will be cached)
RUN cargo build --release
RUN rm -f src/main.rs

# Copy the actual source code
COPY src ./src

# Build the binary in release mode
# We touch main.rs to ensure it's rebuilt since we replaced the dummy one
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create data directory
RUN mkdir -p /data

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/memory-mcp /usr/local/bin/memory-mcp

# Set environment variables
ENV DATA_DIR=/data
ENV LOG_LEVEL=info

# Use non-root user for security (optional but recommended)
# RUN useradd -m appuser
# USER appuser

# Entrypoint
ENTRYPOINT ["memory-mcp"]
CMD ["--data-dir", "/data"]
