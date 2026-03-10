.PHONY: help all clean test build release lint fmt check-fmt markdownlint nixie typecheck wasm package e2e unit bdd

PACKAGE_NAME ?= jmap-tool
TARGET ?= libjmap_tool.rlib
WASM_TARGET ?= wasm32-wasip2
WASM_ARTIFACT ?= target/$(WASM_TARGET)/release/jmap_tool.wasm
DIST_DIR ?= dist/$(PACKAGE_NAME)
BUNDLE_NAME ?= $(patsubst %-tool,%,$(patsubst %_tool,%,$(PACKAGE_NAME)))
BUNDLE_ARTIFACT ?= dist/$(BUNDLE_NAME)-$(WASM_TARGET).tar.gz
LEGACY_BUNDLE_ARTIFACT ?= dist/$(PACKAGE_NAME)-$(WASM_TARGET).tar.gz

CARGO ?= cargo
BUILD_JOBS ?=
RUST_FLAGS ?=
RUST_FLAGS := -D warnings $(RUST_FLAGS)
RUSTDOC_FLAGS ?=
RUSTDOC_FLAGS := -D warnings $(RUSTDOC_FLAGS)
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
TEST_CMD := $(if $(shell $(CARGO) nextest --version 2>/dev/null),nextest run,test)
MDLINT ?= markdownlint-cli2
NIXIE ?= nixie

build: target/debug/$(TARGET) ## Build debug library
release: target/release/$(TARGET) ## Build release library

all: check-fmt lint test ## Perform a comprehensive check of code

clean: ## Remove build artifacts
	$(CARGO) clean

unit: ## Run unit-style Rust tests
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) $(TEST_CMD) $(TEST_FLAGS) $(BUILD_JOBS)

bdd: ## Run behavioural tests
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) test $(BUILD_JOBS) tests_bdd:: -- --nocapture

test: unit bdd e2e ## Run all tests with warnings treated as errors

target/%/$(TARGET): ## Build library in debug or release mode
	$(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release) --lib

wasm: ## Build the release Wasm component
	$(CARGO) rustc --lib --target $(WASM_TARGET) --release --crate-type=cdylib

package: wasm ## Package the Wasm artifact, sidecar, and Ironclaw tar.gz bundle
	rm -rf $(DIST_DIR)
	mkdir -p $(DIST_DIR)
	cp $(WASM_ARTIFACT) $(DIST_DIR)/jmap-tool.wasm
	cp jmap-tool.capabilities.json $(DIST_DIR)/
	cp docs/users-guide.md $(DIST_DIR)/README.md
	rm -f $(LEGACY_BUNDLE_ARTIFACT)
	rm -f $(BUNDLE_ARTIFACT)
	tar -C $(DIST_DIR) -czf $(BUNDLE_ARTIFACT) \
		jmap-tool.wasm \
		jmap-tool.capabilities.json \
		README.md

e2e: wasm ## Run rusmes-jmap-backed end-to-end tests
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) test $(BUILD_JOBS) -- --ignored --nocapture

lint: ## Run Clippy with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --no-deps
	$(CARGO) clippy $(CLIPPY_FLAGS)
	RUSTFLAGS="$(RUST_FLAGS)" whitaker --all -- $(CARGO_FLAGS)

typecheck: ## Type-check without building
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) check $(CARGO_FLAGS)

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	mdformat-all

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check

markdownlint: ## Lint Markdown files
	$(MDLINT) '**/*.md'

nixie: ## Validate Mermaid diagrams
	$(NIXIE) --no-sandbox

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
