#!/usr/bin/env bash
# Mechanical enforcement of the documentation discipline described in CLAUDE.md.
#
# Every rule here was previously a promise. A promise that a machine does not check
# is a promise that eventually is not kept.
#
# This script has its own test suite: scripts/test-checks.sh. A broken check fails
# open — silently — so any change here must keep the suite green, and any new check
# must arrive with a fixture that triggers it and one that clears it.
#
# Usage:
#   ./scripts/check-docs.sh <base-ref>   # CI: compare against a base branch
#   ./scripts/check-docs.sh              # pre-commit: check staged changes
set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration. Everything project-specific lives here or in the final section.
# ---------------------------------------------------------------------------

CLAUDE_MAX_LINES=100

# Directory whose changes require a BUILD-LOG.md entry.
SOURCE_DIR="src"

# A prompt over this many changed lines (added + removed, within SOURCE_DIR) needs an
# explicit `**Oversize:**` justification in its build-log entry. The number is a
# tripwire, not a target — it exists to force the split conversation at the moment a
# prompt is growing, which is the only moment the conversation is cheap.
MAX_PROMPT_CHANGED_LINES=800

# A recurring failure category (see the Correction template) must become a rule or a
# check by its Nth occurrence.
CATEGORY_LIMIT=3

# One entry per stage: "<stage number>:<prompt file>:<total prompts>".
# Add a line when a stage opens. Do not remove finished stages — their carry-forward,
# status and retrospective checks still have to hold.
STAGES=(
	"1:STAGE-1-PROMPTS.md:11"
	# "2:STAGE-2-PROMPTS.md:18"
)

# Which stage the README's badge and progress table track. Set to the stage in
# progress; a finished stage's badge will not move again.
README_STAGE=1

cd "$(git rev-parse --show-toplevel)"

# Each numbered section sets $rule before its checks; err() records failures against
# it in discipline-stats.txt. Passive telemetry, not enforcement: nothing reads this
# file mechanically. It exists so the retrospective's rule review argues from counts
# instead of memory — "this rule never helps" is otherwise an asserted-not-tested
# claim. Local runs only (CI checkouts are discarded), one line per day/rule/branch
# so re-running against the same unfixed failure does not inflate the count.
rule="unset"
LEDGER=discipline-stats.txt
fail=0
err() {
	printf '  FAIL  %s\n' "$1" >&2
	fail=1
	if [ "$ci_mode" -eq 0 ]; then
		line="$(date +%F) $rule $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"
		grep -qxF "$line" "$LEDGER" 2>/dev/null || printf '%s\n' "$line" >>"$LEDGER"
	fi
}
ok() { printf '  ok    %s\n' "$1"; }

