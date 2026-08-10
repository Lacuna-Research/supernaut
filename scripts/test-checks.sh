#!/usr/bin/env bash
# Test suite for scripts/check-docs.sh.
#
# The enforcement script is unowned code that only grows, and a broken check fails
# open — silently. Every rule gets two fixtures here: one that triggers it and one
# that clears it. A new check in check-docs.sh lands in the same PR as its fixtures,
# and section 16's project-specific checks are not exempt.
#
# Each test builds a synthetic repo in a temp dir, stages a change, and asserts the
# script's exit code and (on failure) that the message names the right rule — a check
# that fails for the wrong reason is a check that will not fail for the right one.
#
# Usage: ./scripts/test-checks.sh
#
# No -e, deliberately: the suite's whole job is running a script that exits nonzero
# and asserting on the code. Failures are counted, not fatal.
set -uo pipefail

TEMPLATE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

pass=0
failures=0

# ---------------------------------------------------------------------------
# Fixture builder: a minimal, fully-green repo with a 3-prompt stage at 2/3.
# Tests mutate this baseline; keeping it green is itself the first test.
# ---------------------------------------------------------------------------
build_repo() {
	repo="$WORK/repo-$RANDOM$RANDOM"
	mkdir -p "$repo"
	cd "$repo" || exit 1
	git init -q .
	git config user.email test@example.invalid
	git config user.name test

	mkdir -p scripts src
	cp "$TEMPLATE_ROOT/scripts/check-docs.sh" scripts/
	cp "$TEMPLATE_ROOT/scripts/measure-ratchets.sh" scripts/
	chmod +x scripts/*.sh
	# The fixture stage is 3 prompts; the template default is a placeholder.
	sed -i.bak 's|"1:STAGE-1-PROMPTS.md:11"|"1:STAGE-1-PROMPTS.md:3"|' scripts/check-docs.sh
	rm scripts/check-docs.sh.bak

	seq 1 50 | sed 's/^/rule /' >CLAUDE.md

	cat >STAGE-1-PROMPTS.md <<'EOF'
# Stage 1 — The Prompts

**Status:** 2/3 complete. Next: prompt 3.

## Prompt 1 — Scaffold

**Item:** Scaffold

## Prompt 2 — Parser

**Item:** Message parser

## Prompt 3 — Transport

**Item:** Transport

**Carry-forward** *(consumed when this prompt runs)*

- From prompt 2: a note.
EOF

	cat >PLAN.md <<'EOF'
# Plan

### Still open

- **A question?** *(not blocking)* words.

## Stage 1 — Basic

1. **Scaffold**. words
2. **Message parser**. words
3. **Transport**. words

## Stage 2 — Next
EOF

	cat >README.md <<'EOF'
# Proj

![](https://img.shields.io/badge/stage%201-2%2F3-blue)

### Stage 1

| # | Prompt | Status |
|---|---|---|
| 1 | Scaffold | ✅ done |
| 2 | Parser | ✅ done |
| 3 | Transport | ⬜ todo |

### Stage 2
EOF

	cat >BUILD-LOG.md <<'EOF'
# Build Log

## Prompt 1 — Scaffold

**Shipped:** the skeleton.
**Learned:** nothing yet.
**Live run:** launched it.
EOF

	printf 'x\n' >src/main.txt
	git add -A
	git commit -qm baseline
}

# stage_all: stage the current tree so the script sees it as the pending change.
stage_all() { git add -A; }

# expect <pass|fail> <name> [message-fragment]
expect() {
	want=$1
	name=$2
	fragment=${3:-}
	out=$(./scripts/check-docs.sh 2>&1)
	code=$?
	if [ "$want" = pass ] && [ "$code" -eq 0 ]; then
		pass=$((pass + 1))
		printf '  ok    %s\n' "$name"
	elif [ "$want" = fail ] && [ "$code" -ne 0 ]; then
		# Substring match in pure shell rather than `printf | grep -q`: grep -q exits
		# at the first match, printf can catch SIGPIPE, and pipefail then reports 141 —
		# a timing-dependent false failure that this suite flaked on exactly once.
		hit=0
		case "$out" in *"$fragment"*) hit=1 ;; esac
		if [ -n "$fragment" ] && [ "$hit" -eq 0 ]; then
			failures=$((failures + 1))
			printf '  FAIL  %s — failed, but not for the expected reason\n' "$name"
			printf '%s\n' "$out" | sed 's/^/        /'
		else
			pass=$((pass + 1))
			printf '  ok    %s\n' "$name"
		fi
	else
		failures=$((failures + 1))
		printf '  FAIL  %s — expected %s, exit code %s\n' "$name" "$want" "$code"
		printf '%s\n' "$out" | sed 's/^/        /'
	fi
}

printf 'check-docs.sh test suite\n'

# --- Baseline -------------------------------------------------------------
build_repo
printf 'y\n' >>src/main.txt
cat >>BUILD-LOG.md <<'EOF'

## Prompt 2b — touch

**Shipped:** a line.
**Learned:** None.
**Live run:** ran it.
EOF
stage_all
expect pass "baseline repo is green"

# --- 1. CLAUDE.md line cap ------------------------------------------------
build_repo
seq 1 120 | sed 's/^/rule /' >>CLAUDE.md
stage_all
expect fail "CLAUDE.md over cap fails" "cap 100"

# --- 2. Append-only log ---------------------------------------------------
build_repo
sed -i.bak 's/the skeleton/rewritten history/' BUILD-LOG.md && rm BUILD-LOG.md.bak
stage_all
expect fail "editing a past log entry fails" "append-only"

# --- 3. Substantive build-log entry ---------------------------------------
build_repo
printf 'y\n' >>src/main.txt
stage_all
expect fail "src change with no log entry fails" "BUILD-LOG.md did not"

build_repo
printf 'y\n' >>src/main.txt
cat >>BUILD-LOG.md <<'EOF'

## Prompt 2b — touch

fixed stuff
EOF
stage_all
expect fail "hollow log entry fails" "missing section"

# --- 4. Carry-forward consumption is recorded -----------------------------
build_repo
# Consume the note on prompt 3 without recording it.
sed -i.bak '/Carry-forward/,$d' STAGE-1-PROMPTS.md && rm STAGE-1-PROMPTS.md.bak
stage_all
expect fail "silent carry-forward deletion fails" "Carry-forward consumed"

build_repo
sed -i.bak '/Carry-forward/,$d' STAGE-1-PROMPTS.md && rm STAGE-1-PROMPTS.md.bak
cat >>BUILD-LOG.md <<'EOF'

## Prompt 3 — Transport (partial)

**Carry-forward consumed:** the note from prompt 2, applied to the timer.
EOF
stage_all
expect pass "recorded carry-forward deletion passes"

# --- 5/6. Status line and README agreement --------------------------------
build_repo
sed -i.bak 's/^\*\*Status:.*/Status: broken/' STAGE-1-PROMPTS.md && rm STAGE-1-PROMPTS.md.bak
stage_all
expect fail "missing status line fails" "needs a line"

build_repo
sed -i.bak 's/2%2F3/1%2F3/' README.md && rm README.md.bak
stage_all
expect fail "stale badge fails" "badge disagrees"

build_repo
sed -i.bak 's/| 2 | Parser | ✅ done |/| 2 | Parser | ⬜ todo |/' README.md && rm README.md.bak
stage_all
expect fail "stale progress table fails" "progress table"

# --- 7. Stale carry-forward (position, not number) ------------------------
build_repo
sed -i.bak 's|\*\*Status:\*\* 2/3 complete. Next: prompt 3.|**Status:** 3/3 complete.|' STAGE-1-PROMPTS.md && rm STAGE-1-PROMPTS.md.bak
sed -i.bak -e 's/2%2F3/3%2F3/' -e 's/| 3 | Transport | ⬜ todo |/| 3 | Transport | ✅ done |/' README.md && rm README.md.bak
cat >>BUILD-LOG.md <<'EOF'

## Retrospective — Stage 1

Pruned.
EOF
stage_all
expect fail "note outliving its prompt fails" "Unconsumed carry-forward"

# --- 8. Blocking open question gates the next prompt ----------------------
build_repo
sed -i.bak 's/\*(not blocking)\*/*(blocking: prompt 3)*/' PLAN.md && rm PLAN.md.bak
stage_all
expect fail "blocked next prompt fails" "blocking prompt 3"

build_repo
sed -i.bak 's/\*(not blocking)\*/*(blocking: prompt 2)*/' PLAN.md && rm PLAN.md.bak
stage_all
expect pass "question blocking a non-next prompt passes"

# --- 9. Plan attachment ---------------------------------------------------
build_repo
sed -i.bak 's/3. \*\*Transport\*\*. words/3. **Transport layer**. words/' PLAN.md && rm PLAN.md.bak
stage_all
expect fail "unattached plan item fails" "not attached"

# --- 10. Stage retrospective ----------------------------------------------
build_repo
sed -i.bak -e 's|\*\*Status:\*\* 2/3 complete. Next: prompt 3.|**Status:** 3/3 complete.|' \
	-e '/Carry-forward/,$d' STAGE-1-PROMPTS.md && rm STAGE-1-PROMPTS.md.bak
sed -i.bak -e 's/2%2F3/3%2F3/' -e 's/| 3 | Transport | ⬜ todo |/| 3 | Transport | ✅ done |/' README.md && rm README.md.bak
cat >>BUILD-LOG.md <<'EOF'

## Prompt 3 — Transport

**Carry-forward consumed:** the note from prompt 2.
EOF
stage_all
expect fail "finished stage without retrospective fails" "Retrospective"

cat >>BUILD-LOG.md <<'EOF'

## Retrospective — Stage 1

CLAUDE.md pruned to 50 lines; docs audited; cold-start drill run, no gaps.
EOF
stage_all
expect pass "finished stage with retrospective passes"

# --- 11. Oversize prompt --------------------------------------------------
build_repo
seq 1 900 >src/big.txt
cat >>BUILD-LOG.md <<'EOF'

## Prompt 2b — big

**Shipped:** a lot.
**Learned:** None.
**Live run:** ran it.
EOF
stage_all
expect fail "oversize change without justification fails" "Oversize"

cat >>BUILD-LOG.md <<'EOF'
**Oversize:** generated fixture corpus; mechanical, reviewed by sampling.
EOF
stage_all
expect pass "justified oversize change passes"

# --- 12. Correction entries close the loop --------------------------------
build_repo
cat >>BUILD-LOG.md <<'EOF'

## Correction — the cache is not relocatable

**Claimed:** it was. **Actually:** it is not.
EOF
stage_all
expect fail "correction without mechanization answer fails" "Mechanized as"

cat >>BUILD-LOG.md <<'EOF'
**Not mechanizable because:** one-time environmental fact, now recorded.
EOF
stage_all
expect pass "correction with an answer passes"

# --- 13. Failure-mode register --------------------------------------------
build_repo
cat >>BUILD-LOG.md <<'EOF'

## Correction — one
**Category:** asserted-not-tested
**Not mechanizable because:** judgement.

## Correction — two
**Category:** asserted-not-tested
**Not mechanizable because:** judgement.

## Correction — three
**Category:** asserted-not-tested
**Not mechanizable because:** judgement.
EOF
stage_all
expect fail "third category occurrence without a rule fails" "Rule raised"

cat >>BUILD-LOG.md <<'EOF'

## Correction — four
**Category:** asserted-not-tested
**Not mechanizable because:** judgement.
**Rule raised:** CLAUDE.md now says: test the limit before designing around it.
EOF
stage_all
expect pass "category with a raised rule passes"

# --- 14. Ratchets ---------------------------------------------------------
build_repo
./scripts/measure-ratchets.sh >/dev/null # sanity: it runs
cat >ratchets.txt <<'EOF'
todo-count 0
longest-file 5
EOF
stage_all
expect pass "ratchets at or under ceiling pass"

printf 'TODO one\nTODO two\n' >>src/main.txt
cat >>BUILD-LOG.md <<'EOF'

## Prompt 2b — touch

**Shipped:** todos. **Learned:** None. **Live run:** ran it.
EOF
stage_all
expect fail "worsened ratchet fails" "worsened"


# --- 15. Failure ledger (passive telemetry) --------------------------------
build_repo
seq 1 120 | sed 's/^/rule /' >>CLAUDE.md
stage_all
./scripts/check-docs.sh >/dev/null 2>&1
if grep -qE "claude-cap" discipline-stats.txt 2>/dev/null; then
	pass=$((pass + 1)); printf '  ok    %s\n' "local failure lands in the ledger with its rule id"
else
	failures=$((failures + 1)); printf '  FAIL  %s\n' "local failure lands in the ledger with its rule id"
fi
before=$(wc -l <discipline-stats.txt)
./scripts/check-docs.sh >/dev/null 2>&1
after=$(wc -l <discipline-stats.txt)
if [ "$before" -eq "$after" ]; then
	pass=$((pass + 1)); printf '  ok    %s\n' "re-running an unfixed failure does not inflate the ledger"
else
	failures=$((failures + 1)); printf '  FAIL  %s — %s then %s lines\n' "re-running an unfixed failure does not inflate the ledger" "$before" "$after"
fi
./scripts/check-docs.sh origin/nonexistent >/dev/null 2>&1 || true
after_ci=$(wc -l <discipline-stats.txt)
if [ "$after" -eq "$after_ci" ]; then
	pass=$((pass + 1)); printf '  ok    %s\n' "CI mode does not write the ledger"
else
	failures=$((failures + 1)); printf '  FAIL  %s\n' "CI mode does not write the ledger"
fi

# --- 16. README progress apparatus is opt-in -------------------------------
build_repo
cat >README.md <<'EOF2'
# Proj

A readme with no progress claims at all.
EOF2
stage_all
expect pass "README without a badge skips the agreement checks"

build_repo
sed -i.bak 's/2%2F3/1%2F3/' README.md && rm README.md.bak
stage_all
expect fail "README with a stale badge still fails" "badge disagrees"

# --- Summary ---------------------------------------------------------------
printf '\n%d passed, %d failed\n' "$pass" "$failures"
[ "$failures" -eq 0 ]
