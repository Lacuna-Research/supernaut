# Stage 1 — The Prompts

**Status:** 7/10 complete. Next: prompt 8.

<!-- 10 must match the STAGES array in scripts/check-docs.sh — change both together.
The line is machine-read; `make check` fails if they disagree. -->

Stage 1's work queue. Every numbered item in `PLAN.md`'s stage 1 is attached to exactly
one prompt here. The grouping is by **shared seam**, not by theme — two items belong
together when they touch the same code, and apart when they merely sound related.

Each block is self-contained and assumes the previous ones are done and merged. Every
prompt has a **Do not** section: the scope fence that keeps later work from leaking
backward. Standing rules — Rust stable, cargo workspace under `crates/`, zero-warning
builds, clippy strict, rustfmt, the dependency allowlist — live in `CLAUDE.md` and load
automatically.

Each prompt is one branch (`prompt-NN-slug`), one PR, squash-merged once CI is green.
Bump the **Status** line above in the same PR; `make check` fails if it is missing or
malformed.

### Prompts are written just-in-time, and that is deliberate

Prompts 1–4 are written out in full. **Prompts 5–10 carry their scope, their grouping
and their fence, but not yet their detail** — and are to be fleshed out immediately
before they start, not now.

Writing a detailed brief for work that six intervening prompts will have reshaped
produces a brief that is confidently wrong, and a wrong brief is worse than a thin one
because it is followed. The `PLAN.md` items behind these are the durable statement of
intent; a prompt is the short-lived working document that turns one into a session.
Reorder, rescope and merge these freely as the stage teaches you things — that is what
`BUILD-LOG.md` preserves the history for.

### Carry-forward notes

When work on one prompt turns up something a later prompt needs to know, the note is
appended to **that prompt, in this file**, under a `### Carry-forward` heading:

```
### Carry-forward
- From prompt 4: <Transport> reports `.ready` before the handshake completes, so do
  not start the registration timer here — wait for the first byte. The seam is
  `Transport.stateDidChange`.
```

Notes live in the destination rather than a separate file because this file is already
re-read at the start of every prompt. There is no second place to remember to check.

A note names a file and a symbol — one that says "think about X" is worth little; one
that names the seam is worth a session. A note is **deleted when the prompt that
received it runs**, and the fact that it was applied is recorded in `BUILD-LOG.md`.
`make check` fails if a note is still attached to a prompt the status line says is
complete.

For notes aimed at **stage 2 or later**, where there is no prompt to attach to, append
the same block under the relevant numbered item in `PLAN.md`. Same rule: consumed and
deleted when that item is built.

---

## Prompt 1 — Workspace scaffold and build discipline

**Item:** Workspace scaffold and build discipline.
**Branch:** `prompt-01-workspace-scaffold`

```
Scaffold the cargo workspace and make the discipline machinery real: after this
prompt, make build/test/fmt/lint run actual cargo commands and CI's build job runs
instead of skipping.

- Create a cargo workspace with five crates under crates/: havoc-ipc, havoc-core,
  supernaut-tui, havoc-transport, and supernaut (binary). Minimal lib.rs each
  (main.rs for the binary), with a one-line doc comment per crate stating what it
  owns, pointing at NORTH-STAR §4.2 and its naming amendment (Supernaut app,
  havoc engine).
- Declare the §4.2 dependency edges now, while the crates are empty: havoc-core,
  supernaut-tui, and havoc-transport each depend on havoc-ipc; supernaut depends
  on all four. The mechanical boundary check on Cargo.toml files needs real edges
  to guard, and adding them later invites adding them wrong.
- No external dependencies in this prompt. The allowlist gets its first entries in
  prompt 2, each with its decision entry.
- Replace the Makefile's placeholder build/test/fmt/lint bodies: cargo build
  --workspace; cargo test --workspace; cargo fmt --all -- --check; cargo clippy
  --workspace --all-targets -- -D warnings. Make the build itself warnings-as-
  errors too (workspace lints in the root Cargo.toml, so the flag travels with the
  repo rather than the shell). Keep check, check-tests, and hooks untouched.
- Add /target to .gitignore.
- Give supernaut's main something observable: print name + version and exit 0. A
  placeholder, but it makes `cargo run -p supernaut` a real smoke test from day
  one.

Acceptance: from a fresh clone, make build, make test, make fmt, make lint, and
make check all pass; cargo run -p supernaut prints the name/version line and
exits 0; CI's build job executes and is green.

Do not: add wire types (prompt 2 — the protocol surface deserves its own session
and review), add any dependency (prompt 2 onward, decision entry each), or add
banner/ASCII personality (stage 2's theme work — personality done twice is done
badly). Do not touch scripts/check-docs.sh beyond what the adaptation checklist
already did.
```

**Status:** complete. Shipped as ordered except: `make fmt` formats in place with the
`--check` moved into `make lint` (deviation recorded in `BUILD-LOG.md`), and the
review stripped a personality tagline, three `pub use` aliases, and the `license`
manifest field (now a Still-open item). Nothing left untested — the workspace is
empty scaffolding by design.

---

## Prompt 2 — IPC wire types

**Item:** IPC wire types.
**Branch:** `prompt-02-ipc-types`

```
Define the stage-1 wire surface in havoc-ipc: the typed vocabulary every other
crate codes against.

- Newtype IDs: NetworkId, BufferId, Seq, RequestId. Seq is the only ordering key
  (NORTH-STAR §4.6): it implements Ord, and it is ours.
- ServerTime is a distinct type that deliberately exposes no ordering — no Ord, no
  comparison helpers. NORTH-STAR §6.1 asks for timestamps to be structurally
  unavailable to sort paths; do it with the type system now, while there are zero
  call sites to migrate.
- Request enum (each carries a RequestId; exactly one Response comes back):
  Connect, Join, SendText, FetchBacklog { buffer, anchor, limit }, Search,
  SetReadMarker — the stage-1 surface only. Anchor = Before(Seq) | After(Seq) |
  Latest | AroundSearchHit(Seq), per §4.7. There is deliberately no "give me the
  buffer" request; do not add one, whatever prompt 9 seems to want.
- Event enum (unsolicited, broadcast to every client): ConnectionState,
  BufferCreated, MessageAdded, SearchResults, ReadMarkerChanged. MessageAdded
  carries buffer, seq, kind, nick, text, server_time, and remaining tags.
- Buffer/network models and a MessageKind enum (privmsg / notice / join / part /
  quit / mode / topic / nick / server) matching the storage `kind` column's needs,
  so prompt 3 does not invent a second enum.
- A protocol version constant and an empty capability-constants module — stage 4's
  handshake fills it; the constants living here is the point (§4.2).
- Dependencies: serde (derive) and a time type, per §4.2's near-zero allowance;
  ciborium as a dev-dependency for tests. Every Request/Response/Event round-trips
  through CBOR, and one test documents unknown-field tolerance — the schema-
  evolution story of §5.3, proved before there is a schema to evolve. Each
  dependency gets its BUILD-LOG decision entry and allowlist edit.

Acceptance: cargo test -p havoc-ipc green including CBOR round-trips; cargo tree
-p havoc-ipc shows nothing beyond serde and the time crate (plus dev-deps); make
check green. Live run: N/A — types only, and that is the honest answer.

Do not: implement any transport (prompt 5 owns the trait and the mpsc impl), add
storage row types that never cross the wire (prompt 3 — DB rows are core's private
business), or put behavior on these types beyond construction and serde. havoc-ipc
is data; logic here is the boundary eroding in its first week.
```

