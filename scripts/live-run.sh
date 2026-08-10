#!/usr/bin/env bash
# The per-prompt live run (PLAN.md testing strategy): boot a local ergo, drive
# two supernaut debug sessions against it, and assert on their greppable
# output. Deterministic by construction — the CLI's `wait` verbs replace every
# sleep, and teardown is a trap, so a failed assertion still cleans up.
#
# ergo is a test harness acquired here — never a Cargo dependency. Override
# with ERGO_BIN; otherwise a version-pinned release is fetched into the
# gitignored .cache/ergo/ and its sha256 verified before first use.
set -euo pipefail
# A dead session A must fail an assert, not kill this script: ignore SIGPIPE
# so fifo writes return EPIPE (caught by || true) instead of terminating bash
# (which would also skip the EXIT trap and leak ergo).
trap '' PIPE

cd "$(git rev-parse --show-toplevel)"

# The harness must not depend on ambient PATH (a bare invocation from a
# fresh shell died at 'cargo: command not found' with zero output).
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"

ERGO_VERSION=2.19.1
ERGO_SHA256=1bd97a0917036061e2dcdfc29149c2e252e3bc856282e0956afa2aaaa54dd787
ERGO_PLATFORM=macos-arm64 # the dogfood machine; extend when CI runs this

ergo_bin() {
	if [ -n "${ERGO_BIN:-}" ]; then
		printf '%s\n' "$ERGO_BIN"
		return
	fi
	local dir=".cache/ergo/ergo-${ERGO_VERSION}-${ERGO_PLATFORM}"
	if [ ! -x "$dir/ergo" ]; then
		mkdir -p .cache/ergo
		local tarball=".cache/ergo/ergo-${ERGO_VERSION}.tar.gz"
		curl -sSfL -o "$tarball" \
			"https://github.com/ergochat/ergo/releases/download/v${ERGO_VERSION}/ergo-${ERGO_VERSION}-${ERGO_PLATFORM}.tar.gz"
		printf '%s  %s\n' "$ERGO_SHA256" "$tarball" | shasum -a 256 -c - >&2
		tar -xzf "$tarball" -C .cache/ergo
	fi
	printf '%s\n' "$dir/ergo"
}

ERGO=$(ergo_bin)
WORK=$(mktemp -d)
PORT=$((RANDOM % 20000 + 20000))
for _ in 1 2 3; do
	if ! nc -z 127.0.0.1 "$PORT" 2>/dev/null; then break; fi
	PORT=$((RANDOM % 20000 + 20000))
done
TLS_PORT=$((PORT + 1))
ERGO_PID=""
A_PID=""

cleanup() {
	[ -n "$A_PID" ] && kill "$A_PID" 2>/dev/null || true
	[ -n "$ERGO_PID" ] && kill "$ERGO_PID" 2>/dev/null || true
	if [ -n "${KEEP_WORK:-}" ]; then
		echo "KEEP_WORK: artifacts left in $WORK" >&2
		return
	fi
	# The one sanctioned rm -rf: a mktemp-created dir, path guarded non-empty.
	[ -n "$WORK" ] && rm -rf "$WORK"
}
trap cleanup EXIT

cat >"$WORK/ircd.yaml" <<EOF
network:
  name: SupernautLiveRun
server:
  name: liverun.localhost
  listeners:
    "127.0.0.1:${PORT}": {}
    "127.0.0.1:${TLS_PORT}":
      tls:
        cert: ${WORK}/fullchain.pem
        key: ${WORK}/tls.key
  casemapping: "ascii"
  enforce-utf8: true
  max-sendq: 96k
  relaymsg:
    enabled: false
  ip-limits:
    count: false
    throttle: false
datastore:
  path: ${WORK}/ircd.db
accounts:
  authentication-enabled: true
  registration:
    enabled: true
    allow-before-connect: true
    throttling:
      enabled: false
    email-verification:
      enabled: false
  multiclient:
    enabled: true
    allowed-by-default: true
channels:
  default-modes: +nt
  registration:
    enabled: true
limits:
  nicklen: 32
  channellen: 64
  awaylen: 390
  kicklen: 390
  topiclen: 390
  monitor-entries: 100
  whowas-entries: 100
  chan-list-modes: 60
  registration-messages: 1024
  multiline:
    max-bytes: 4096
    max-lines: 100
fakelag:
  enabled: false
history:
  enabled: true
  channel-length: 2048
  client-length: 256
  chathistory-maxmessages: 100
EOF

