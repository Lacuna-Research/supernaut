.PHONY: all bootstrap hooks check check-tests build test fmt lint

# What CI runs.
all: build test lint check check-tests

# One-time project setup: hooks, branch protection, and what remains. Idempotent.
bootstrap:
	./scripts/bootstrap.sh

# Install the git hooks only. bootstrap runs this too; kept for clones of an
# already-bootstrapped repo, where protection is set and only the hooks are missing.
hooks:
	git config core.hooksPath .githooks
	@echo "Hooks installed (core.hooksPath=.githooks)"

# Documentation discipline over staged changes. CI runs the same script with a base ref.
check:
	./scripts/check-docs.sh

# The enforcement script's own test suite. A broken check fails open — silently —
# so the checks are tested like any other code. Run whenever check-docs.sh changes.
check-tests:
	./scripts/test-checks.sh

# --- Toolchain: Rust stable. -----------------------------------------------
# Warnings-as-errors lives in [workspace.lints] in Cargo.toml, so the rule travels
# with the repo rather than the shell; these bodies stay plain cargo invocations.

build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
