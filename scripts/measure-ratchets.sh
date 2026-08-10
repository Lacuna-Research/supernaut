#!/usr/bin/env bash
# Emits the current value of every ratcheted metric, one "key value" per line.
# check-docs.sh compares this output against the ceilings in ratchets.txt.
#
# The two defaults below are generic slow-rot metrics. Replace or extend them with
# whatever this project's rot actually looks like — warning counts, dead-code lines,
# skipped tests, cyclic imports. Every key emitted here must have a ceiling in
# ratchets.txt, and vice versa; check 14 fails on a key that exists on only one side.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SOURCE_DIR="src"

if [ -d "$SOURCE_DIR" ]; then
	# Deferred work markers. Each one is a promise; the ratchet caps how many
	# promises may be outstanding at once.
	# `|| true` because grep exits 1 on zero matches, and pipefail would turn a
	# clean codebase into a dead script.
	todos=$({ grep -rEc 'TODO|FIXME|HACK' "$SOURCE_DIR" 2>/dev/null || true; } | awk -F: '{ n += $2 } END { print n + 0 }')

	# Longest file. Files grow one reasonable addition at a time; nothing but a
	# ceiling ever makes the split happen.
	longest=$(find "$SOURCE_DIR" -type f -exec wc -l {} + 2>/dev/null | awk '$2 != "total" && $1 > max { max = $1 } END { print max + 0 }')
else
	todos=0
	longest=0
fi

printf 'todo-count %s\n' "$todos"
printf 'longest-file %s\n' "$longest"