# Self-signed cert for the DIALED name (localhost) — ergo's own mkcerts would
# mint one for server.name, which is not what the client verifies against.
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
	-keyout "$WORK/tls.key" -out "$WORK/fullchain.pem" -days 2 -nodes \
	-subj "/CN=localhost" -addext "subjectAltName=DNS:localhost" \
	-addext "basicConstraints=critical,CA:FALSE" \
	>>"$WORK/ergo.log" 2>&1

"$ERGO" run --conf "$WORK/ircd.yaml" >"$WORK/ergo.log" 2>&1 &
ERGO_PID=$!

cargo build -q -p supernaut
BIN=target/debug/supernaut

# Wait for ergo to listen before anything dials it.
for _ in $(seq 1 50); do
	nc -z 127.0.0.1 "$PORT" 2>/dev/null && break
	sleep 0.2
done

# Pre-register the SASL account over the plaintext loopback listener: NICK,
# register with NickServ after 001, quit. The poll is grep-based — the same
# honest sleep-class wait as the cross-session sync below; a `wait` verb
# cannot help here because this exchange is raw nc, not a session.
FAKE_PASS='fake-livetest-passw0rd'
{
	printf 'NICK alice\r\nUSER alice 0 * :pre-reg\r\n'
	sleep 1
	printf 'PRIVMSG NickServ :REGISTER %s\r\n' "$FAKE_PASS"
	sleep 1
	printf 'QUIT\r\n'
} | nc -w 5 127.0.0.1 "$PORT" >"$WORK/prereg.out" || true
grep -qi 'Account created\|already registered' "$WORK/prereg.out" || {
	echo "FAIL: could not pre-register the alice account:" >&2
	cat "$WORK/prereg.out" >&2
	echo "--- ergo log tail:" >&2
	tail -10 "$WORK/ergo.log" >&2
	exit 1
}

# Session A: connect, register, join, then hold the session open while B's
# message arrives (observed in A's --trace-irc capture — MessageAdded events
# arrive in prompt 7). The fifo keeps stdin open until we say quit.
mkfifo "$WORK/a.in"
SUPERNAUT_SASL_PASSWORD="$FAKE_PASS" \
	"$BIN" session --host localhost --port "$TLS_PORT" --nick alice \
	--sasl alice --tls-ca "$WORK/fullchain.pem" \
	--trace-irc --data-dir "$WORK/data-a" \
	<"$WORK/a.in" >"$WORK/a.out" 2>"$WORK/a.trace" &
A_PID=$!
exec 3>"$WORK/a.in"
printf 'connect\nwait registered 10\njoin #supernaut\nwait buffer #supernaut 10\n' >&3 || true

# Wait for A to be joined before B speaks.
for _ in $(seq 1 50); do
	grep -q 'waited buffer #supernaut' "$WORK/a.out" && break
	sleep 0.2
done
grep -q 'waited buffer #supernaut' "$WORK/a.out" || {
	echo "FAIL: session A never joined; a.out:" >&2
	cat "$WORK/a.out" >&2
	tail -5 "$WORK/ergo.log" >&2
	exit 1
}

# Session B: the second party — join and send one line, then quit.
printf 'connect\nwait registered 10\njoin #supernaut\nwait buffer #supernaut 10\nsend #supernaut the deployment failed\nquit\n' |
	"$BIN" session --host localhost --port "$TLS_PORT" --nick bob \
		--tls-ca "$WORK/fullchain.pem" --data-dir "$WORK/data-b" >"$WORK/b.out" 2>&1

# B's send must land at A as a MessageAdded event (the write path, not just
# the raw trace). Sync on the wait verb's own echo line.
printf 'wait message #supernaut 1 15\n' >&3 || true
for _ in $(seq 1 75); do
	grep -q 'waited message #supernaut' "$WORK/a.out" && break
	sleep 0.2
done

# The flood: carol pours 500 numbered lines into #flood while A counts them
# arriving as events; storage commit lines in A's stderr measure batching.
printf 'join #flood\nwait buffer #flood 10\n' >&3 || true
for _ in $(seq 1 50); do
	grep -q 'waited buffer #flood' "$WORK/a.out" && break
	sleep 0.2
done
FLOOD_START=$(date +%s)
{
	printf 'connect\nwait registered 10\njoin #flood\nwait buffer #flood 10\n'
	seq 1 500 | awk '{ print "send #flood flood line " $1 }'
	# Drain before quit: carol counts her own 500 echo-message copies (+1 for
	# her join) — without this, quit races ~300 in-flight requests and the
	# runtime drop discards them (observed: c.out stopped at ok 192).
	printf 'wait message #flood 501 120\nquit\n'
} | "$BIN" session --host localhost --port "$TLS_PORT" --nick carol \
	--tls-ca "$WORK/fullchain.pem" --data-dir "$WORK/data-c" >"$WORK/c.out" 2>&1
