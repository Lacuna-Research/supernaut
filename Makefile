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

# --- Replace the bodies below with this project's toolchain. ---------------
# Keep warnings-as-errors at the build invocation: zero-warnings is a rule, so the
# compiler should be the thing that enforces it rather than a review comment.

build:
	@echo "TODO: build" && false

test:
	@echo "TODO: test" && false

fmt:
	@echo "TODO: format in place" && false

lint:
	@echo "TODO: format lint --strict" && false
