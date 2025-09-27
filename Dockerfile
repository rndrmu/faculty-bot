# =================================================================
# Builder Stage - Thanks to ChatGPT for updating this <3 <3 <3
# =================================================================
FROM rust:1.90-bullseye as builder

# Set a working directory
WORKDIR /faculty_manager

# (Optional) Install cmake only if a dependency truly needs it
RUN apt-get update && apt-get install -y cmake && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/deps/faculty_manager*

# Build the actual application
COPY . .
RUN cargo build --release

# =================================================================
# Final Runtime Stage
# =================================================================
FROM debian:bullseye-slim

# Install runtime dependencies in a single, clean layer
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates \
      graphicsmagick \
      imagemagick \
      ghostscript && \
    rm -rf /var/lib/apt/lists/*

# Note: The path to policy.xml might differ in newer ImageMagick versions.
# This assumes ImageMagick 6 is still used in Bookworm. Verify if issues arise.
RUN mv /etc/ImageMagick-6/policy.xml /etc/ImageMagick-6/policy.xml.off || true

# Copy the compiled binary from the builder stage
COPY --from=builder /faculty_manager/target/release/faculty_manager /usr/local/bin/faculty_manager

# Set the command to run the application
CMD ["/usr/local/bin/faculty_manager"]