if [ $# -ge 1 ]; then
	ci_mode=1
	base="$1"
	scope="$base...HEAD"
	diff_names() { git diff --name-only "$base...HEAD"; }
	diff_file() { git diff "$base...HEAD" -- "$1"; }
	diff_numstat() { git diff --numstat "$base...HEAD" -- "$1"; }
else
	ci_mode=0
	scope="staged changes"
	diff_names() { git diff --cached --name-only; }
	diff_file() { git diff --cached -- "$1"; }
	diff_numstat() { git diff --cached --numstat -- "$1"; }
fi

# Lines added to a file in this change, with the leading '+' stripped and the
# +++ header excluded. Several checks below reason about what an entry *says*,
# not merely that the file changed.
added_lines() { diff_file "$1" | sed -n '/^+++/!s/^+//p'; }

printf 'Documentation discipline (%s)\n' "$scope"

# ---------------------------------------------------------------------------
# 1. CLAUDE.md must stay short.
#
# Instruction files rot by accretion: rules get appended, never removed, until the
# file is long enough that nothing in it is read carefully. A hard cap forces the
# pruning that good intentions do not.
# ---------------------------------------------------------------------------
rule='claude-cap'
lines=$(wc -l <CLAUDE.md | tr -d ' ')
if [ "$lines" -gt "$CLAUDE_MAX_LINES" ]; then
	err "CLAUDE.md is $lines lines (cap $CLAUDE_MAX_LINES). Prune it. Do not raise the cap."
else
	ok "CLAUDE.md $lines/$CLAUDE_MAX_LINES lines"
fi

# ---------------------------------------------------------------------------
# 2. BUILD-LOG.md is append-only.
#
# A removed line is a rewrite of history. Corrections belong in a new entry.
# ---------------------------------------------------------------------------
rule='log-append-only'
if diff_file BUILD-LOG.md | grep -qE '^-[^-]'; then
	err "BUILD-LOG.md has removed or modified lines. It is append-only; correct in a new entry."
else
	ok "BUILD-LOG.md append-only"
fi

# ---------------------------------------------------------------------------
# 3. Code changes must carry a build-log entry — and the entry must have substance.
#
# "The file changed" was the old rule, and 'fixed stuff' satisfied it. The entry is
# only worth writing if it carries the template's load-bearing sections; Learned and
# Live run are required by name because they are the two that pay for the file
# existing and the two most often skipped.
# ---------------------------------------------------------------------------
rule='log-entry-substance'
if diff_names | grep -qE "^$SOURCE_DIR/"; then
	if ! diff_names | grep -qx 'BUILD-LOG.md'; then
		err "$SOURCE_DIR/ changed but BUILD-LOG.md did not. Record deviations, surprises, measurements."
	else
		log_added=$(added_lines BUILD-LOG.md)
		missing=""
		for section in '**Shipped:**' '**Learned:**' '**Live run:**'; do
			printf '%s\n' "$log_added" | grep -qF "$section" || missing="$missing $section"
		done
		if [ -n "$missing" ]; then
			err "BUILD-LOG.md entry is missing section(s):${missing}. 'None' is a valid body; an absent section is not."
		else
			ok "$SOURCE_DIR/ change has a substantive BUILD-LOG.md entry"
		fi
	fi
fi

# ---------------------------------------------------------------------------
# 4. Consuming a carry-forward note and recording that you did are one act.
#
# Deleting the note used to be the pass condition on its own, which meant a note
# could vanish silently — indistinguishable, later, from a note that was acted on.
# If this change removes a carry-forward block, the same change's build-log entry
# must say what was consumed.
# ---------------------------------------------------------------------------
rule='carry-forward-recorded'
removed_cf=""
for entry in PLAN.md "${STAGES[@]}"; do
	f=$entry
	case "$f" in *:*)
		f=${f#*:}
		f=${f%%:*}
		;;
	esac
	[ -f "$f" ] || continue
	if diff_file "$f" | grep -qE '^-(### Carry-forward|\*\*Carry-forward\*\*)'; then
		removed_cf="$removed_cf $f"
	fi
done
if [ -n "$removed_cf" ]; then
	if added_lines BUILD-LOG.md | grep -qF '**Carry-forward consumed:**'; then
		ok "carry-forward removal is recorded in BUILD-LOG.md"
	else
		err "carry-forward notes removed from${removed_cf} but BUILD-LOG.md's entry has no '**Carry-forward consumed:**' section. Deleting a note and recording it are one act."
	fi
fi

# ---------------------------------------------------------------------------
# 5-10. Per-stage: status line, README agreement, stale carry-forward notes,
#       blocking open questions, plan attachment, and the stage retrospective.
# ---------------------------------------------------------------------------
for entry in "${STAGES[@]}"; do
	IFS=: read -r stage file total <<<"$entry"
	rule='stage-config'
	[ -f "$file" ] || {
		err "$file is listed in STAGES but does not exist"
		continue
	}

	# 5. Completion state must exist and be machine-readable.
	rule='status-line'
	completed=$(sed -n "s/^\*\*Status:\*\* \([0-9]\{1,\}\)\/$total complete.*/\1/p" "$file" | head -1)
	if [ -z "$completed" ]; then
		err "$file needs a line of exactly the form: **Status:** N/$total complete. Next: prompt M."
		continue
	fi
	ok "$file status $completed/$total"

	# 6. The README must agree with the status line — if it makes progress claims.
	#
	# Opt-in by construction: these checks engage only when a stage badge exists in
	# the README. A public front door earns the per-prompt cost of keeping a badge
	# and table current; a private working repo does not, and mandating the apparatus
	# everywhere is exactly the busywork this system is accused of. But once the
	# claims exist they are checked, because a stale badge is worse than none — it is
	# confidently wrong. The skip is printed, never silent.
	rule='readme-agreement'
	if [ "$stage" = "$README_STAGE" ] && [ -f README.md ] &&
		grep -q 'img\.shields\.io/badge/stage' README.md; then
		badge="stage%20${stage}-${completed}%2F${total}"
		if grep -qF "$badge" README.md; then
			ok "README badge matches ($completed/$total)"
		else
			err "README badge disagrees with the status line; expected to contain '$badge'"
		fi

		# Counted within this stage's section, not across the whole file: once there is
		# a second progress table, an unscoped count breaks the first stage's check the
		# moment a row lands in the second.
		done_rows=$(awk -v s="### Stage $stage" '
			index($0, s) == 1 { inside = 1; next }
			/^### Stage /     { inside = 0 }
			inside
		' README.md | grep -cE '^\| *[0-9]+[a-z]* *\|.*\| *✅ done *\|' || true)
		if [ "$done_rows" -eq "$completed" ]; then
			ok "README progress table matches ($done_rows done)"
		else
			err "README progress table shows $done_rows done, status line says $completed"
		fi
	elif [ "$stage" = "$README_STAGE" ]; then
		printf '  skip  README makes no progress claims; nothing to keep in agreement\n'
	fi

	# 7. Carry-forward notes must not outlive the prompt they were addressed to.
	#
	# A note that survives its prompt means the note was never consumed — and a stale
	# note is indistinguishable from a live one to the next session that reads it.
	#
	# **Position, not number.** The status line is a *count* of finished prompts, and a
	# split prompt (13a, 13b) breaks the coincidence that the two were the same.
	# Counting headings in order compares like with like, and works whether or not
	# anything is split.
	rule='stale-carry-forward'
	stale=$(awk -v done="$completed" '
		/^## Prompt [0-9]+[a-z]* / { position += 1; label = $3; next }
		/^### Carry-forward/       { if (position > 0 && position <= done) printf "%s ", label; next }
		/^\*\*Carry-forward\*\*/   { if (position > 0 && position <= done) printf "%s ", label }
	' "$file")
	if [ -n "$stale" ]; then
		err "Unconsumed carry-forward notes on completed prompt(s) in $file: ${stale% }"
	else
		ok "$file: no carry-forward notes outliving their prompt"
	fi

	# 8. The next prompt must not be blocked by an open question.
	#
	# PLAN.md's Still open list marks questions blocking with a machine-readable
	# annotation: *(blocking: prompt 5)* or *(blocking: stage 2 prompt 5)*. Nothing
	# used to stop the blocked prompt from simply starting — and a question run into
	# mid-prompt gets answered by whatever is expedient that session, then shipped as
	# public API the real answer has to migrate.
	rule='blocked-prompt'
	next=$(sed -n 's/^\*\*Status:\*\*.*Next: prompt \([0-9]\{1,\}[a-z]\{0,1\}\).*/\1/p' "$file" | head -1)
	if [ -n "$next" ] && [ -f PLAN.md ]; then
		if grep -qE "blocking:( stage $stage)? prompt $next\b" PLAN.md; then
			err "PLAN.md has an open question blocking prompt $next (stage $stage), which the status line says is next. Answer it — or re-mark it not blocking, as a decision entry — before starting."
		else
			ok "stage $stage: next prompt ($next) is not blocked by an open question"
		fi
	fi

	# 9. Every item in this stage of PLAN.md must be attached to a prompt.
	#
	# A roadmap item nobody scheduled is an item that quietly does not get built.
	# Matches on the item's bolded name, which is why prompts must quote it verbatim.
	if [ -f PLAN.md ]; then
		rule='plan-attachment'
		unattached=$(awk -v s="## Stage $stage " '
			index($0, s) == 1 { inside = 1; next }
			/^## Stage /      { inside = 0 }
			inside
		' PLAN.md |
			sed -n 's/^[0-9]\{1,\}[a-z]\{0,\}\. \*\*\([^*]*\)\*\*.*/\1/p' |
			sed 's/\.$//' |
			while IFS= read -r item; do
				grep -qF "$item" "$file" || printf '%s; ' "$item"
			done)
		if [ -n "$unattached" ]; then
			err "PLAN.md stage $stage items not attached to any prompt: $unattached"
		else
			ok "every stage $stage item is attached to a prompt"
		fi
	fi

	# 10. A finished stage requires a retrospective before anything else lands.
	#
	# The stage boundary is where CLAUDE.md gets pruned, the docs get audited, the
	# cold-start drill runs, and the next stage gets planned — and it is exactly the
	# moment momentum says to skip all four and just start. Failing every commit until
	# the retrospective exists is the cheapest way to make the pause non-optional.
	rule='stage-retro'
	if [ "$completed" = "$total" ]; then
		if grep -qE "^## Retrospective — Stage $stage\b" BUILD-LOG.md; then
			ok "stage $stage retrospective recorded"
		else
			err "stage $stage is complete ($completed/$total) but BUILD-LOG.md has no '## Retrospective — Stage $stage' entry. Write it before starting anything else."
		fi
	fi
done

# ---------------------------------------------------------------------------
# 11. A prompt over the size cap must say so, out loud.
#
# Prompt 1 of the source project grew a whole second prompt inside one session — the
# Do-not fences guarded each prompt's scope, but nothing guarded the prompt itself.
# The cap forces the split conversation while it is still cheap. Oversize is allowed;
# silent oversize is not.
# ---------------------------------------------------------------------------
rule='oversize'
changed=$(diff_numstat "$SOURCE_DIR" | awk '$1 != "-" { n += $1 + $2 } END { print n + 0 }')
if [ "$changed" -gt "$MAX_PROMPT_CHANGED_LINES" ]; then
	if added_lines BUILD-LOG.md | grep -qF '**Oversize:**'; then
		ok "oversize change ($changed lines) is justified in BUILD-LOG.md"
	else
		err "$changed changed lines in $SOURCE_DIR/ (cap $MAX_PROMPT_CHANGED_LINES) with no '**Oversize:**' justification in the BUILD-LOG.md entry. Justify it or split the prompt."
	fi
elif [ "$changed" -gt 0 ]; then
	ok "change size $changed/$MAX_PROMPT_CHANGED_LINES lines"
fi

# ---------------------------------------------------------------------------
# 12. A correction entry must close its own loop.
#
# The governing rule — make it mechanical rather than more emphatic — used to be
# aspirational. Now it is checked at the exact moment it applies: every appended
# `## Correction` entry must say `**Mechanized as:**` (naming the check or hook) or
# `**Not mechanizable because:**` (owning the judgement). Either answer is fine;
# no answer is the thing this forbids.
# ---------------------------------------------------------------------------
rule='correction-loop'
bad_corrections=$(added_lines BUILD-LOG.md | awk '
	/^## / {
		if (open && !closed) bad = bad title "; "
		open = ($0 ~ /^## Correction/); closed = 0
		title = substr($0, 4)
	}
	open && (/\*\*Mechanized as:\*\*/ || /\*\*Not mechanizable because:\*\*/) { closed = 1 }
	END {
		if (open && !closed) bad = bad title "; "
		printf "%s", bad
	}
')
if [ -n "$bad_corrections" ]; then
	err "Correction entries without '**Mechanized as:**' or '**Not mechanizable because:**': ${bad_corrections% }"
elif added_lines BUILD-LOG.md | grep -q '^## Correction'; then
	ok "correction entries close the mechanization loop"
fi

# ---------------------------------------------------------------------------
# 13. A failure category that keeps recurring must become a rule.
#
# Correction entries carry a `**Category:**` tag. One occurrence is an accident, two
# a coincidence; at CATEGORY_LIMIT the pattern is a habit, and a habit gets a rule in
# CLAUDE.md or a check in this script — recorded as a `**Rule raised:**` line on one
# of that category's entries. This is the register that surfaced 'asserting instead
# of testing' in the source project, buried in one handoff entry until then.
# ---------------------------------------------------------------------------
rule='failure-register'
if [ -f BUILD-LOG.md ]; then
	unruled=$(awk -v limit="$CATEGORY_LIMIT" '
		/^## / { cat = "" }
		match($0, /\*\*Category:\*\* */) {
			cat = substr($0, RSTART + RLENGTH)
			sub(/[[:space:]]+$/, "", cat)
			count[cat]++
		}
		cat != "" && /\*\*Rule raised:\*\*/ { raised[cat] = 1 }
		END {
			for (c in count)
				if (count[c] >= limit && !(c in raised)) printf "%s (x%d); ", c, count[c]
		}
	' BUILD-LOG.md)
	if [ -n "$unruled" ]; then
		err "Failure categor(ies) at $CATEGORY_LIMIT+ occurrences with no '**Rule raised:**' entry: ${unruled% }. A recurring failure gets a rule or a check."
	else
		ok "no failure category recurring without a rule"
	fi
fi

# ---------------------------------------------------------------------------
# 14. Ratchets only turn one way.
#
# ratchets.txt records ceilings for slow-rot metrics — TODO count, longest file,
# whatever measure-ratchets.sh emits. No single PR makes any of them noticeably
# worse, which is exactly why no per-PR review catches the trend. A worsened value
# fails; an improved one invites tightening the recorded ceiling in the same PR.
# ---------------------------------------------------------------------------
rule='ratchet'
if [ -f ratchets.txt ] && [ -x scripts/measure-ratchets.sh ]; then
	current=$(./scripts/measure-ratchets.sh)
	while read -r key ceiling; do
		[ -n "$key" ] || continue
		case "$key" in \#*) continue ;; esac
		value=$(printf '%s\n' "$current" | awk -v k="$key" '$1 == k { print $2 }')
		if [ -z "$value" ]; then
			err "ratchets.txt lists '$key' but measure-ratchets.sh does not emit it"
		elif [ "$value" -gt "$ceiling" ]; then
			err "ratchet '$key' worsened: $value > ceiling $ceiling. Fix the regression; do not raise the ceiling without a decision entry."
		elif [ "$value" -lt "$ceiling" ]; then
			ok "ratchet '$key' at $value (ceiling $ceiling — tighten it in this PR)"
		else
			ok "ratchet '$key' at ceiling ($value)"
		fi
	done <ratchets.txt
fi

# ---------------------------------------------------------------------------
# 15. Dependency policy.
#
# Each dependency is attack surface and maintenance burden. Adding one means a decision
# entry in BUILD-LOG.md and an edit to this check — the edit is the point, because it
# makes the exception visible in a diff rather than invisible in a lockfile.
#
# Replace the body with whatever "an undeclared dependency" means for this toolchain.
# Delete the check entirely if the project has no such policy; do not leave it passing
# vacuously, because a check that cannot fail is a check nobody knows stopped running.
# ---------------------------------------------------------------------------
# if [ -f <manifest> ]; then
# 	if grep -qE '<dependency pattern>' <manifest>; then
# 		err "<manifest> declares an external dependency. Justify it in BUILD-LOG.md and amend this check."
# 	else
# 		ok "<manifest> has no external dependencies"
# 	fi
# fi

# ---------------------------------------------------------------------------
# 16. Project-specific checks.
#
# This section is the point of the script; everything above is scaffolding for it.
# Add a check here the first time a convention is broken, rather than restating the
# convention more emphatically — check 12 will hold you to that. Each one gets a
# comment saying what went wrong and why the check is shaped the way it is; a check
# whose reason is undocumented is a check somebody deletes when it becomes
# inconvenient. And add a fixture to scripts/test-checks.sh in the same PR.
#
# Things worth checking, from experience:
#   - a generated artefact (diagram, table, ASCII art) matching its generator
#   - a published site or docs branch matching its source directory
#   - an architectural boundary (a module that must not import a platform API)
#   - a version or identifier written down in more than one place
# ---------------------------------------------------------------------------

if [ "$fail" -ne 0 ]; then
	printf '\nDocumentation discipline failed.\n' >&2
	exit 1
fi
printf 'All checks passed.\n'