**Status:** complete. Shipped as ordered except: no `time` crate — `ServerTime` owns
a raw unix-millis `i64` (decision entry 2026-08-09), and the ipc dependency cap was
tightened to serde/bitflags with build-deps now counted (check fix + fixtures in the
same PR). The review froze eight wire-shape consequences into carry-forward notes on
prompts 3–9 and PLAN stages 2/4. Unknown-*variant* intolerance is deliberately
untested — it is real, and stage 4's handshake owns it (note on the plan item).

---

## Prompt 3 — Storage schema and migrations

**Item:** Storage schema and migrations.
**Branch:** `prompt-03-storage-schema`

```
Give havoc-core its storage layer: SQLite opened, migrated, and owned by a
dedicated thread — before any messages exist to store.

- The two questions that blocked this prompt are already settled — decision
  entries in BUILD-LOG.md dated 2026-08-09: buffer identity (two buffers;
  `UNIQUE(network_id, name)` stands; merged view is a client/query-side
  projection) and the migration mechanism (hand-rolled runner keyed on
  `PRAGMA user_version`, numbered SQL via `include_str!`, one transaction
  each). Execute them; do not re-decide, and do not reach for refinery.
- rusqlite, WAL on. Storage lives on its own thread behind a channel (NORTH-STAR
  §6.6): request messages in, replies via oneshot, and nothing else ever touches
  the connection. This is also what keeps prompt 8's search from ever blocking a
  render loop.
- Versioned migrations run at startup, from this first commit, including the
  initial migration (§6.10). Migration files are ordered and immutable once
  merged.
- Schema per §4.9: network, buffer, and message tables; message keyed (buffer_id,
  seq) WITHOUT ROWID; the partial unique index on (buffer_id, msgid); the
  (buffer_id, server_time) index. No FTS table yet — prompt 8 adds it as a
  migration, which is precisely why migrations exist before data does.
- The supernaut binary opens the store at startup (default XDG data dir,
  --data-dir override) and prints the schema version, so migration behavior is
  observable from outside a test.
- Tests: migrations create the schema on an empty file and are a no-op on a
  current one; a smoke insert/read through the storage-thread channel; an
  assertion on the exact schema shape, so drift from §4.9 is loud.

Acceptance: run supernaut twice against a fresh --data-dir — the first run creates
the DB and reports the migration, the second reports up-to-date; sqlite3
'.schema' shows the §4.9 shape; make check green.

Do not: implement seq assignment, dedup, or write batching (prompt 7 — the write
path deserves its own session with a flood test), FTS5 (prompt 8), or read-marker
logic beyond the column existing (prompt 9). Do not export storage row types
through havoc-ipc unless they genuinely cross the wire.
```

**Status:** complete. Shipped as ordered; the smoke insert/read path is
`#[cfg(test)]`-gated so no shipped write API pre-decides prompt 7, and the review
strengthened the schema assertion from a silhouette to column-exact, removed a
premature `synchronous` pragma, and raised seven carry-forward notes (prompts 5, 7,
8 and stage 6). `ensure_network`/`ensure_buffer` ship un-gated as the deliberate
buffer-creation seam — their conflict semantics are re-examined at prompt 7 via the
note there.

---

## Prompt 4 — Connection state machine, offline

**Item:** Connection state machine, offline.
**Branch:** `prompt-04-connection-state-machine`

```
Build the connection state machine in havoc-core — the crown jewels (NORTH-STAR
§5.2) — entirely offline, against a scripted fake transport.

- The SASL-mechanism question is already settled — decision entry in
  BUILD-LOG.md dated 2026-08-10: stage 1 implements PLAIN over TLS only, but the
  mechanism-selection state is shaped as an ordered preference list with
  per-mechanism states, so EXTERNAL (CertFP) drops in later without reshaping
  the machine. Execute that shape; do not implement EXTERNAL here.
- One actor per network, instantiated from a HashMap<NetworkId, _> even though
  exactly one network is configured (§6.9). Communication by channels only; no
  shared mutable state with anything.
- The actor codes against a small line-transport trait — lines in, lines out,
  connection lifecycle events. This prompt ships only a scripted fake
  implementing it; real sockets arrive in prompts 5–6. This split is what makes
  protocol bugs and I/O bugs separable forever after.
- irc-proto for Message parse/serialize only (§5.2) — decision entry + allowlist.
  Its Message is a plain data type; nothing else of the irc crate family comes in.
- States: CAP LS 302 → cap request → SASL → CAP END → registration → ISUPPORT
  parsing → autojoin → steady state, with the reconnect/resync state present as a
  named seam but unimplemented (backoff is prompt 6, CHATHISTORY is stage 5).
- Cap rules from §6.8, the part most likely to be subtly wrong: SASL completes
  before CAP END; CAP END is sent only after every requested cap has resolved
  (ACK or NAK, in any order); CAP NEW / CAP DEL are handled mid-session. Request
  server-time, message-tags, echo-message, batch, labeled-response, sasl;
  tolerate any subset being denied.
- SASL as explicit states with the decided mechanism set. Credentials enter the
  actor as opaque inputs; where they are stored is prompt 10's problem.
- ISUPPORT tokens parsed into the actor's network state; PING/PONG handled in
  steady state.
- The tests are the deliverable as much as the code: table-driven transcript
  fixtures (server lines in → expected client lines and final state) covering the
  happy path, cap denial, SASL failure, out-of-order cap resolution, and CAP DEL
  mid-session. Hand-written from real-server behavior now; prompt 6 captures live
  transcripts into this same corpus.

Acceptance: cargo test -p havoc-core green with the transcript suite; the
out-of-order-resolution transcript still ends registered with CAP END sent only
after the last cap resolves; make check green. Live run: N/A — no I/O exists yet
by design; prompt 5 is where this machine first touches a socket.

Do not: open real sockets, do DNS, or touch TLS (prompts 5–6 — I/O is a different
bug class), write anything to storage (prompt 7), implement reconnect timing or
backoff (prompt 6), or implement CHATHISTORY resync (stage 5 — the seam is
enough).
```

