# edgefirst-v4l2 developer interface. CI runs the same checks through the
# shared EdgeFirstAI/.github workflows pinned in .github/workflows/ci.yml.

.PHONY: help format lint build test sbom verify-version pre-release clean

help: ## Show available targets (default)
	@grep -E '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-15s %s\n", $$1, $$2}'

format: ## Format all source code
	cargo fmt --all

lint: ## Run all linters with zero warnings
	cargo fmt --all -- --check
	cargo clippy --all-targets --locked -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked

build: ## Build with coverage instrumentation
	cargo llvm-cov --no-report build --locked

test: ## Run all tests with coverage
	cargo llvm-cov --locked --lcov --output-path target/coverage.lcov

sbom: ## Generate sbom/sbom.json and check the license policy
	@CI_SCRIPTS=$$(.github/scripts/fetch-ci-scripts.sh); \
		PROJECT_NAME=edgefirst-v4l2 PROJECT_TYPE=library VERSION_FILE=Cargo.toml \
		SOURCE_DIRS="src" SBOM_MODE=$${SBOM_MODE:-dependency} \
		OUTPUT_DIR=sbom SCRIPT_DIR="$$CI_SCRIPTS" \
		"$$CI_SCRIPTS/generate_sbom.sh"

verify-version: ## Check Cargo.toml and Cargo.lock agree on the version (release.yml also checks CHANGELOG.md)
	@version=$$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1); \
		echo "Cargo.toml version: $$version"; \
		grep -A1 '^name = "edgefirst-v4l2"' Cargo.lock | grep -q "version = \"$$version\"" \
			|| { echo "Cargo.lock does not record $$version"; exit 1; }; \
		echo "✓ Cargo.lock matches"

pre-release: lint test sbom verify-version ## Complete pre-release validation
	@echo "✓ Pre-release checks passed"

clean: ## Remove build artifacts
	cargo clean
	rm -rf sbom sbom.json *.cdx.json
