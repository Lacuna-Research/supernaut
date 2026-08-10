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

cd "$(git rev-parse --show-toplevel)"

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
printf 'connect\nwait registered 10\njoin #supernaut\nwait buffer #supernaut 10\n' >&3

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

# B's send must land in A's raw capture.
for _ in $(seq 1 50); do
	grep -q 'PRIVMSG #supernaut :the deployment failed' "$WORK/a.trace" && break
	sleep 0.2
done

# The invisible-reconnect proof: kill ergo, restart it on the same port with
# the same datastore, and watch A come back with no operator action.
kill "$ERGO_PID" 2>/dev/null || true
wait "$ERGO_PID" 2>/dev/null || true
sleep 0.5
"$ERGO" run --conf "$WORK/ircd.yaml" >>"$WORK/ergo.log" 2>&1 &
ERGO_PID=$!
printf 'wait registered 30\n' >&3
for _ in $(seq 1 150); do
	[ "$(grep -c 'phase=registered' "$WORK/a.out")" -ge 2 ] && break
	sleep 0.2
done
printf 'quit\n' >&3
exec 3>&-
wait "$A_PID" 2>/dev/null || true
A_PID=""

# Keep the capture for the corpus harvest (gitignored; trace-to-steps.sh).
cp "$WORK/a.trace" .cache/last-a.trace

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

if [ "$fail" -ne 0 ]; then
	grep -E 'AUTHENTICATE|90[0-9] ' "$WORK/a.trace" >&2 || true
	echo "live run failed; ergo log tail:" >&2
	tail -5 "$WORK/ergo.log" >&2
	exit 1
fi
echo "live run passed"