printf 'wait message #flood 500 60\n' >&3 || true
for _ in $(seq 1 300); do
	grep -q 'waited message #flood' "$WORK/a.out" && break
	sleep 0.2
done
FLOOD_SECS=$(($(date +%s) - FLOOD_START))

# The invisible-reconnect proof: kill ergo, restart it on the same port with
# the same datastore, and watch A come back with no operator action.
kill "$ERGO_PID" 2>/dev/null || true
wait "$ERGO_PID" 2>/dev/null || true
sleep 0.5
"$ERGO" run --conf "$WORK/ircd.yaml" >>"$WORK/ergo.log" 2>&1 &
ERGO_PID=$!
printf 'wait registered 30\n' >&3 || true
for _ in $(seq 1 150); do
	[ "$(grep -c 'waited registered' "$WORK/a.out")" -ge 2 ] && break
	sleep 0.2
done
# A's joins were verbs, not autojoin config: re-join after the restart, and
# sync on the Join row's own MessageAdded (#supernaut count: A join, B join,
# B privmsg, then this re-join = 4).
printf 'join #supernaut\nwait message #supernaut 4 10\n' >&3 || true
for _ in $(seq 1 50); do
	[ "$(grep -c 'waited message #supernaut' "$WORK/a.out")" -ge 2 ] && break
	sleep 0.2
done
# --- Search (prompt 8): by now the corpus spans two channels and a restart.
SEARCH_START=$(date +%s)
printf 'search deployment\nwait search 1 10\n' >&3 || true
for _ in $(seq 1 50); do
	grep -q 'waited search' "$WORK/a.out" && break
	sleep 0.2
done
SEARCH_SECS=$(($(date +%s) - SEARCH_START))
printf 'search from:bob deployment\nwait search 2 10\n' >&3 || true
printf 'search from:carol deployment\nwait search 3 10\n' >&3 || true
printf 'search in:#flood "flood line 250"\nwait search 4 10\n' >&3 || true
printf 'search before:2020 deployment\nwait search 5 10\n' >&3 || true
# Read-your-writes: wait (harness-level — the echo's arrival is network, not
# storage) for the fresh line's echo in the raw trace, then search with no
# further delay: the job-queue flush barrier must make it visible.
printf 'send #supernaut xyzzysearchtoken\n' >&3 || true
for _ in $(seq 1 50); do
	grep -q '^<< .*PRIVMSG #supernaut :xyzzysearchtoken' "$WORK/a.trace" && break
	sleep 0.2
done
printf 'search xyzzysearchtoken\nwait search 6 10\n' >&3 || true
for _ in $(seq 1 75); do
	grep -q 'waited search 6\|event search-results request=[0-9]* hits=1' "$WORK/a.out" &&
		[ "$(grep -c 'waited search' "$WORK/a.out")" -ge 6 ] && break
	sleep 0.2
done
# Malformed MATCH: an error response, and the session keeps running.
printf 'search "\n' >&3 || true
for _ in $(seq 1 25); do
	grep -q '^error ' "$WORK/a.out" && break
	sleep 0.2
done

# --- Backlog (prompt 9a): windows over the corpus the flood and the restart
# built. `wait backlog` counts responses, so a failed window ends the wait with
# a printed error instead of a timeout with nothing to read.
BACKLOG_START=$(date +%s)
printf 'backlog #flood after:0 5\nwait backlog 1 10\n' >&3 || true
for _ in $(seq 1 50); do
	grep -q 'waited backlog' "$WORK/a.out" && break
	sleep 0.2
done
# 9999 asked for, 200 delivered: the cap observed from outside the engine.
printf 'backlog #flood latest 9999\nwait backlog 2 20\n' >&3 || true
for _ in $(seq 1 100); do
	[ "$(grep -c 'waited backlog' "$WORK/a.out")" -ge 2 ] && break
	sleep 0.2
done
# Jump-to-context, headless: the anchor is the newest #flood search hit above
# ("flood line 250"), not a seq this script pasted in.
printf 'backlog #flood around-hit 7\nwait backlog 3 10\n' >&3 || true
for _ in $(seq 1 50); do
	[ "$(grep -c 'waited backlog' "$WORK/a.out")" -ge 3 ] && break
	sleep 0.2
