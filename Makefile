.PHONY: help all clean test build release lint fmt check-fmt markdownlint nixie wasm package e2e

PACKAGE_NAME ?= jmap-tool
WASM_TARGET ?= wasm32-wasip2
WASM_ARTIFACT ?= target/$(WASM_TARGET)/release/jmap_tool.wasm
DIST_DIR ?= dist/$(PACKAGE_NAME)
BUNDLE_ARTIFACT ?= dist/$(PACKAGE_NAME)-$(WASM_TARGET).tar.gz

CARGO ?= cargo
BUILD_JOBS ?=
RUST_FLAGS ?= -D warnings
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
MDLINT ?= markdownlint-cli2
NIXIE ?= nixie

build: target/debug/lib$(subst -,_,$(PACKAGE_NAME)).rlib ## Build debug library
release: target/release/lib$(subst -,_,$(PACKAGE_NAME)).rlib ## Build release library

all: check-fmt lint test ## Perform a comprehensive check of code

clean: ## Remove build artifacts
	$(CARGO) clean

test: ## Run tests with warnings treated as errors
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) test $(TEST_FLAGS) $(BUILD_JOBS)

target/%/lib$(subst -,_,$(PACKAGE_NAME)).rlib: ## Build library in debug or release mode
	$(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release)

wasm: $(WASM_ARTIFACT) ## Build the release Wasm component

$(WASM_ARTIFACT):
	$(CARGO) rustc --lib --target $(WASM_TARGET) --release --crate-type=cdylib

package: wasm ## Package the Wasm artifact, sidecar, and Ironclaw tar.gz bundle
	rm -rf $(DIST_DIR)
	mkdir -p $(DIST_DIR)
	cp $(WASM_ARTIFACT) $(DIST_DIR)/jmap-tool.wasm
	cp jmap-tool.capabilities.json $(DIST_DIR)/
	cp docs/users-guide.md $(DIST_DIR)/README.md
	rm -f $(BUNDLE_ARTIFACT)
	tar -C $(DIST_DIR) -czf $(BUNDLE_ARTIFACT) \
		jmap-tool.wasm \
		jmap-tool.capabilities.json \
		README.md

e2e: wasm ## Run rusmes-jmap-backed end-to-end tests
	$(CARGO) test -- --ignored --nocapture

lint: ## Run Clippy with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --no-deps
	$(CARGO) clippy $(CLIPPY_FLAGS)

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