**Status:** complete. Shipped sans-I/O rather than trait-plus-fake — the machine is
a pure lines-in/lines-out function and the transcript tables *are* the scripted
transport; the actor loop and transport seam move wholesale to prompt 5 (deviation
recorded, constraint restated in that prompt's notes). Fail-closed SASL semantics
(`State::Failed`) added beyond the order, with the reasoning logged. The review
caught a real protocol bug — CAP NEW mid-negotiation escaped the CAP END gate —
fixed with its transcript. Six carry-forward notes raised (prompts 5, 6, 7; stage
6). Live run: N/A by design — no I/O exists; prompt 5 is where this machine first
touches a socket.

---

## Prompt 5 — Event bus, request handler, and debug CLI

**Item:** Event bus, request handler, and debug CLI.
**Branch:** `prompt-05-event-bus-debug-cli`

```
Wire the core together and give it its first driver: after this prompt a person
boots a local ergo and drives havoc-core over the typed havoc-ipc boundary from a
debug CLI, watching a connection register and a channel join arrive as events.

Three consequences the item does not authorize on its own. They are decided here
and called out first because two of them touch shipped surfaces:

- The client/core trait cannot live where both sides code against it. There is no
  Cargo edge between havoc-core and havoc-transport in either direction, so core
  cannot implement havoc_transport::ClientTransport. Decision: core exposes plain
  tokio channels; the adapter that dresses them as the trait lives in the binary,
  crates/supernaut/src/wiring.rs. Do not add the edge — the boundary-check
  amendment plus its fixtures is a larger change than the adapter, and the adapter
  is exactly what stage 4's UDS impl will do internally anyway. Note there are now
  two things called a transport and only one is in havoc-transport: the IRC
  line-transport trait lives in havoc-core (see below), which is also what
  check-docs.sh's boundary rule already implies by forbidding irc-proto in
  havoc-transport.
- Event::SearchResults is request-correlated inside a type whose doc says
  "broadcast to every attached client" (crates/havoc-ipc/src/lib.rs). Decision:
  the bus gets two lanes — a broadcast lane and a per-session directed lane — and
  correlated events go out directed only. Amend the Event doc comment in this
  prompt so the type stops asserting something false. This is a doc change, not a
  shape change; stage 4 would otherwise inherit an information-leak-shaped
  accident.
- Event::ConnectionState gains detail: Option<String>, with #[serde(default)].
  Machine::phase() folds State::Failed's reason away, and a debug CLI that cannot
  say why a connection died fails its own live run; routing that reason out of
  band would be a shortcut embedded mode can take and attached mode cannot
  (§4.3). Adding a field to a struct-like variant is precisely the evolution
  unknown_struct_fields_are_tolerated in crates/havoc-ipc/tests/roundtrip.rs
  proves, so PROTOCOL_VERSION stays 1 — bumping it would claim a break that test
  says is not one. Extend the roundtrip test with the populated field. Do not
  widen ConnectionPhase: its three variants remain the coarse projection.

The work:

- The bus, crates/havoc-core/src/bus.rs: Bus, ClientId, an explicit
  EVENT_CHANNEL_CAPACITY constant, Bus::broadcast(Event) over tokio broadcast and
  Bus::direct(ClientId, Event) over that session's mpsc. broadcast() carries a
  debug_assert rejecting correlated variants so the lane choice is structural
  rather than remembered, and a test asserts a second session never observes the
  first's SearchResults. A lagged broadcast receiver is surfaced to the client as
  a loud error, never a silent drop — a client whose projection quietly missed
  events is the §4.5 bug class that is undebuggable later.
- Request dispatch, crates/havoc-core/src/core.rs: Core::spawn(storage, ...) ->
  CoreHandle, and CoreHandle::attach() -> Session { id, requests, responses,
  events }. Sessions send on a shared mpsc::Sender<(ClientId, Request)>; the
  ClientId tag is what makes routing and per-session correlation possible, and it
  is the same thing stage 4's accept loop will do. RequestId is client-chosen and
  therefore unique only within a session — never key a core-global map on it.
  Every request produces exactly one Response, including ResponseBody::Error for
  an unknown network or buffer: a debug CLI that hangs teaches nothing.
- The IRC line transport, crates/havoc-core/src/connection/io.rs: the trait prompt
  4 deferred — connect, send_line, next_line, close — plus a tokio TCP impl doing
  CRLF framing. It lives in havoc-core, per the prompt-1 boundary above. The
  scripted-fake role stays with the transcript tables; do not build a second fake.
- The actor, crates/havoc-core/src/connection/actor.rs: one tokio task per
  network owning its Machine by value and its transport, selecting over inbound
  lines and inbound commands. Machine::handle_line keeps its Vec<String>
  signature — the nine-transcript corpus in crates/havoc-core/tests/state_machine.rs
  asserts on exactly that, and widening it to carry state changes churns the whole
  corpus to buy what a diff buys for free. The actor snapshots phase() before each
  line, compares after, and emits ConnectionState on change, reading state() for
  the Failed reason to fill detail. Keep that diff helper keyed on state(), not
  phase(): prompt 6's backoff must distinguish Failed from Disconnected and
  phase() folds them.
- Networks (crates/havoc-core/src/connection/mod.rs) is re-typed to hold actor
  handles (command sender + JoinHandle) rather than Machines, since the actor task
  now owns its Machine outright and no state is shared (§5.5). Update the map test
  at the bottom of state_machine.rs into an actor-level test rather than keeping a
  second map — one map from commit one is the whole §6.9 point.
- A raw-line trace sink on the actor, off by default, enabled by --trace-irc,
  writing `>> line` / `<< line` to stderr. This is diagnostics, not wire state,
  and it is deliberately the capture mechanism prompt 6 harvests live transcripts
  from — shape the format so a capture is mechanically convertible to Step rows.
  Do not reach for tracing or log: two eprintln! calls behind a flag is the whole
  need, and a logging framework arrives when something wants structured filtering.
- havoc-transport, crates/havoc-transport/src/lib.rs: ClientTransport with async
  fn send(Request) and recv() -> Option<Incoming>, where Incoming multiplexes
  Response and Event; TransportError; and an InProcess impl over tokio channels
  carrying typed values with no serialization anywhere. One impl exists, so the
  trait is not made dyn-compatible; if stage 4 wants runtime selection it wraps
  both impls in an enum rather than boxing. Incoming is transport-local — whether
  a framed union needs a wire type in havoc-ipc is stage 4's call, not this one.
- The CLI, crates/supernaut/src/main.rs plus a session module. One subcommand,
  `session`, taking --host --port --nick --join and the mandatory
  --allow-plaintext; it boots core plus the in-process transport and reads
  newline commands from stdin: connect, join <channel>, send <channel> <text>,
  wait registered [secs], wait buffer <name> [secs], quit. Every event received
  prints one greppable line to stdout (`event connection-state network=1
  phase=registered`), every response prints `ok <id>` or `error <id> <message>`.
  The command flow is built on the event stream, not on the reply:
  ResponseBody::Ack means accepted and nothing more, so `join` returns when
  BufferCreated arrives and `send` resolves a channel name through a session-local
  name→BufferId map built from BufferCreated events — which is exactly the
  projection the TUI will keep. `wait` exists so live-run.sh never sleeps; a
  timeout exits non-zero with the deadline named, and every later prompt inherits
  the determinism.
- Plaintext is a loud explicit opt-in (§2.3): without --allow-plaintext the
  session refuses to start, and even with it the host must resolve to loopback.
  TLS is prompt 6; keeping this prompt's failures on-box keeps them about wiring
  rather than certificates.
- No subcommand keeps prompt 3's behavior byte for byte: `supernaut --data-dir
  <path>` still prints name/version and the history line and exits 0. That is
  prompt 3's documented acceptance and it survives the rewrite.
- Dependencies, both with BUILD-LOG decision entries. tokio (already on
  DEP_ALLOWLIST) arrives in havoc-core (sync, net, time, rt, io-util),
  havoc-transport (sync), and supernaut (rt-multi-thread, macros) — explicit
  feature lists, never "full". clap with derive is added to supernaut only and to
  DEP_ALLOWLIST: prompt 3's "one flag, no dependency justified" rationale expires
  the moment subcommands exist, and prompts 8, 9 and 10 each add verbs to this
  same grammar. Do not add tokio-util — LengthDelimitedCodec is stage 4's framing.
  An allowlist edit is not a check change, so no scripts/test-checks.sh fixtures
  are owed here.
- scripts/live-run.sh, born here and reused by every later prompt: set -euo
  pipefail, shellcheck-clean. It uses $ERGO_BIN when set, otherwise downloads a
  version-pinned ergo release into a gitignored .cache/ergo/ and verifies a
  recorded sha256; generates an ergo config in mktemp -d on a random free port;
  runs two supernaut sessions against it, the second as the other party; asserts
  on the greppable output; and tears everything down on trap. ergo is a test
  harness acquired by this script — never a Cargo dependency, never on
  DEP_ALLOWLIST. Add the cache dir to .gitignore.
- No message rows are written and no MessageAdded event is emitted. Connect
  resolves its NetworkId through Storage::ensure_network and join emits
  BufferCreated from Storage::ensure_buffer, both wrapped in
  tokio::task::spawn_blocking at the two call sites, because the Storage handle in
  crates/havoc-core/src/storage/mod.rs blocks on recv() and an actor task calling
  it inline stalls the executor. Two call sites is not three: do not build an
  async facade over the job channel, which is prompt 7's decision to make with the
  flood test in front of it.

Acceptance: on a machine with no ergo installed, run scripts/live-run.sh — it
fetches the pinned binary, writes a config into a temp dir, boots ergo on a random
free port, and drives two sessions against it while you watch the first print
`event connection-state ... phase=connecting`, then `phase=registered`, then
`event buffer-created ... #supernaut`, and the second's send land; the script exits
0 and leaves no ergo process and no temp dir behind. Run it once more with
--trace-irc and read the `>> CAP LS 302` / `<< :ergo CAP * LS ...` exchange, which
is the capture prompt 6 harvests. Kill ergo mid-session and the CLI prints
phase=disconnected once and stops rather than spinning. Separately, `supernaut
--data-dir <tmp>` with no subcommand still prints the name/version and history
lines and exits 0; cargo test --workspace and make check are green.

Do not: TLS, rustls, or DNS against a real network (prompt 6 — I/O bugs and
certificate bugs are different classes and this prompt's failures should all be
wiring); SASL against a live server (prompt 6 — the SASL states are already
transcript-tested and a live failure here would be indistinguishable from a wiring
bug); reconnect timing or backoff (prompt 6 — on disconnect the actor emits
Disconnected and stops, which is the seam, not the policy); persisting anything
seen, seq assignment, or MessageAdded (prompt 7 — the write path deserves its own
session with a flood test); FTS or a search verb (prompt 8), backlog or mark-read
verbs (prompt 9 — the stdin grammar grows there, one prompt per verb set); a
config file (prompt 10 — connection parameters are flags here, and a config
surface designed before its features calcifies); CBOR framing or any serialization
on the transport (stage 4 — in-process is typed values); and no Cargo edge between
havoc-core and havoc-transport in either direction.
```

**Examined for a split and left whole**, because bus, dispatch, transport trait, and
CLI are one wiring seam — none is testable in anger without the others. Revisit if the
plain-TCP connector balloons; it can move wholesale into prompt 6.

**Status:** complete. Shipped per the JIT detail with recorded deviations: `Directed`
carries responses too (the ordered `direct(ClientId, Event)` signature could not carry
them), `recv()` returns `Result` not `Option` (the order was internally inconsistent
with its own loud-lag demand), `join` is fire-and-forget with `wait buffer` as the
completion verb, and the live-run script polls twice where no event exists yet to wait
on (prompt 7 deletes the polls). The review's eleven carry-forward proposals were all
adopted (prompts 6/7/8/10, PLAN stages 2/4). Live run: passed, all six assertions,
first try.

---

## Prompt 6 — Live connection, TLS, and reconnect

**Item:** Live connection, TLS, and reconnect.
**Branch:** `prompt-06-live-connection`

```
The real network path: after this prompt the debug CLI dials TLS by default and
verifies it, authenticates over live SASL against a local ergo, survives an ergo
restart through jittered backoff inside the actor with no operator action, and
feeds the first live-captured transcripts back into prompt 4's corpus.

Two conflicts between the plan and this prompt's notes, surfaced rather than
resolved silently:

- PLAN's testing strategy promises transcript "fixture files"; what shipped is
  inline Rust Step tables, and the note on this prompt forbids a second corpus
  format. Decided: the corpus stays inline Rust and captures are converted into
  Step rows; amend PLAN's testing-strategy wording in this PR (a stale doc is
  fixed in the commit that staled it). Migrating nine transcripts to data files
  would churn the durable asset mid-stage to buy format purity nobody asked for.
