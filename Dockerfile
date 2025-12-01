## Build stage (mimics original jtop Docker intent: simple image with tool prebuilt)
# Use nightly to support crates requiring edition2024.
FROM rustlang/rust:nightly-bullseye AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --locked --all-features

## Runtime stage (slim, ships binaries; defaults to TUI like original jtop)
FROM debian:bullseye-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl1.1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/jscope /usr/local/bin/jscope
COPY --from=builder /app/target/release/jscoped /usr/local/bin/jscoped
COPY --from=builder /app/target/release/jscopectl /usr/local/bin/jscopectl

# Default socket inside container
ENV JETSONSCOPE_SOCKET_PATH=/tmp/jetsonscope.sock

# Entry shim: run TUI by default; allow MODE=daemon|cli overrides
RUN printf '#!/bin/sh\nset -e\nMODE=\"${MODE:-tui}\"\ncase \"$MODE\" in\n  daemon) exec /usr/local/bin/jscoped ;;\n  cli) exec /usr/local/bin/jscopectl \"$@\" ;;\n  *) exec /usr/local/bin/jscope ;;\nesac\n' > /usr/local/bin/entrypoint.sh && chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
