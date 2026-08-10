#!/usr/bin/env bash
# One-time setup for a repo created from this template. Idempotent — safe to re-run.
#
# This exists because the two setup steps that make the discipline non-optional are
# themselves unenforced: git hooks (local, per-clone) and branch protection
# (server-side, per-repo). Until both are done, every rule in CLAUDE.md is voluntary.
# The script does the mechanical half and prints the judgement half.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

git config core.hooksPath .githooks
echo "hooks       core.hooksPath=.githooks"

# Server-side protection: the local hooks are a fast first line, not the enforcement.
# Requires an authenticated gh with admin on the repo; prints how to do it manually
# otherwise, because a skipped step that says nothing is a step that never happens.
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1 &&
	repo=$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null) &&
	[ -n "$repo" ]; then
	gh api -X PUT "repos/$repo/branches/main/protection" --input - >/dev/null <<'JSON'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["discipline", "discipline-tests", "secrets"]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": null,
  "restrictions": null
}
JSON
	echo "protection  main requires: discipline, discipline-tests, secrets (admins included)"
else
	echo "protection  SKIPPED — gh missing, unauthenticated, or no GitHub remote." >&2
	echo "            Set it manually: require the discipline, discipline-tests and" >&2
	echo "            secrets checks on main before merging anything." >&2
fi

# The rest is adaptation, not mechanics — point at it rather than pretending.
if [ -f north-star.md ]; then
	echo "north-star  found — first session: turn it into PLAN.md and STAGE-1-PROMPTS.md"
	echo "            (Plan bootstrap brief, SUBAGENT-BRIEFS.md)"
else
	echo "north-star  none — add north-star.md, or write PLAN.md directly"
fi

remaining=$(grep -lE '<[A-Za-z][^>]*>' CLAUDE.md PLAN.md STAGE-1-PROMPTS.md 2>/dev/null |
	tr '\n' ' ' || true)
if [ -n "$remaining" ]; then
	echo "todo        placeholders remain in: $remaining"
fi
echo "todo        adaptation checklist at the bottom of METHOD.md — STAGES and"
echo "            SOURCE_DIR in scripts/check-docs.sh, SOURCE_DIR in"
echo "            scripts/measure-ratchets.sh, Makefile bodies, ratchet baselines"