- The plan item says backoff is driven "through the state machine's seam", but
  the Failed/Disconnected distinction does not survive the ActorReport boundary
  — outside the actor task, a refused connect and a fatal SASL failure are both
  Disconnected-with-detail. The notes win: retry policy is written inside
  run() in crates/havoc-core/src/connection/actor.rs, reading Machine::state();
  the plan's sentence is satisfied by the policy consuming the seam
  (State, on_disconnect), not by the policy living in the machine.

The work:

- TLS transport, crates/havoc-core/src/connection/io.rs: TlsLineTransport over
  tokio-rustls (rustls and tokio-rustls are already on DEP_ALLOWLIST), plus a
  two-variant connector enum so run() holds one transport type — LineTransport's
  RPITIT methods make an enum cheaper than generics here. Root store:
  webpki-roots, with --tls-ca <pem> appending one extra anchor via
  rustls-pki-types' pem support. Both crates are new to the allowlist: one
  BUILD-LOG decision entry for the trust story covers both edits, and an
  allowlist edit is not a check change, so no test-checks fixtures are owed.
  webpki-roots over the platform keychain because it is deterministic across
  machines and adds zero OS-integration code; revisit only if a real
  enterprise-CA user appears. There is deliberately no
  --tls-insecure-skip-verify and never will be: --tls-ca keeps verification ON
  against an anchor you name, which is the §2.3 loud-opt-in shape — you say
  what you trust, you never switch trust off.
