#!/usr/bin/env bash
# The per-prompt live run (PLAN.md testing strategy): boot a local ergo, drive
# two supernaut debug sessions against it, and assert on their greppable
# output. Deterministic by construction — the CLI's `wait` verbs replace every
# sleep, and teardown is a trap, so a failed assertion still cleans up.
#
# ergo is a test harness acquired here — never a Cargo dependency. Override
# with ERGO_BIN; otherwise a version-pinned release is fetched into the
# gitignored .cache/ergo/ and its sha256 verified before first use.
#
# Since prompt 10a **every session runs from a generated config file** and there
# is no --host, --port, --nick, --join, --tls-ca or --allow-plaintext flag left to
# pass: the connection surface is the TOML file, so this script exercises the
# surface every later stage builds on rather than a parallel flags path.
#
# WHAT THIS DOES TO YOUR MACHINE, stated plainly (prompt 10b): everything else
# lives in $WORK, but the credential cannot. This run creates — and then removes,
# kept artifacts or not — exactly one generic-password item in your **login**
# keychain: service `supernaut`, account `liverun`, holding the recognisably-fake
# password `fake-livetest-passw0rd`. keyring's macOS store writes to the User
# keychain and pointing it at a scratch keychain would need the keyring-core
# route prompt 10b rejected, so there is nowhere else to put it. The item is
# created by `supernaut credential set` — the product's own write path, not a
# `security add-generic-password` shortcut — because a path only the harness
# takes is a path the product never proves. `security ... -g` appears nowhere in
# this file: it prints the secret.
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

# Every keychain touch runs under a bounded watchdog. A macOS authorization
# dialog is invisible from a script and indistinguishable from a deadlock, and
# this script must never sleep and never hang — so a stall becomes a loud line
# and a killed child instead of a run that never ends.
#
# stdin is an explicit argument because bash redirects an asynchronous command's
# stdin from /dev/null when job control is off, which would silently feed
# `credential set` nothing at all.
#
# `quiet` is an argument rather than a redirection on the call, because
# `with_watchdog … >/dev/null 2>&1` silences the *watchdog's own* FAIL line along
# with the command's chatter — which is precisely the line that must survive, since
# a stalled keychain call is invisible otherwise.
#
# Usage: with_watchdog <label> <seconds> <stdin-file> <quiet|loud> <command...>
with_watchdog() {
	local label=$1 secs=$2 stdin=$3 quiet=$4
	shift 4
	if [ "$quiet" = quiet ]; then
		"$@" <"$stdin" >/dev/null 2>&1 &
	else
		"$@" <"$stdin" &
	fi
	local pid=$!
	{
		sleep "$secs"
		if kill -0 "$pid" 2>/dev/null; then
			echo "FAIL: $label did not finish in ${secs}s — a keychain authorization" \
				"dialog looks exactly like this from a script" >&2
			kill -9 "$pid" 2>/dev/null || true
		fi
	} &
	local guard=$!
	local status=0
	wait "$pid" || status=$?
	# Kill the guard's `sleep` as well as the subshell holding it: killing only the
	# subshell leaves the sleep orphaned for up to $secs. Harmless either way (a
	# late guard finds no such pid), but a script that must never sleep should not
	# leave sleeps behind.
	pkill -P "$guard" 2>/dev/null || true
	kill "$guard" 2>/dev/null || true
	wait "$guard" 2>/dev/null || true
	return "$status"
}

# Best-effort, hence the `|| true`: run before seeding as well as in cleanup.
# Before, because the debug binary is ad-hoc signed and its cdhash changes on
# every rebuild — an item whose ACL names last week's build is exactly what turns
# session A's read into a GUI prompt, i.e. a hang. After, because the harness
# leaves no credential behind.
keychain_forget() {
	with_watchdog 'security delete-generic-password' 10 /dev/null quiet \
		security delete-generic-password -s supernaut -a liverun || true
}