done
BACKLOG_SECS=$(($(date +%s) - BACKLOG_START))

printf 'quit\n' >&3 || true
exec 3>&-
wait "$A_PID" 2>/dev/null || true
A_PID=""

# Keep the capture for the corpus harvest (gitignored; trace-to-steps.sh).
cp "$WORK/a.trace" .cache/last-a.trace

# The announcement proof (prompt 9a): a process that never dials anything. D
# opens A's closed data dir, issues no `connect`, and resolves #supernaut purely
# from the attach-time replay — then reads back a line a different process
# wrote. It must pass the same --host as A: the network name is derived
# (debug-<host>) and the replay resolves buffers by that name, a coupling
# prompt 10a's config file deletes. Two live processes over one file is
# deliberately not attempted — cached per-buffer seq counters in two writers
# would race the write path, and stage 4's daemon makes one writer structural.
printf 'wait buffer #supernaut 10\nbacklog #supernaut latest 5\nwait backlog 1 10\nquit\n' |
	"$BIN" session --host localhost --port "$TLS_PORT" --nick dave \
		--tls-ca "$WORK/fullchain.pem" --data-dir "$WORK/data-a" >"$WORK/d.out" 2>&1

fail=0
assert() {
	if grep -q "$2" "$1"; then
		printf 'ok    %s\n' "$3"
	else
		printf 'FAIL  %s (pattern %q not in %s)\n' "$3" "$2" "$1" >&2
		fail=1
	fi
}

assert "$WORK/a.out" 'event connection-state network=1 phase=connecting' 'A saw connecting'
assert "$WORK/a.out" 'event connection-state network=1 phase=registered' 'A saw registered'
assert "$WORK/a.out" 'event buffer-created .* name=#supernaut' 'A saw its buffer event'
assert "$WORK/a.trace" '>> CAP LS 302' 'A trace captured the opening'
assert "$WORK/a.trace" 'PRIVMSG #supernaut :the deployment failed' "B's send landed at A"
assert "$WORK/b.out" 'event connection-state network=1 phase=registered' 'B registered'
assert "$WORK/a.trace" '903 .*uthentication successful' 'A authenticated via SASL (903)'
assert "$WORK/a.trace" '>> AUTHENTICATE PLAIN' 'A offered SASL PLAIN'
assert "$WORK/a.out" 'phase=disconnected' 'A saw the ergo restart'
assert "$WORK/a.out" 'phase=connecting detail=retry' 'A retried through backoff'
if [ "$(grep -c 'phase=registered' "$WORK/a.out")" -ge 2 ]; then
	printf 'ok    %s\n' 'A re-registered after the restart'
else
	printf 'FAIL  %s\n' 'A re-registered after the restart' >&2
	fail=1
fi
assert "$WORK/a.out" 'event message-added' "B's send arrived as a MessageAdded event"
assert "$WORK/a.out" 'waited message #flood' 'A counted all 500 flood lines'
if [ "$(grep -c 'event buffer-created .* name=#supernaut' "$WORK/a.out")" -eq 1 ]; then
	printf 'ok    %s\n' 'exactly one buffer-created for #supernaut across the restart'
else
	printf 'FAIL  %s (saw %s)\n' 'exactly one buffer-created for #supernaut across the restart' \
		"$(grep -c 'event buffer-created .* name=#supernaut' "$WORK/a.out")" >&2
	fail=1
fi
COMMITS=$(grep -c 'storage commit rows=' "$WORK/a.trace" || true)
if [ "$COMMITS" -gt 0 ] && [ "$COMMITS" -lt 50 ]; then
	printf 'ok    %s (%s commits for the flood+session, %ss flood)\n' 'writes batched' "$COMMITS" "$FLOOD_SECS"
else
	printf 'FAIL  %s (%s commits)\n' 'writes batched' "$COMMITS" >&2
	fail=1
fi

assert "$WORK/a.out" 'hit .*text=the deployment failed' 'search found the deployment line'
assert "$WORK/a.out" 'hit .*text=flood line 250' 'phrase+buffer filter found exactly the one line'
if [ "$(grep -c 'hit .*text=flood line 250' "$WORK/a.out")" -eq 1 ]; then
	printf 'ok    %s\n' 'phrase search did not match the 2500-prefix'
else
	printf 'FAIL  %s\n' 'phrase search did not match the 2500-prefix' >&2
	fail=1
fi
if [ "$(grep -c 'event search-results request=[0-9]* hits=0' "$WORK/a.out")" -ge 2 ]; then
	printf 'ok    %s\n' 'nick and time filters excluded correctly (two empty result sets)'