- Plumb-through: NetworkSettings in crates/havoc-core/src/core.rs and
  ActorSpawn gain security: Tls { server_name, ca_file } | Plaintext. DNS is
  already real — TcpStream::connect resolves names — so "real DNS" costs
  nothing beyond pointing it at a real hostname.
- CLI, crates/supernaut/src/session.rs: TLS becomes the default; delete the
  "TLS arrives in prompt 6" refusal. --allow-plaintext remains the loud opt-in
  and keeps its loopback-only rule — plaintext off-box is the trap, not a
  feature. New flags: --tls-ca <path>, and --sasl <account> taking the password
  from SUPERNAUT_SASL_PASSWORD — argv is world-readable in ps, so a --sasl-pass
  flag would put a secret where CLAUDE.md forbids it; env is the debug-grade
  middle until prompt 10's keyring. A session running with an extra CA or with
  plaintext prints one loud line saying so at startup.
- Reconnect, inside run() in crates/havoc-core/src/connection/actor.rs:
  restructure into an outer attempt loop wrapping the connect/register head — a
  fresh Machine::start per attempt, never a reset() (there is no re-arm path,
  by design). After each attempt ends, read Machine::state():
  State::Failed { reason } reports Disconnected with the reason and returns —
  fail-closed SASL is never retried, and this is exactly why the check must be
  state(), not phase(), which folds Failed into Disconnected. Every other loss
  (refused connect, EOF, read/send error) reports Disconnected with detail,
  then backs off: exponential 1s doubling to a 60s cap, ±50% jitter derived
  from SystemTime subsec nanos — a rand dependency for one jitter fails the
  allowlist's own justification bar — with the counter reset once an attempt
  reaches Registered. Each attempt re-emits Connecting with a retry detail,
  so the loop is greppable in a live run; the stage-2 carry-forward already
  commits the TUI to deduping repeated Connecting. Commands arriving while
  disconnected are dropped: autojoin re-fires on the fresh machine, and a
  PRIVMSG delivered seconds after a reconnect is a surprise, not a feature. A
  closed command channel exits the actor even mid-backoff-sleep.
- Loop tests, alongside the actor test in
  crates/havoc-core/tests/state_machine.rs, under
  #[tokio::test(start_paused = true)] so backoff timing is asserted without
  wall-clock cost: (1) a refused connect yields Connecting, Disconnected,
  Connecting again — retry proven; (2) a scripted tokio TcpListener that speaks
  the CAP LS / NAK sasl lines yields Disconnected with the SASL reason exactly
  once and the task ends — Failed is never retried. The listener is a
  socket-level peer, not a second protocol fake: I/O-layer behavior is
  precisely this prompt's bug class.
- scripts/live-run.sh grows the TLS + SASL + reconnect story: generate a
  self-signed cert for the dialed name; add a TLS listener with cert/key paths
  inside $WORK; keep a plaintext loopback listener for the harness's own
  pre-registration exchange. Pre-register the alice account over nc (NickServ
  REGISTER after 001, recognisably-fake password), poll for success, fail
  loudly otherwise. Sessions A and B then connect over TLS with --tls-ca and no
  --allow-plaintext; A adds --sasl alice. Assert the 903 SASL-success line in
  a.trace. Then the reconnect proof: kill ergo, restart it on the same port
  with the same config (the datastore in $WORK persists the account), send
  `wait registered` down A's fifo, and assert a.out shows phase=disconnected, a
  retry line, and a second phase=registered. That sequence is NORTH-STAR's
  invisible-reconnect promise observed end to end.
- Capture into the corpus: scripts/trace-to-steps.sh (set -euo pipefail,
  shellcheck-clean) converts a trace file into draft Rust Step rows. It must
  filter on the `>> ` / `<< ` prefixes only — the trace shares stderr with
  session diagnostics — and its header comment must warn that `>> ` covers
  user-command lines as well as machine replies, so the output is a draft a
  person reviews and pastes, never text committed blind. Add the captured ergo
  TLS+SASL registration as a new test (live_ergo_registration) in
  crates/havoc-core/tests/state_machine.rs. Redaction rule: never paste a live
  AUTHENTICATE payload from a real account; the harness credentials are
  recognisably fake by construction, so the ergo capture is safe.
