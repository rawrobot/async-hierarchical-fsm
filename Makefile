.PHONY: help build test test-all coverage coverage-open clean fmt clippy clippy-pedantic check examples docs install-deps

# Default target
help:
	@echo "🚀 Async Hierarchical FSM - Available Commands:"
	@echo ""
	@echo "📦 Building:"
	@echo "  build          - Build the project"
	@echo "  build-release  - Build in release mode"
	@echo "  build-all      - Build with all features"
	@echo ""
	@echo "🧪 Testing:"
	@echo "  test           - Run unit tests"
	@echo "  test-all       - Run all tests with all features"
	@echo "  test-integration - Run integration tests only"
	@echo ""
	@echo "📊 Coverage:"
	@echo "  coverage       - Generate test coverage report"
	@echo "  coverage-open  - Generate coverage and open in browser"
	@echo "  coverage-ci    - Generate coverage for CI (XML output)"
	@echo ""
	@echo "🔍 Code Quality:"
	@echo "  check          - Run cargo check"
	@echo "  clippy         - Run clippy lints (strict)"
	@echo "  clippy-pedantic - Run clippy with pedantic lints"
	@echo "  fmt            - Format code"
	@echo "  fmt-check      - Check code formatting"
	@echo ""
	@echo "📚 Documentation:"
	@echo "  docs           - Generate documentation"
	@echo "  docs-open      - Generate docs and open in browser"
	@echo ""
	@echo "🎯 Examples:"
	@echo "  examples       - Run all examples"
	@echo "  example-basic  - Run basic device example"
	@echo "  example-ui     - Run hierarchical UI example"
	@echo ""
	@echo "🛠️  Utilities:"
	@echo "  install-deps   - Install required dependencies"
	@echo "  clean          - Clean build artifacts"
	@echo "  clean-all      - Clean everything including coverage"

# Build targets
build:
	@echo "🔨 Building project..."
	cargo build

build-release:
	@echo "🔨 Building project in release mode..."
	cargo build --release

build-all:
	@echo "🔨 Building project with all features..."
	cargo build --all-features

# Test targets
test:
	@echo "🧪 Running unit tests..."
	cargo test

test-all:
	@echo "🧪 Running all tests with all features..."
	cargo test --all-features

test-integration:
	@echo "🧪 Running integration tests..."
	cargo test --test integration_tests --all-features

# Coverage targets
coverage:
	@echo "📊 Generating test coverage..."
	@./ws-coverage.sh

coverage-open: coverage
	@echo "🌐 Opening coverage report in browser..."
	@if command -v xdg-open > /dev/null; then \
		xdg-open coverage/tarpaulin-report.html; \
	elif command -v open > /dev/null; then \
		open coverage/tarpaulin-report.html; \
	else \
		echo "Please open coverage/tarpaulin-report.html manually"; \
	fi

coverage-ci:
	@echo "📊 Generating coverage for CI..."
	@rm -rf coverage
	@mkdir -p coverage
	@if ! command -v cargo-tarpaulin &> /dev/null; then \
		echo "Installing cargo-tarpaulin..."; \
		cargo install cargo-tarpaulin; \
	fi
	cargo tarpaulin \
		--workspace \
		--out Xml \
		--output-dir coverage \
		--skip-clean \
		--timeout 600 \
		--no-fail-fast

# Code quality targets
check:
	@echo "🔍 Running cargo check..."
	cargo check --all-features

clippy:
	@echo "🔍 Running clippy (strict)..."
	cargo clippy --all-features -- -D warnings

clippy-pedantic:
	@echo "🔍 Running clippy with pedantic lints..."
	cargo clippy --all-features -- -W clippy::pedantic

fmt:
	@echo "🎨 Formatting code..."
	cargo fmt

fmt-check:
	@echo "🎨 Checking code formatting..."
	cargo fmt -- --check

# Documentation targets
docs:
	@echo "📚 Generating documentation..."
	cargo doc --all-features --no-deps

docs-open:
	@echo "📚 Generating documentation and opening in browser..."
	cargo doc --all-features --no-deps --open

# Example targets
examples:
	@echo "🎯 Running all examples..."
	@echo "Running basic device example..."
	cargo run --example basic_device --features "plantuml,tokio-integration"
	@echo ""
	@echo "Running hierarchical UI example..."
	cargo run --example hierarchical_ui --all-features

example-basic:
	@echo "🎯 Running basic device example..."
	cargo run --example basic_device --features "plantuml,tokio-integration"

example-ui:
	@echo "🎯 Running hierarchical UI example..."
	cargo run --example hierarchical_ui --all-features

# Utility targets
install-deps:
	@echo "🛠️  Installing required dependencies..."
	@if ! command -v cargo-tarpaulin &> /dev/null; then \
		echo "Installing cargo-tarpaulin..."; \
		cargo install cargo-tarpaulin; \
	fi
	@if ! command -v jq &> /dev/null; then \
		echo "⚠️  jq not found. Please install jq for coverage statistics."; \
		echo "   Ubuntu/Debian: sudo apt-get install jq"; \
		echo "   macOS: brew install jq"; \
		echo "   Arch: sudo pacman -S jq"; \
	fi
	@if ! command -v bc &> /dev/null; then \
		echo "⚠️  bc not found. Please install bc for coverage calculations."; \
		echo "   Ubuntu/Debian: sudo apt-get install bc"; \
		echo "   macOS: brew install bc"; \
		echo "   Arch: sudo pacman -S bc"; \
	fi

clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean

clean-all: clean
	@echo "🧹 Cleaning everything..."
	rm -rf coverage/
	rm -rf target/doc/

# CI/CD targets
ci-test: install-deps test-all clippy fmt-check

ci-coverage: install-deps coverage-ci

# Development workflow
dev-check: fmt clippy-pedantic test

# Release workflow
release-check: clean build-release test-all clippy fmt-check docs

# Quick development cycle
quick: fmt test

# Full development cycle
full: clean fmt clippy test-all coverage docs examples