cleanup() {
	[ -n "$A_PID" ] && kill "$A_PID" 2>/dev/null || true
	[ -n "$ERGO_PID" ] && kill "$ERGO_PID" 2>/dev/null || true
	# Before the KEEP_WORK early return: kept artifacts are for debugging, and a
	# credential is not an artifact.
	keychain_forget
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

# The harness writes each session's config file; the program never does (the
# 2026-08-10 config-vs-runtime-state decision, and the toml serializer is not
# even compiled into havoc-core). Usage: write_config <dir> <nick> <network>
# [autojoin] [sasl_account]. Both optional lines are emitted from an `if` block,
# not a trailing `[ -n ... ] &&` test — under `set -e` that would make an absent
# value the function's failing last command.
#
# `sasl_account` is the whole credential surface config has: a *name*. The
# password is in the OS keyring, and this file has no way to write one there
# other than running the product's own `credential set` (below).
write_config() {
	local dir=$1 nick=$2 network=$3 autojoin=${4:-} sasl_account=${5:-}
	mkdir -p "$dir"
	{
		printf '# Generated by scripts/live-run.sh. Seed data only.\n'
		printf 'nick = "%s"\n\n' "$nick"
		printf '[networks.%s]\n' "$network"
		printf 'host = "localhost"\n'
		printf 'port = %s\n' "$TLS_PORT"
		printf 'tls_ca = "%s"\n' "$WORK/fullchain.pem"
		if [ -n "$autojoin" ]; then
			printf 'autojoin = ["%s"]\n' "$autojoin"
		fi
		if [ -n "$sasl_account" ]; then
			printf 'sasl_account = "%s"\n' "$sasl_account"
		fi
	} >"$dir/config.toml"
}

# Session A: connect, land in #supernaut with no `join` verb at all — the channel
# comes from the config file's autojoin — then hold the session open while B's
# message arrives (observed in A's --trace-irc capture — MessageAdded events
# arrive in prompt 7). The fifo keeps stdin open until we say quit. A passes
# --network explicitly: the name `liverun` is the coupling session D's proof
# rests on, so it is stated, not defaulted.
write_config "$WORK/config-a" alice liverun '#supernaut' alice

# A's password reaches the keyring through the product's own write path, from
# stdin — never argv (`ps` is world-readable) and never the environment (`ps eww`,
# and every child inherits it), which is why the `--sasl` flag and the
# SUPERNAUT_SASL_PASSWORD bridge are both gone rather than kept as a fallback.
# After this, the credential exists in nothing A can see: not its config, not its
# argv, not its environment.
keychain_forget
printf '%s' "$FAKE_PASS" >"$WORK/a.pass"
SUPERNAUT_CONFIG_DIR="$WORK/config-a" with_watchdog \
	'supernaut credential set liverun' 10 "$WORK/a.pass" loud \
	"$BIN" credential set liverun || {
	rm -f "$WORK/a.pass"
	echo "FAIL: could not store session A's SASL password in the OS keyring" >&2
	exit 1
}
# Immediately, on both paths: stdin needed a file, and a file holding the fake
# password is the one artifact KEEP_WORK must not keep — the header promises the
# credential lives in the keychain and nowhere else.
rm -f "$WORK/a.pass"

mkfifo "$WORK/a.in"
SUPERNAUT_CONFIG_DIR="$WORK/config-a" \
	"$BIN" session --network liverun \
	--trace-irc --data-dir "$WORK/data-a" \
	<"$WORK/a.in" >"$WORK/a.out" 2>"$WORK/a.trace" &
A_PID=$!
exec 3>"$WORK/a.in"
# The seed is observed as a *Join row* — `wait rows`, never `wait message`, which
# counts privmsg/notice only and would hang until a human speaks (note 9b).
printf 'connect\nwait registered 10\nwait rows #supernaut 1 10\n' >&3 || true

# Wait for A's autojoin to land before B speaks.
for _ in $(seq 1 50); do
	grep -q 'waited rows #supernaut' "$WORK/a.out" && break
	sleep 0.2
done
grep -q 'waited rows #supernaut' "$WORK/a.out" || {
	echo "FAIL: session A never autojoined; a.out:" >&2
	cat "$WORK/a.out" >&2
	echo "--- a.trace tail (A now refuses to dial at all if the keychain read fails:" >&2
	echo "    a missing supernaut/liverun item, or an ACL naming an older build):" >&2
	tail -5 "$WORK/a.trace" >&2
	tail -5 "$WORK/ergo.log" >&2
	exit 1
}

# Session B: the second party — `join` as an explicit verb (both paths stay
# exercised), send one line, then quit. No --network: B's config names exactly
# one network, which is the single-network default this segment proves.
#
# The bare `quit` has no deadline argument and does not need one: nothing
# autoconnects and autojoin issues no `Request`, so `outstanding` holds exactly
# what this segment named. When stage 2's embedded wiring adds a startup-issued
# request nobody typed, note 9b's early-quit failure mode reopens here.
write_config "$WORK/config-b" bob liverun
printf 'connect\nwait registered 10\njoin #supernaut\nwait buffer #supernaut 10\nsend #supernaut the deployment failed\nquit\n' |
	SUPERNAUT_CONFIG_DIR="$WORK/config-b" "$BIN" session \
		--data-dir "$WORK/data-b" >"$WORK/b.out" 2>&1

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
write_config "$WORK/config-c" carol liverun
{
	printf 'connect\nwait registered 10\njoin #flood\nwait buffer #flood 10\n'
	seq 1 500 | awk '{ print "send #flood flood line " $1 }'
	# Carol counts her own 500 echo-message copies — `wait message` is
	# privmsg/notice only now, so her join no longer inflates the number. The
	# echo wait stays because it proves the *server* saw the lines, which no Ack
	# does; `quit` now additionally drains her outstanding requests (before
	# prompt 9b the runtime drop discarded them and c.out stopped near ok 192).
	printf 'wait message #flood 500 120\nquit\n'
} | SUPERNAUT_CONFIG_DIR="$WORK/config-c" "$BIN" session \
	--data-dir "$WORK/data-c" >"$WORK/c.out" 2>&1
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
# A's #supernaut is config autojoin, which re-fires on the fresh machine
# (prompt 6) — so there is no re-join verb here, only the sync on the Join row it
# produces. `wait rows` counts every MessageAdded kind — what the *store* counts —
# so this says what it means: the thing being awaited IS a Join row (#supernaut
# rows: A's autojoin, B's join, B's privmsg, A's autojoin re-fire = 4). The
# arithmetic is unchanged from when both of A's joins were verbs; only who issued
# them moved. Counted `-ge 2`, because A's first autojoin already printed one.
printf 'wait rows #supernaut 4 10\n' >&3 || true
for _ in $(seq 1 50); do
	[ "$(grep -c 'waited rows #supernaut' "$WORK/a.out")" -ge 2 ] && break
	sleep 0.2
done
# The post-loop check the other sync points have and this one did not: without it a
# timeout here falls through into the search and marker assertions, which then fail
# for the wrong reason (an empty corpus) and hide the reconnect as the real cause.
[ "$(grep -c 'waited rows #supernaut' "$WORK/a.out")" -ge 2 ] || {
	echo "FAIL: A never re-autojoined #supernaut after the ergo restart; a.out tail:" >&2
	tail -20 "$WORK/a.out" >&2
	tail -10 "$WORK/ergo.log" >&2
	exit 1
}
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
# ("flood line 250"), not a seq this script pasted in. Note the coupling this
# depends on: `last_hits` is filled by the SearchResults *event*, while `wait
# search` now returns on the *response*. It is safe only because search's Response
# and its correlated Event ride the same directed lane, in that order — if
# SearchResults ever moves lanes, this line races its own data.
printf 'backlog #flood around-hit 7\nwait backlog 3 10\n' >&3 || true
for _ in $(seq 1 50); do
	[ "$(grep -c 'waited backlog' "$WORK/a.out")" -ge 3 ] && break
	sleep 0.2
done
BACKLOG_SECS=$(($(date +%s) - BACKLOG_START))

# --- Read markers (prompt 9b). The arithmetic, so a later traffic change is
# diagnosable rather than mysterious: #supernaut's rows are A's join (1), B's
# join (2), B's "the deployment failed" (3), A's re-join after the restart (4),
# and xyzzysearchtoken (5). The assertions below are that the *value* 3 round
# trips — through the broadcast event here, and through session D's attach
# announcement and sqlite3 later — so they survive seq 3 ceasing to be that line.
printf 'mark-read #supernaut 3\nwait marker 1 10\n' >&3 || true
for _ in $(seq 1 50); do
	grep -q 'waited marker' "$WORK/a.out" && break
	sleep 0.2
done

printf 'quit\n' >&3 || true
exec 3>&-
wait "$A_PID" 2>/dev/null || true
A_PID=""

# Keep the capture for the corpus harvest (gitignored; trace-to-steps.sh).
cp "$WORK/a.trace" .cache/last-a.trace

# The announcement proof (prompt 9a): a process that never dials anything. D
# opens A's closed data dir, issues no `connect`, and resolves #supernaut purely
# from the attach-time replay — then reads back a line a different process
# wrote. D's config names the network A's config named, `liverun`, and that is
# *why* the replay resolves: config identity and storage identity are one name
# (prompt 10a). It is no longer the `debug-<host>` accident of two sessions
# pointed at one host string. Two live processes over one file is
# deliberately not attempted — cached per-buffer seq counters in two writers
# would race the write path, and stage 4's daemon makes one writer structural.
# D reads `after:0` rather than `latest 5`: anchored at the start of history, so
# traffic added to #supernaut before A quits can never push "the deployment
# failed" out of the window and turn a window failure into an announcement one.
write_config "$WORK/config-d" dave liverun
printf 'wait buffer #supernaut 10\nbacklog #supernaut after:0 50\nwait backlog 1 10\nquit\n' |
	SUPERNAUT_CONFIG_DIR="$WORK/config-d" "$BIN" session --network liverun \
		--data-dir "$WORK/data-a" >"$WORK/d.out" 2>&1

# Session E: abandonment, observed. Same data dir as A and D, but a config naming
# a *different* network — so every buffer on `liverun` is an orphan: reported
# loudly, one line per network, and announced to nobody. The `wait buffer` is
# meant to time out, which is a non-zero exit, hence the `|| true` on the
# invocation rather than on the assertions below. This is also the reversibility
# proof read backwards: adding `[networks.liverun]` to E's file is all it would
# take, which is why no `debug-*` migration is owed.
write_config "$WORK/config-e" erin elsewhere
printf 'wait buffer #supernaut 3\nquit\n' |
	SUPERNAUT_CONFIG_DIR="$WORK/config-e" "$BIN" session --network elsewhere \
		--data-dir "$WORK/data-a" >"$WORK/e.out" 2>&1 || true

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
# These two stay from prompt 6, and are now the regression net proving the
# *keyring* produced a real credential: the only place A's password came from is
# the keychain item `credential set` wrote.
assert "$WORK/a.trace" '903 .*uthentication successful' 'A authenticated via SASL (903)'
assert "$WORK/a.trace" '>> AUTHENTICATE PLAIN' 'A offered SASL PLAIN'
assert "$WORK/a.trace" '>> AUTHENTICATE <redacted>' 'the SASL payload was redacted in the trace'
# CLAUDE.md's never-in-logs rule, made mechanical. The trace outlives $WORK (it
# is copied to .cache/last-a.trace for the corpus harvest), so a base64 payload
# in it would be a password persisted on disk — base64 is not redaction.
# `\0alice\0…`, not `alice\0alice\0…`: SASL PLAIN is authzid NUL authcid NUL
# password and `crates/havoc-core/src/connection/caps.rs` sends an *empty* authzid
# (its own unit test pins `\0alice\0sesame`). The first version of this line used
# the wrong framing, which made the assertion below inert — it was checking for a
# string the program could never emit. Verified by breaking the redactor on purpose
# and watching this fail.
PLAIN_PAYLOAD=$(printf '\0alice\0%s' "$FAKE_PASS" | base64)
for log in "$WORK/a.trace" "$WORK/a.out" .cache/last-a.trace; do
	# The file must exist: `grep` on a missing path fails, which would otherwise
	# read as "the secret is not in there" — a check that fails open, silently.
	if [ ! -f "$log" ]; then
		printf 'FAIL  %s (%s is missing)\n' 'no credential material in the log' "$log" >&2
		fail=1
	elif grep -qF "$FAKE_PASS" "$log" || grep -qF "$PLAIN_PAYLOAD" "$log"; then
		printf 'FAIL  %s (%s)\n' 'no credential material in the log' "$log" >&2
		fail=1
	else
		printf 'ok    %s (%s)\n' 'no credential material in the log' "$log"
	fi
done
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
# The arithmetic, so a future change to #flood's traffic is diagnosable: "flood
# line 250" sits at seq 252 (A's join is seq 1, carol's join seq 2, then the 500
# lines), and limit 7 splits 3 before / 3 after — hence 247 and 253 exactly.
assert "$WORK/a.out" 'line .*text=flood line 250' 'around-hit centred on the search hit'
assert "$WORK/a.out" 'line .*text=flood line 247' 'around-hit carried three lines before the hit'
assert "$WORK/a.out" 'line .*text=flood line 253' 'around-hit carried three lines after the hit'
printf 'ok    backlog wall time %ss for three windows incl. the 200-row cap (recorded, not asserted)\n' "$BACKLOG_SECS"

# --- Read markers (prompt 9b): a marker A set, observed by A as broadcast state.
assert "$WORK/a.out" 'event read-marker buffer=[0-9]* seq=3' \
	'A saw its own read marker come back as broadcast core state'

# --- The announcement proof: session D never touched the network.
assert "$WORK/d.out" 'event buffer-created .* name=#supernaut' 'D was told about a buffer it never saw created'
assert "$WORK/d.out" 'waited buffer #supernaut' 'D resolved #supernaut from the attach replay alone'
assert "$WORK/d.out" 'line .*text=the deployment failed' "D read another process's history out of a window"
assert "$WORK/d.out" 'event buffer-created .* name=#supernaut last_read=3' \
	"D was told another process's read marker, out of the attach announcement"
if grep -q 'event connection-state' "$WORK/d.out"; then
	printf 'FAIL  %s\n' 'D never dialled anything' >&2
	fail=1
else
	printf 'ok    %s\n' 'D never dialled anything (no connection-state line in its output)'
fi

# --- Abandonment, observed (prompt 10a): session E's orphan report.
assert "$WORK/e.out" 'orphan network liverun' \
	"E said out loud that its data dir holds buffers no configured network claims"
if grep -q 'event buffer-created .* name=#supernaut' "$WORK/e.out"; then
	printf 'FAIL  %s\n' 'E announced nothing from an unconfigured network' >&2
	fail=1
else
	printf 'ok    %s\n' 'E announced nothing from an unconfigured network'
fi

# --- The quit drain (prompt 9b), proved by counting the responses nobody used to
# get: B's three (connect, join, and the send whose Ack raced the runtime drop)
# and carol's 502 (connect, join, 500 sends).
B_OKS=$(grep -c '^ok ' "$WORK/b.out" || true)
if [ "$B_OKS" -eq 3 ]; then
	printf 'ok    %s\n' 'quit drained B: all three responses printed'
else
	printf 'FAIL  %s (saw %s of 3)\n' 'quit drained B: all three responses printed' "$B_OKS" >&2
	fail=1
fi
C_OKS=$(grep -c '^ok ' "$WORK/c.out" || true)
if [ "$C_OKS" -eq 502 ]; then
	printf 'ok    %s\n' "quit drained carol's flood: all 502 responses printed"
else
	printf 'FAIL  %s (saw %s of 502)\n' "quit drained carol's flood: all 502 responses printed" "$C_OKS" >&2
	fail=1
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
MARKER=$(sqlite3 "$DB" "SELECT last_read_seq FROM buffer WHERE name = '#supernaut'")
if [ "$MARKER" = 3 ]; then
	printf 'ok    %s (last_read_seq=%s)\n' 'the read marker is on disk' "$MARKER"
else
	printf 'FAIL  %s (last_read_seq=%s)\n' 'the read marker is on disk' "$MARKER" >&2
	fail=1
fi
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