- Libera.Chat spot-check, manual and once, deliberately outside the scripted
  loop (PLAN's testing strategy keeps it there): `session --host
  irc.libera.chat --port 6697 --nick <scratch> --join <quiet channel>` with no
  --tls-ca and no --allow-plaintext — the webpki-roots default path is exactly
  what is being proven. SASL at Libera only if a registered account exists;
  recording "TLS + registration verified, SASL ergo-only" is the honest entry
  otherwise. Its capture may join the corpus via the same converter if clean.
- live-run.sh stays scoped to the dogfood Mac (the pinned ERGO_PLATFORM
  stands). Platform detection lands when CI actually runs this script; a
  download matrix nobody executes is speculative surface.

Acceptance: run scripts/live-run.sh and watch session A register over TLS with
SASL (the AUTHENTICATE exchange and 903 visible in a.trace) and B's message
land; then — with no operator action — watch ergo die and restart while A
prints phase=disconnected, a connecting retry line, and a second
phase=registered; the script exits 0 and leaves no ergo process and no temp
dir behind. Run scripts/trace-to-steps.sh over the kept capture and read valid
Step rows matching the exchange you just watched; cargo test --workspace is
green including live_ergo_registration and the two paused-time backoff tests,
with the wrong-password test proving a SASL failure is reported once and never
retried. Separately, once, connect to irc.libera.chat:6697 with the stock root
store, register, join, and record the result in BUILD-LOG's Live run section.
make check green.

Do not: SASL EXTERNAL / CertFP (stage 6 menu — the mechanism slot is shape
only and the machine work is budgeted there, not here); any skip-verify flag
or cert-management surface (--tls-ca is the entire local-trust story; a
verification off-switch is the §2.3 trap and must never exist to be grabbed);
CHATHISTORY resync on reconnect (stage 5 — nothing exists to merge into yet);
storage writes or MessageAdded (prompt 7 — the write path gets its own session
with a flood test); any TCP listener or inbound connection (§2.4; stage 6 menu
at most); config file or keyring (prompt 10 — flags and one env var are the
debug surface until then); platform detection in live-run.sh (arrives with CI
coverage); and no tracing/log framework — the two eprintln! calls remain the
whole trace story, because the converter now depends on exactly that format.
```

**Examined for a split (TLS vs reconnect) and left whole**, because both edit the
connect head of the same `run()` function; splitting them means two prompts editing
one function. Revisit if the live SASL pre-registration harness fights back — the
capture-and-corpus work could trail as its own small prompt without touching the
connect path.

**Status:** complete. Shipped per the JIT detail with recorded deviations: ergo's
`mkcerts` was replaced by an openssl cert for the *dialed* name (ergo requires a
dotted server name, so mkcerts would mint the wrong CN — the detail's own named
fallback), and the 903 assertion matches ergo's `*`-nick reply shape. Live run:
all eleven assertions passed, including the invisible-reconnect proof; Libera.Chat
registered over TLS with stock webpki roots (SASL ergo-only, honestly recorded).

---

## Prompt 7 — Message ingestion, identity, and batched writes

**Item:** Message ingestion, identity, and batched writes.
**Branch:** `prompt-07-ingestion`

```
The write path: after this prompt every line the actor sees on a live connection
lands in SQLite exactly once — seq assigned at insert, msgid dedup enforced by
the partial unique index, commits batched on a ~100ms/256-row window — and comes
back out as a MessageAdded event a person watches in the debug CLI and a flood
measures.

Three places the notes and the item pull apart, surfaced rather than resolved
silently:

- The item's phrasing "actor events flow through the bus into the storage
  thread" cannot be built literally. The broadcast bus is allowed to lag and
  drop (loudly — that is its prompt-5 contract), and a lane that may drop can
  never be the history write path. Ingest flows actor → core reports → storage
  job queue; the bus carries only the post-insert MessageAdded. The sentence is
  satisfied in effect — events reach storage, then every client — not in
  topology.
- The content-hash fallback is unobservable in this prompt's live run: ergo tags
  every message with msgid, so the fallback ships proven by unit tests against
  real temp-file SQLite only, and its live proof waits for stage 5's bouncer
  work. Recording that honestly beats contriving a fake tagless server nothing
  in the plan asked for.
- The live-run note says the event-shaped polls "get deleted", but a
  fifo-driven background session still needs one sync primitive the script can
  see. Decided: what gets deleted is grepping for raw event patterns (the
  PRIVMSG-in-trace loop and the registered-count loop); the script's remaining
  loops sync only on the wait verbs' own `waited ...` echo lines — the pattern
  the surviving `waited buffer` loop already set.

The work:

- Parse once, in the actor — the seam decision. Machine::handle_message gains
  pub visibility; handle_line stays as the parse-then-delegate wrapper so the
  transcript corpus in crates/havoc-core/tests/state_machine.rs is untouched.
  The actor parses each inbound line exactly once (unparseable lines are traced
  and skipped, preserving the machine's ignore rule), feeds the machine the
  &Message, then feeds the same &Message to a new classifier in
  crates/havoc-core/src/connection/ingest.rs. Delete our_join and
  ActorReport::JoinedChannel outright: our confirmed JOIN now arrives as an
  ingested Join message and buffer creation rides ingestion. The machine grows
  no message output — widening handle_line's Vec<String> would churn the whole
  corpus to buy what the actor's single parse already holds.
- The ingest type is core-private: wire Message has no msgid berth (excluded
  from tags and absent as a field, deliberately), and rows are core's business
  per prompt 3's fence — do not amend havoc-ipc. `Ingest { target, kind, nick,
  account, text, server_time, msgid: Option<String>, tags }`, built by the
  classifier from the parsed message plus machine context (our nick). Kinds
  ingested: Privmsg, Notice, Join, Part, Topic, Mode — everything addressed to
  a named target. QUIT and NICK fan out to every shared channel, which needs
  membership state no prompt has built: skipped, with a carry-forward to PLAN
  stage 2, where nick completion needs the same tracking. Numerics stay
  un-ingested; a server-console buffer would be new scope, not a silent kind.
- server-time: a strict hand-rolled parser for the one grammar the IRCv3 spec
  pins (`YYYY-MM-DDThh:mm:ss.sssZ`, days-from-civil arithmetic, unit-tested
  against known vectors); anything else falls back to local receipt time. A
  time dependency for one fixed format fails the same allowlist bar the jitter
  rand did.
- Identity, all of it enforced in the storage layer (§6.4): seq comes from a
  per-buffer counter cached in the storage thread, seeded from MAX(seq) on
  first touch — sound because that thread is the only writer — and incremented
  only when a row actually inserts, so per-buffer seq is contiguous and the
  flood can assert COUNT(*) == MAX(seq). Dedup is INSERT ... ON CONFLICT
  (buffer_id, msgid) DO NOTHING with the changed-row count deciding everything
  downstream: 0 changed → no seq consumed, no event emitted. MessageAdded
  idempotency across reconnect replays is thereby a consequence of the index,
  never an application-layer memory. Tagless messages get a synthetic msgid
  `fnv:<hex>` — FNV-1a 64 written inline (ten lines; sha2 fails the dependency
  bar, and std's DefaultHasher is unstable across releases where this value is
  disk format) over (nick, text, server_time / 30_000) — stored in the same
  msgid column so the one partial index enforces both identities. The cost is
  accepted and unit-tested: identical (nick, text) inside one 30s bucket on a
  tagless server collapses to one row — §6.5's replay-safety trade; stage 5
  revisits it against a real bouncer.
- tags land as the CBOR the schema already promises: ciborium becomes a runtime
  dependency of havoc-core (already on DEP_ALLOWLIST; the crate move still gets
  its decision entry). An empty map stores NULL so "no tags" stays queryable.
- The lane, resolving both decoupling notes: ActorReport gains Message(Ingest);
  handle_report maps the caller NetworkId to its row id and forwards with a
  plain non-blocking send into the storage job queue — zero awaits on the
  ingest path, so the flood can no longer serialize through the core select
  loop. Job::Ingest { network (caller id, echoed through untouched), row, item,
  outcome } is fire-and-forget; the std mpsc is unbounded, deliberately —
  history is never dropped for backpressure, and bounded-queue design waits for
  dogfood evidence. `NetworkRow` is a new core-private newtype returned by
  ensure_network, so the two-id-spaces-both-equal-1 accident class dies at
  compile time; wire BufferIds are storage rows by design and stand.
- Batching, in the storage thread's run loop (crates/havoc-core/src/storage/):
  pending rows flush as one transaction at 256 rows, at ~100ms after the first
  pending row (recv_timeout against the deadline), when any non-ingest job
  arrives (reads must see writes), and at Shutdown — queue order puts pending
  ingests ahead of the Shutdown job, so quit loses nothing. Inside the
  transaction the thread ensures the buffer itself (it owns the connection;
  kind derives from the target's leading #/& → channel, else query —
  deterministic per name, so ensure_buffer's ON CONFLICT DO NOTHING can never
  silently re-kind a buffer from this path; kind is immutable after creation
  and a mismatch is reported loudly, never swallowed), assigns seqs, inserts,
  then sends one IngestOutcome back on the tokio sender the job carried
  (blocking_send is fine from a std thread): created buffers first, then
  inserted rows with their seqs.
- Core, on outcome: update the id/name maps, emit BufferCreated only on first
  touch per core instance — across an ergo restart the replayed autojoin
  re-ensures the same row and emits nothing, which is exactly the idempotency
  the reconnect note demands — then MessageAdded per inserted row, in order.
- Own messages are recorded via echo-message only (requested since prompt 4;
  ergo and Libera grant it): one identity source, dedup-safe by msgid. Servers
  without echo-message will not log own sends yet — a known limitation recorded
  in BUILD-LOG; dogfood decides whether it earns work.
- PRAGMA synchronous belongs to this flood harness: run the flood at the
  default (FULL) and at NORMAL, record both numbers in the BUILD-LOG entry, and
  ship `synchronous=NORMAL` in Storage::open unless the numbers argue otherwise
  — in WAL, NORMAL fsyncs at checkpoint rather than per commit, keeps committed
  data across an app crash, and concedes only the power-loss tail, the right
  side of the trade for chat history. Storage::open grows a trace flag (the
  session passes --trace-irc's value through) and the thread prints one
  `storage commit rows=N` line per batch to stderr — measurement is grep, not a
  logging framework; the two-eprintln stance stands.
- CLI, crates/supernaut/src/session.rs: `wait message <buffer> [count] [secs]`
  (count defaults to 1), counting MessageAdded per BufferId through the same
  name→id projection the other verbs use; a timeout still names its deadline
  and exits non-zero.
- scripts/live-run.sh: delete the a.trace PRIVMSG poll and the
  registered-count poll. After B runs, A's fifo gets `wait message #supernaut
  1 10`; after the restart the script syncs on the second `waited registered`
  echo. New flood segment before the restart: a third scripted session (carol)
  — connect, join #flood, then 500 numbered `send` lines piped upfront,
  sleep-free by construction because the session processes commands serially.
  A joins #flood first and runs `wait message #flood 500 60`. The
  harness-level loops (ergo listening, raw-nc pre-registration) stay, honestly
  commented as outside any session.

Acceptance: run scripts/live-run.sh and watch B's line arrive at A as `event
message-added` rather than only in the raw trace; watch the flood — 500 sends
from carol, A's `waited message #flood`, and a `storage commit rows=N` count in
A's stderr well under a tenth of 500 (record the number, and the FULL/NORMAL
pair, in BUILD-LOG); watch ergo die and restart with exactly one
`event buffer-created ... name=#supernaut` across both registrations. After the
script quits the sessions, run sqlite3 against A's data dir and read
journal_mode wal, COUNT(*) equal to MAX(seq) for #flood, and each flood line
present exactly once — kill-and-lose-nothing observed from outside the process.
cargo test --workspace green, including: same msgid twice → one row and one
MessageAdded; a tagless (nick, text, bucket) pair → one row; the server-time
parser vectors; and the transcript corpus untouched. make check green.