else
	printf 'FAIL  %s\n' 'nick and time filters excluded correctly' >&2
	fail=1
fi
assert "$WORK/a.out" 'hit .*text=xyzzysearchtoken' 'read-your-writes: fresh line searchable through the flush barrier'
assert "$WORK/a.out" '^error [0-9]' 'malformed MATCH came back as an error, session survived'
printf 'ok    search wall time %ss over the %s-commit corpus (recorded, not asserted)\n' "$SEARCH_SECS" "$COMMITS"

# --- Backlog windows (prompt 9a).
assert "$WORK/a.out" 'backlog request=[0-9]* buffer=[0-9]* count=5' 'after:0 returned a five-line window'
assert "$WORK/a.out" 'line buffer=[0-9]* seq=1 ' 'the after:0 window starts at seq=1, ascending'
assert "$WORK/a.out" 'backlog request=[0-9]* buffer=[0-9]* count=200' 'the engine capped a 9999-row window at 200'
assert "$WORK/a.out" 'line .*text=flood line 250' 'around-hit centred on the search hit'
assert "$WORK/a.out" 'line .*text=flood line 247' 'around-hit carried three lines before the hit'
assert "$WORK/a.out" 'line .*text=flood line 253' 'around-hit carried three lines after the hit'
printf 'ok    backlog wall time %ss for three windows incl. the 200-row cap (recorded, not asserted)\n' "$BACKLOG_SECS"

# --- The announcement proof: session D never touched the network.
assert "$WORK/d.out" 'event buffer-created .* name=#supernaut' 'D was told about a buffer it never saw created'
assert "$WORK/d.out" 'waited buffer #supernaut' 'D resolved #supernaut from the attach replay alone'
assert "$WORK/d.out" 'line .*text=the deployment failed' "D read another process's history out of a window"
if grep -q 'event connection-state' "$WORK/d.out"; then
	printf 'FAIL  %s\n' 'D never dialled anything' >&2
	fail=1
else
	printf 'ok    %s\n' 'D never dialled anything (no connection-state line in its output)'
fi

# Post-mortem, from outside the process: WAL held, seq contiguous, no dupes.
DB="$WORK/data-a/history.db"
JOURNAL=$(sqlite3 "$DB" 'PRAGMA journal_mode')
FLOOD_BUF=$(sqlite3 "$DB" "SELECT id FROM buffer WHERE name = '#flood'")
ROWS=$(sqlite3 "$DB" "SELECT COUNT(*) FROM message WHERE buffer_id = $FLOOD_BUF AND kind = 0")
MAXSEQ=$(sqlite3 "$DB" "SELECT MAX(seq) FROM message WHERE buffer_id = $FLOOD_BUF")
ALLROWS=$(sqlite3 "$DB" "SELECT COUNT(*) FROM message WHERE buffer_id = $FLOOD_BUF")
DISTINCT=$(sqlite3 "$DB" "SELECT COUNT(DISTINCT text) FROM message WHERE buffer_id = $FLOOD_BUF AND kind = 0")
FTS_ROWS=$(sqlite3 "$DB" "SELECT COUNT(*) FROM message_fts WHERE message_fts MATCH 'flood'")
if [ "$JOURNAL" = wal ] && [ "$ROWS" -eq 500 ] && [ "$DISTINCT" -eq 500 ] && [ "$ALLROWS" -eq "$MAXSEQ" ]; then
	printf 'ok    %s (journal=%s rows=%s distinct=%s seq contiguous at %s)\n' 'history survived on disk' "$JOURNAL" "$ROWS" "$DISTINCT" "$MAXSEQ"
	if [ "$FTS_ROWS" -ge 500 ]; then
		printf 'ok    %s (%s indexed)\n' 'the FTS index is real on disk' "$FTS_ROWS"
	else
		printf 'FAIL  %s (%s indexed)\n' 'the FTS index is real on disk' "$FTS_ROWS" >&2
		fail=1
	fi
else
	printf 'FAIL  %s (journal=%s rows=%s distinct=%s maxseq=%s allrows=%s)\n' 'history survived on disk' "$JOURNAL" "$ROWS" "$DISTINCT" "$MAXSEQ" "$ALLROWS" >&2
	fail=1
fi

if [ "$fail" -ne 0 ]; then
	grep -E 'AUTHENTICATE|90[0-9] ' "$WORK/a.trace" >&2 || true
	echo "live run failed; ergo log tail:" >&2
	tail -5 "$WORK/ergo.log" >&2
	exit 1
fi
echo "live run passed"
