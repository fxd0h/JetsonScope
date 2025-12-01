SHELL := /bin/bash
.PHONY: help build install service vendor clean test

help:
	@echo "Tegrastats TUI - Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  build        - Build all binaries (release)"
	@echo "  install      - Install binaries to /usr/local/bin"
	@echo "  service      - Install systemd service"
	@echo "  package      - Genera tarball para Jetson"
	@echo "  docker-build - Build Docker image"
	@echo "  docker-run   - Run Docker image (daemon)"
	@echo "  vendor       - Vendor dependencies for offline build"
	@echo "  test         - Run tests"
	@echo "  clean        - Clean build artifacts"

build:
	@echo "📦 Building release binaries..."
	cargo build --release --all-features --locked

install: build
	@echo "📥 Installing binaries to /usr/local/bin..."
	sudo cp target/release/jscope /usr/local/bin/jscope
	sudo cp target/release/jscoped /usr/local/bin/jscoped
	sudo cp target/release/jscopectl /usr/local/bin/jscopectl
	sudo chmod +x /usr/local/bin/jscope
	sudo chmod +x /usr/local/bin/jscoped
	sudo chmod +x /usr/local/bin/jscopectl
	@echo "✅ Binaries installed"

service:
	@echo "📋 Installing systemd service..."
	sudo cp install/jscoped.service /etc/systemd/system/
	sudo systemctl daemon-reload
	sudo systemctl enable jscoped
	@echo "✅ Service installed (use 'sudo systemctl start jscoped' to start)"

package:
	@echo "📦 Generating Jetson tarball..."
	packaging/jetson/pack.sh
	@echo "✅ Tarball listo en target/"

docker-build:
	@echo "🐳 Building Docker image..."
	docker build -t jetsonscope .
	@echo "✅ Docker image 'jetsonscope' built"

docker-run:
	@echo "🐳 Running Docker image (TUI by default)..."
	docker run --rm -it jetsonscope

scripts-check:
	@echo "🔍 Checking scripts (executable bit)..."
	chmod +x scripts/*.sh
	@echo "✅ Scripts ready"

vendor:
	@echo "📦 Vendoring dependencies..."
	mkdir -p .cargo
	cargo vendor > .cargo/config.toml
	@echo "✅ Dependencies vendored to ./vendor/"
	@echo "   To build offline: cargo build --offline --release"

test:
	@echo "🧪 Running tests..."
	cargo test

clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@echo "✅ Clean complete"