Do not: FTS indexing or sync (prompt 8 — external-content against WITHOUT
ROWID is a schema question already noted there, and deciding it under flood
pressure is how it goes wrong); backlog reads or read markers (prompt 9 — the
read path deserves its own session against a populated store); replay/resync
scenarios beyond what ergo produces today, including CHATHISTORY (stage 5 — a
real bouncer is the only honest test); QUIT/NICK fan-out or membership
tracking (carry-forward to PLAN stage 2, where nick completion needs the same
state); a server-console buffer for numerics (new scope — a PLAN item if ever,
not a silent ingest kind); an async facade over StorageClient (the ingest lane
is the only bridge added here; connect's single spawn_blocking site stays —
one site is not three); any new dependency beyond ciborium (no sha2 for one
hash, no time for one fixed grammar, no rand — the jitter set the bar); and no
wire changes — Message and MessageAdded stand as shipped, msgid stays off the
wire because dedup is storage's business, and PROTOCOL_VERSION stays 1.
```

**Examined for a split (identity vs batching) and left whole**, because both are the
single insert path, and the flood harness that proves batching is the same one that
proves dedup. Revisit if the content-hash fallback design stalls the session — it can
trail as its own small prompt.

**Status:** complete. Shipped per the JIT detail with recorded deviations: the ON
CONFLICT target needed the partial index's WHERE clause spelled out; the harness
taught two real lessons — a session's `quit` races in-flight requests (carol lost
~300 sends to the runtime drop until she drained via her own echo count; noted on
prompt 9), and a dead session's fifo SIGPIPEs the whole script unless the script
ignores PIPE (bash skips the EXIT trap on signal death, leaking ergo). Flood: 500
lines → 8 commits at both FULL and NORMAL (~6s wall each); NORMAL shipped per the
a-priori argument, numbers recorded. All 16 live assertions green.

---

## Prompt 8 — Full-text search

**Item:** Full-text search.
**Branch:** `prompt-08-search`

Search that is actually search (§7.1): the FTS5 external-content table added by
migration and kept in sync with `message`; the `Search` request with structural
filters — `from:`, `in:`, time range — planned against real indexes; results delivered
as events (never blocking anything, §6.6); a debug CLI `search` command. Seed a corpus
in the live run and show search staying instant.

**Examined for a split (index vs query language) and left whole**, because the filter
grammar and the index shape must be designed together or the filters end up
unindexable. Revisit if the filter grammar grows past the three filters named here —
anything more is stage 2+ polish.

Do not: jump-to-context around a hit (prompt 9 — AroundSearchHit belongs to the
backlog API), saved searches / virtual buffers (§7.1 extension, stage 6 menu), or any
ranking work beyond FTS5 defaults.

*To be written out before it starts.*

### Carry-forward

- From prompt 2: **Search's wire shape is frozen: verbatim string in, events
  out.** `RequestBody::Search { query: String }` in `crates/havoc-ipc/src/lib.rs`
  (core parses `from:`/`in:`) and no search variant on `ResponseBody`. Structured
  filter fields or a synchronous results response are wire changes — design the
  filter grammar as core-side parsing of a plain string with no wire cooperation.
- From prompt 7: **a failed batch used to poison the writer caches; FTS sync is
  the first realistic new error source in that transaction.** `write_batch` in
  `crates/havoc-core/src/storage/exec.rs` now clears `WriterState.buffers` and
  `next_seq` on rollback — the FTS-sync writes this prompt adds join exactly
  that transaction, so its error story must keep the invalidation correct (and
  test a mid-batch failure).
- From prompt 7: **flush-before-reads guards only the storage thread's own job
  queue.** A second read-only WAL connection (the option weighed below for
  search) bypasses the flush barrier and can read up to 256 rows / ~100ms
  stale — choosing it silently forfeits read-your-writes. Either search rides
  the job queue, or the prompt accepts and documents the staleness window.
- From prompt 5: **request dispatch discards the `ClientId` before the handler
  runs.** `handle_request` in `crates/havoc-core/src/core.rs` receives neither
  the client id nor the bus; SearchResults must go out `Bus::direct(client, ..)`
  and `Bus::broadcast`'s debug_assert fires if routed the easy way. Plan the
  signature change into the prompt text.
- From prompt 3: **`message` is WITHOUT ROWID — FTS5 external-content's rowid
  contract cannot hold.** External-content FTS5 (`content='message'`) syncs by
  `content_rowid`, and `message` in `crates/havoc-core/migrations/0001_init.sql`
  has no rowid at all. Design the FTS migration around contentless-delete FTS5 or
  an explicit docid mapping *before* the session starts — this is a schema-design
  question, not an implementation detail.
- From prompt 3: **one connection, one FIFO — search queues behind batched
  writes.** `run` in `crates/havoc-core/src/storage/mod.rs` drains a single job
  queue on the single connection, and prompt 7 holds that thread in ~100ms write
  transactions. WAL permits a second read-only connection; decide deliberately
  whether search gets its own reader, in the prompt text.

---

## Prompt 9 — Windowed backlog and read markers

**Item:** Windowed backlog and read markers.
**Branch:** `prompt-09-backlog-read-markers`

`FetchBacklog` for real: all four anchors including `AroundSearchHit`, `limit` capped
server-side regardless of what the client asks (§6.3); read markers (`last_read_seq`)
set and read per buffer, emitted as events; debug CLI `backlog` and `mark-read`
commands. There is still no "give me the buffer" request — the fence of §4.7 holds
here or nowhere.

**Examined for a split (backlog vs read markers) and left whole**, because read
markers alone are a session only if multi-client reconciliation is in scope, and it is
not (Still open; stage 6). Revisit if AroundSearchHit's context-window semantics turn
out to need design work with prompt 8's hit shape.

Do not: read-marker reconciliation between clients or `draft/read-marker` upstream
(stage 6, blocked on the Still-open), pagination UI concepts (stage 2), or raising the
server-side cap for any caller.

*To be written out before it starts.*

### Carry-forward

- From prompt 2: **backlog replies are a bare Vec — no has-more signal, no hit
  marker.** `ResponseBody::Backlog { messages }` and `Anchor::AroundSearchHit(Seq)`
  in `crates/havoc-ipc/src/lib.rs`: with `limit` capped server-side, a short Vec is
  ambiguous between window-exhausted and history-start, and an around-hit window
  carries no indication of where the hit sits. If this prompt needs either signal,
  it is a wire change to a shipped type — settle that while writing the prompt
  detail, not mid-session.

---

## Prompt 10 — Network config and credentials

**Item:** Network config and credentials.
**Branch:** `prompt-10-config-credentials`

The config file (TOML, §5.8): networks, nick, autojoin — as seed data, per whatever
the config-vs-runtime-state Still-open decides (it blocks this prompt). Credentials
via `keyring` where available with an encrypted-file fallback; never plaintext secrets
in the config file, including for SASL (§5.8). The debug CLI runs from config alone,
and the prompt ends with the stage-1 acceptance run: connect, join, log, kill,
restart, search.

### Carry-forward

- From prompt 6: **the `SUPERNAUT_SASL_PASSWORD` bridge and the
  `--sasl`/`--tls-ca` flags are installed base this prompt must replace, not
  extend.** `crates/supernaut/src/session.rs` hardcodes `[Plain]` and reads the
  env var; `NetworkSettings.security` carries `{ server_name, ca_file }`. The
  TOML schema must own per-network tls_ca and the SASL account; the keyring
  path must delete the env var (replace, don't deprecate); and the
  loopback-only plaintext rule needs a new home once config supplies the host.
- From prompt 5: **the debug session hardcodes network identity that config must
  replace.** `const NETWORK: NetworkId = NetworkId(1)` in
  `crates/supernaut/src/session.rs`, and the storage network name is
  `debug-<host>` — `ensure_network` keys on name, so a reused data dir accretes
  one row per host string. Config becomes the authority for caller ids and
  stable names; decide whether `debug-*` rows are migrated or abandoned.

**Examined for a split (config vs credentials) and left whole**, because config
parsing without the credential story ships exactly the plaintext-password trap the
north-star forbids — the two must land together or the insecure version becomes the
installed base.

Do not: interactive first-run flow (stage 2 item 7 — it needs a TUI), theme
configuration (stage 2 item 6), or multi-network config surface beyond what the
schema already permits (stage 5 item 1 — one network is stage 1's scope).

*To be written out before it starts.*

---

<!--
Once a prompt completes, append its outcome under the block, in this shape. Keep it to
two or three sentences and put the reasoning in BUILD-LOG.md, not here:

**Status:** complete. <What shipped differently from the block above, and where that was
recorded.> <Anything deliberately left without a test, and why.>
-->
