default:
	@just --list

# Unit tests
test:
	cargo test

# uinput integration tests (input/uinput groups required)
test-integration:
	cargo test -- --ignored

# Strict lint (all targets, tests included)
lint:
	cargo clippy --all-targets -- -D warnings

# Formatting check
fmt:
	cargo fmt --check

# Auto-format
fmt-fix:
	cargo fmt

# Hexagonal dependency rule
arch:
	./scripts/check-architecture.sh

# Documentation link check (requires network)
links:
	./scripts/check-links.sh

# English-only codebase guard
lang:
	./scripts/check-language.sh

# Install versioned git hooks (one-time per clone)
setup:
	git config core.hooksPath .githooks


# Release build
build:
	cargo build --release

# Install binaries to ~/.local/bin
install-user:
	cargo build --release -p trinity-daemon -p trinity-gui
	install -Dm755 target/release/trinity-daemon ~/.local/bin/trinity-daemon
	install -Dm755 target/release/trinity-gui ~/.local/bin/trinity-gui
	install -Dm755 target/release/trinity-ctl ~/.local/bin/trinity-ctl

# Remove dev-installed binaries
uninstall:
	rm -f ~/.local/bin/trinity-daemon ~/.local/bin/trinity-gui ~/.local/bin/trinity-ctl

# Run the engine (before the GUI)
run-daemon:
	cargo run -p trinity-daemon --bin trinity-daemon

# Run the GUI
run-gui:
	cargo run -p trinity-gui

# Run every check
check: fmt lint test arch lang
