#!/usr/bin/env bash
# Convert a --trace-irc capture into DRAFT Rust Step rows for the transcript
# corpus in crates/havoc-core/tests/state_machine.rs.
#
# The trace shares stderr with session diagnostics, so only `>> ` / `<< `
# prefixed lines are read. CAUTION: `>> ` covers user-command lines (JOIN,
# PRIVMSG sent by hand) as well as machine replies — the output is a draft a
# person reviews and pastes, never text committed blind. Redaction rule: never
# paste a live AUTHENTICATE payload from a real account.
#
# Usage: trace-to-steps.sh <trace-file>
set -euo pipefail

[ $# -eq 1 ] || {
	echo "usage: $0 <trace-file>" >&2
	exit 1
}

awk '
	function flush() {
		if (server != "") {
			printf "            Step(%s, &[", server
			for (i = 1; i <= nout; i++) printf "%s%s", (i > 1 ? ", " : ""), outs[i]
			print "]),"
		}
		server = ""
		nout = 0
	}
	/^<< / {
		flush()
		line = substr($0, 4)
		gsub(/\\/, "\\\\", line); gsub(/"/, "\\\"", line)
		server = "\"" line "\""
	}
	/^>> / {
		line = substr($0, 4)
		gsub(/\\/, "\\\\", line); gsub(/"/, "\\\"", line)
		outs[++nout] = "\"" line "\""
	}
	END { flush() }
' "$1"
