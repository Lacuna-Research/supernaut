# Stage 1 — The Prompts

**Status:** 12/12 complete. Stage 1 closed — retrospective in BUILD-LOG.md; stage 2 planning is next.

<!-- 12 must match the STAGES array in scripts/check-docs.sh — change both together.
The line is machine-read; `make check` fails if they disagree. Prompts 9 and 10
were split into 9a/9b/10a/10b (2026-08-10 decision entry) — labels, not
renumbering: the checker counts headings by position. -->

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

```
The flagship, headless (§7.1): after this prompt a person searches all history
instantly from the debug CLI — `search from:bob in:#supernaut "deployment
failed"` — with the FTS index arriving as migration 0002 and backfilling
everything prompt 7 already wrote, which is the payoff migrations-first was
built for.

One conflict between the plan item and this prompt's notes, surfaced rather
than resolved silently:

- The plan item says "FTS5 external-content table", but external-content sync
  keys on content_rowid and message in
  crates/havoc-core/migrations/0001_init.sql is WITHOUT ROWID — there is no
  rowid for the contract to hold on. The note wins: migration 0002 creates a
  plain self-contained FTS5 table instead, and the PLAN item's wording is
  amended in this PR (a stale doc is fixed in the commit that staled it).
  Contentless-delete FTS5 with a docid-mapping table was weighed and
  rejected: it saves the duplicated text bytes at the price of a second
  table, a docid allocator, and a three-way join on every query — disk is
  the cheap resource in this trade, moving parts are not.

The work:

- Migration 0002, crates/havoc-core/migrations/0002_fts.sql, appended to
  MIGRATIONS in crates/havoc-core/src/storage/migrations.rs. Contents:
  CREATE VIRTUAL TABLE message_fts USING fts5(text, buffer_id UNINDEXED,
  seq UNINDEXED) — text is the only indexed column; identity rides along
  unindexed and everything else hydrates from message by (buffer_id, seq),
  the source of truth. An AFTER INSERT trigger on message keeps it in sync
  (WHEN new.text IS NOT NULL), and a backfill INSERT..SELECT indexes every
  existing row in the same migration transaction. Sync-by-trigger rather
  than by Rust, deliberately: the FTS write becomes structurally part of the
  statement that inserts the row, so no future insert path can forget it,
  ON CONFLICT DO NOTHING dedup hits never fire it, and rollback carries it
  for free. No UPDATE/DELETE triggers: message is append-only and retention
  owes its own migration (stage 6).
- FTS5 availability, confirmed rather than assumed: rusqlite's bundled build
  compiles -DSQLITE_ENABLE_FTS5 unconditionally (libsqlite3-sys build.rs,
  verified in the vendored source) — no cargo feature, no manifest change,
  no allowlist edit. Were that ever wrong, migration 0002 fails loudly at
  first startup, which is the check.
- The rollback story: the trigger fires inside write_batch's transaction, so
  a failed batch rolls the FTS rows back with it and the existing cache
  invalidation stands unchanged. Test the mid-batch failure this demands.
- Search rides the job queue — the one-connection question, decided:
  Job::Search executes on the storage thread's single connection, after the
  existing flush-before-non-ingest barrier, so read-your-writes holds. The
  second read-only WAL connection is rejected: it silently forfeits exactly
  that to fix a latency problem nobody has measured; revisit on dogfood
  evidence.
- The lane, the ClientId gap, and the error story, one shape: Job::Search
  { spec, client, request, reply } is fire-and-forget like Job::Ingest, with
  (client, request) echoed through untouched. handle_request becomes
  handle_request(state, client, request) -> Option<Response>; the Search arm
  parses, enqueues, and returns None — exactly-one-Response promises one
  response, not an instant one. On outcome, core sends Directed::Response
  first — Ack, or Error carrying the SQLite message for a malformed MATCH —
  then Directed::Event(SearchResults) on Ack only, both via
  Bus::direct(client, ..). The broadcast debug_assert stays as the
  structural guard.
- The grammar, core-side over the frozen wire: a new module
  crates/havoc-core/src/search.rs parses the plain string into SearchSpec
  { match_query, nick, buffer, after, before } with a quote-aware scanner:
  from:/in: exact filters (ASCII NOCASE — matches ergo's casemapping;
  rfc1459 folding deferred until a real network bites), after:/before:
  accepting YYYY[-MM[-DD]] expanded via ingest's days_from_civil. Everything
  else passes to MATCH verbatim — phrase search for free. A filter-shaped
  token inside quotes is text. Parse errors (duplicate filter, bad date,
  no text terms) are immediate Error responses — a filters-only query is a
  backlog scan wearing search's clothes, and scans are prompt 9's.
- The query: message_fts MATCH joined to message on (buffer_id, seq);
  buffer filter by name against the buffer table (history from before this
  process searches too); server_time range as a display-time filter (§6.1
  permits exactly this); ORDER BY rank (stock bm25, used, never tuned);
  LIMIT SEARCH_MAX_HITS = 100, the §6.3 posture — the wire has no has-more
  berth, so a full window means "refine the query" and the CLI's hits count
  keeps truncation visible. kind_from_code inverse added, loud on unknown
  codes.
- CLI: `search <rest of line>` — the raw remainder, not whitespace-split
  parts, or quoting dies before core sees it — plus `wait search [count]
  [secs]`, and one greppable line per hit.
- scripts/live-run.sh, a search segment after the reconnect proof: the
  deployment line found; from:bob 1 hit / from:carol 0; the phrase
  "flood line 250" exactly once; before:2020 → 0; read-your-writes — the
  fresh line's echo arrives (harness-level trace wait: the echo is network,
  not storage), then search with no further delay finds it through the
  flush barrier; a malformed MATCH errors and the session survives; search
  wall time recorded, not asserted.
- Tests: migration 0002 on fresh and on a version-1 file whose backfill
  makes old rows searchable; trigger sync + dedup-indexes-nothing +
  NULL-text absent; the mid-batch rollback test; the scanner units; the
  100-hit cap; and a two-session dispatch-level test — A's search yields
  Response-then-SearchResults in order on A's directed lane while B's lanes
  stay silent. schema_matches_north_star extended to the new objects.

Acceptance: run scripts/live-run.sh and, after the flood and the restart,
watch A answer `search from:bob deployment` with the hit line and
`search in:#flood "flood line 250"` with exactly one hit; watch a line sent
moments earlier come back from search through the flush barrier; a
malformed query errors and the session keeps running; the script exits 0.
After it, sqlite3 proves the index is real on disk. cargo test --workspace
green including the v1-upgrade and rollback tests; make check green.

Do not: jump-to-context (prompt 9 — AroundSearchHit belongs to the backlog
API); saved searches / virtual buffers (stage 6 menu); ranking beyond ORDER
BY rank — no bm25 weights, no snippet()/highlight() (presentation, stage
2+); filters beyond from:/in:/after:/before:; a second read-only connection
(dogfood evidence first); FTS UPDATE/DELETE sync (retention's migration,
stage 6); wire changes of any kind (PROTOCOL_VERSION stays 1); new
dependencies; and no async facade over StorageClient — the search lane
copies the ingest lane's fire-and-forget shape.
```

**Examined for a split (index vs query language) and left whole**, because the filter
grammar and the index shape must be designed together or the filters end up
unindexable. Revisit if the filter grammar grows past the three filters named here —
anything more is stage 2+ polish.

**Status:** complete. Shipped per the JIT detail, and the flood harness earned its
keep twice over: it exposed a real product deadlock — the core↔actor channel cycle
(core blocked on a full command lane while the actor blocked on a full report
lane), which stalled the flood at ~430/500 and got the client sendq-killed by ergo
— fixed by making the report lane unbounded (the storage queue's own never-drop-
history argument) with reads biased first in the actor; and it surfaced the FTS5
hyphen trap (`xyzzy-quicksilver` parses as column-filter syntax and errors
cleanly — the Error path working as designed, recorded for stage 2's /search UX).
The read-your-writes proof waits for the echo (network), then searches with no
further delay (storage). Four consecutive 24/24 live runs.

---

## Prompt 9a — Windowed backlog

**Item:** Windowed backlog and read markers.
**Branch:** `prompt-09a-backlog`

`FetchBacklog` for real: all four anchors including `AroundSearchHit`, `limit`
capped server-side regardless of what the client asks (§6.3); a debug CLI
`backlog` verb. There is still no "give me the buffer" request — the fence of
§4.7 holds here or nowhere. This half also owns the buffer-announcement decision
its notes demand: the first session ever driven against a populated store cannot
resolve buffers no event introduced.

**Split from the original prompt 9** (with read markers) to respect the 800-line
tripwire: the backlog window and the announcement mechanism are one read-path
seam; markers are a write-path seam with its own verb/drain hardening. Revisit if
the announcement decision turns out to need marker state — nothing suggests it.

```
Windowed reads, for real: after this prompt a person opens a fresh session over
a data dir an earlier process wrote, watches the buffers it never saw get
announced to it, and pages history in windows — latest, before, after, and
centred on a search hit — with the limit capped in the engine no matter what is
asked for (§4.7, §6.3). Nothing gained the power to say "give me the buffer".

Three places the notes, the item, and the frozen wire pull apart, surfaced
rather than resolved silently:

- The buffer-announcement note reads like a §4.7 violation and is not one. The
  fence forbids an unbounded *message* fetch; enumerating buffers is the §4.5
  attach contract in as many words ("a fresh client asks what buffers exist and
  where its read markers are"), the set is bounded by human action rather than
  by traffic, and it carries no message text. It is settled here by making it
  not a request at all: the core *announces* at attach and no client can ask.
  RequestBody grows nothing, so the fence is not approached, let alone crossed.
- The read-side backpressure note asks for a decision the prompt-5 topology
  cannot give as shipped: core awaits `bus.direct`, so one wedged reader stalls
  the select loop, and behind it the storage thread's `blocking_send`. This
  prompt adds a second bus primitive rather than rewriting the first (below),
  and leaves search's Response-then-Event pair on the awaiting `direct` —
  converting an ordered pair needs "what does half a delivered pair mean"
  answered, which is prompt 9b's verb/drain seam, not this one's. That
  conversion becomes 9b's carry-forward if it survives review here.
- The item bundles read markers, which are 9b's. The split is read path vs
  write path, so the replay here *reads* `buffer.last_read_seq` into
  `BufferInfo.last_read_seq` — the column has existed since migration 0001 and
  nothing writes it yet, so every value is NULL today. Filling a field the wire
  already froze costs nothing and saves 9b a second pass through the read path.
  Setting it, and `ReadMarkerChanged`, stay in 9b.

The work:

- No wire change: PROTOCOL_VERSION stays 1 and the berths prompt 2 froze are
  the ones used. Windows come back in ResponseBody::Backlog { messages } — the
  response itself, not an event — and announcements reuse Event::BufferCreated.
  Both rejected alternatives were new enum variants (Event::BufferList,
  Event::BacklogWindow): unknown *fields* are tolerated on this wire, unknown
  *variants* are not, so either would be a real v1 break bought for a payload
  shape that already exists. Reusing BufferCreated does restate its meaning —
  "this buffer exists and you did not know it" rather than "it was just
  created"; renaming the variant is itself a wire break, so the meaning is
  documented on the variant instead. When adding variants becomes negotiable —
  stage 4's handshake, whose constants live in havoc_ipc::caps — revisit
  batching the replay into one message.
- The window is always ascending by seq, for every anchor (§4.6: seq is the
  only order; one order for all four anchors means the client's scroll math has
  no cases). Against `message`'s (buffer_id, seq) WITHOUT ROWID primary key
  each anchor is one range scan on the table that *is* the index — which is why
  the windowed API costs nothing, the §6.3 claim made concrete:
  - Latest — `WHERE buffer_id = ?1 ORDER BY seq DESC LIMIT ?2`, reversed in
    Rust.
  - Before(s) — `... AND seq < ?s ORDER BY seq DESC LIMIT ?n`, reversed.
    Exclusive: the client is holding s and asked for what precedes it.
  - After(s) — `... AND seq > ?s ORDER BY seq ASC LIMIT ?n`. Exclusive
    likewise.
  - AroundSearchHit(s) — the row at s, plus floor((n-1)/2) rows before it and
    the remainder after, assembled by calling the same descending and ascending
    helpers and concatenating. A short side does not grow the other: the hit
    stays centred even at a buffer edge, so the client's position math is
    predictable and one cheap After window fills the rest. A seq that no longer
    exists returns the neighbours around the gap rather than an error — history
    is append-only today, but retention (stage 6) will make gaps real.
- Empty window vs unknown buffer, distinguished: no rows in range →
  Backlog { messages: [] }, the normal end-of-scrollback signal a client pages
  until; a buffer id absent from the `buffer` table → Error("unknown buffer N"),
  because a client bug that looks like end-of-scrollback forever is
  undebuggable. One SELECT on `buffer` inside the same job decides which.
- The cap: BACKLOG_MAX_LIMIT = 200, in crates/havoc-core/src/storage/query.rs
  beside SEARCH_MAX_HITS, applied as `limit.min(cap)` at the single site that
  binds the SQL LIMIT, so every later caller inherits it. 200 is two screens at
  any sane terminal height and tens of KB on the wire. `limit == 0` is an
  Error, not an empty window — asking for nothing is a client bug and answering
  it politely hides it. The constant stays core-side rather than in
  havoc_ipc::caps: a client that compiles the cap in is a client that breaks
  when it moves; discovery is the stage-4 handshake's job, and that file says so
  already.
- Two new read jobs on the one connection, behind the existing
  flush-before-non-ingest barrier, so a window sees the line that landed a
  millisecond ago exactly as search does: Job::Backlog { buffer, anchor, limit,
  client, request, reply } and Job::ListBuffers { client, reply }, both
  fire-and-forget like Job::Search. One reply lane, not two: ReadOutcome::{
  Backlog, Buffers } over a new bounded (64) `reads_tx`, so core grows one
  select arm. Under the existing trace flag the thread prints one
  `storage backlog buffer=N rows=K` line per window — measurement is grep, and
  the two-eprintln stance stands.
- Storage rows stay core-private (prompt 3's fence): ListBuffers returns
  Vec<BufferRow> { id, network_name, name, kind, last_read_seq }. The row knows
  a network_id, never the caller's NetworkId, and minting a wire id inside
  storage is precisely the id-space confusion NetworkRow was created to kill.
- Row hydration becomes one private fn in query.rs used by both run_search and
  run_backlog. Two uses, not three, and deliberately: the duplicate is the
  block where kind_from_code's loud-on-unknown behaviour would fork silently.
- The FetchBacklog arm in crates/havoc-core/src/core.rs mirrors the Search arm
  including the part search only got right after review — a failed enqueue
  becomes an immediate ResponseBody::Error("storage unavailable: {e}"), never a
  swallowed promise. Success returns None; the one promised Response arrives
  with the window.
- Attach grows the announcement: `bus.register`, then enqueue Job::ListBuffers.
  On the outcome, core maps each BufferRow's network_name back to a caller
  NetworkId by matching `state.settings` values' `name` — the same string
  ensure_network keyed on — and skips any buffer whose network is not
  configured: a BufferInfo carrying a NetworkId the client cannot name is worse
  than an absence, and that history reappears the moment the network returns to
  config. Core also seeds `state.buffers` from the resolved list, which is what
  makes SendText to a buffer from a previous run answer "buffer's network is
  not connected" instead of "unknown buffer".
- Delivery of the replay is a short-lived spawned task holding a clone of the
  session's lane (new Bus::lane(id) -> Option<Sender<Directed>>), awaiting once
  per announcement. Reasoning: a just-attached client may not have started
  reading yet, the list can exceed the lane's 64 slots, and neither the core
  loop nor the storage thread may wait on it. Ordering against broadcast
  traffic is deliberately not guaranteed; the contract is that announcements
  are idempotent — a duplicate is legal, a missing one is not — which is also
  what makes the race between the ListBuffers snapshot and a concurrent
  BufferCreated a non-event. The CLI's HashMap insert already satisfies it.
- Read-side backpressure, decided: Bus::try_direct(id, message) -> bool,
  non-blocking. Closed removes the lane silently (an ordinary detach,
  unchanged); Full removes it after one loud eprintln naming the ClientId.
  Backlog responses go out this way. The asymmetry the note asked for, stated
  plainly: writes are buffered without bound because a lost line is
  unrecoverable, reads are dropped because a read is by definition re-askable —
  the engine must never hold history behind a reader that has ignored 64
  answers and asked for a 65th. handle_outcome's broadcast path is untouched
  (it never awaited a client), and run()'s immediate-response `direct` stays
  as-is (see the surfaced conflict).
- CLI, crates/supernaut/src/session.rs: `backlog <buffer> <anchor> [limit]`
  with anchor `latest | before:<seq> | after:<seq> | around:<seq> |
  around-hit`, resolving the name through the same projection `send` uses.
  `around-hit` uses the newest SearchResults hit seen for that buffer
  (SessionState.last_hits: HashMap<BufferId, Seq>), so the jump-to-context flow
  AroundSearchHit exists for is the flow the harness actually exercises rather
  than a seq the script pasted in.
- `wait backlog [count] [secs]` counts **responses**, not events — the note's
  fix: SessionState.backlog_pending: HashSet<RequestId> filled at request time,
  and both Backlog and Error responses for those ids increment the counter, so
  a failed window ends the wait with a printed error instead of a timeout with
  nothing to read. `wait search`'s event-counting blindness is deliberately
  left alone and named as prompt 9b's (it owns verb hardening); if
  response-counting proves the better primitive, that is where it converts.
- Printing: handle_incoming's placeholder `ok N (backlog)` is replaced, not
  joined, by one `backlog request=N buffer=B count=K` line plus one
  `line buffer= seq= nick= text=` per message in order. The count on the header
  is how a capped window stays visible with no has-more berth on the wire.
  Anchor parsing and this printing live in a new
  crates/supernaut/src/session_backlog.rs for the same reason session_print.rs
  exists: session.rs is at 349 lines against the 400-line ratchet.
- scripts/live-run.sh, two segments. First, before A quits, over the corpus the
  flood and the restart already built: `backlog #flood after:0 5` → five
  ascending lines starting at seq=1; `backlog #flood latest 9999` → count=200,
  the cap observed from outside the engine; and after the existing
  `search in:#flood "flood line 250"`, `backlog #flood around-hit 7` → the hit
  centred, with `flood line 247` and `flood line 253` both present. Backlog
  wall time recorded, not asserted, as search's is.
- Second, the announcement proof, which requires a process that never touches
  the network: after A has quit and closed its database, a session D over the
  same `--data-dir "$WORK/data-a"` issuing no `connect` at all — `wait buffer
  #supernaut 10` succeeds purely from the replay, `backlog #supernaut latest 5`
  returns `text=the deployment failed` that an earlier process wrote, and D's
  output contains no connection-state line. D must pass the same `--host` as A:
  the network name is derived (`format!("debug-{}", host)`) and the replay
  resolves buffers by that name — a coupling prompt 10a's config file deletes.
  Two live processes over one file is deliberately not attempted: cached
  per-buffer seq counters in two writers would race the write path, and
  demonstrating the read path is not the place to discover that (stage 4's
  daemon makes one writer structural).
- Tests. Storage: the four anchors against a seeded buffer — ascending always,
  Before/After exclusive, the around-window centred and still centred at both
  edges, a vanished hit seq returning its neighbours; u32::MAX capped to 200;
  limit 0 an error; unknown buffer an error where an out-of-range window is a
  success. Core: a new crates/havoc-core/tests/core_backlog.rs — attach over a
  populated store with a settings map whose `name` matches the seeded network,
  assert the BufferCreated replay lands on the attaching session's lane and on
  no other, then a FetchBacklog whose Response carries the window; plus the
  unconfigured-network skip. Bus: try_direct returns false and drops the lane
  on Full, silently on Closed. Implementer's warning: existing assertions that
  expect a specific *first* message on a directed lane
  (crates/havoc-core/tests/core_search.rs) must skip replay events rather than
  assume none arrive.
- The diff stays inside the 800-line tripwire this split bought. If the test
  surface pushes past it, the storage-level edge-case anchors are what trails
  into 9b — never the live run.

Acceptance: run scripts/live-run.sh. Before A quits, watch `backlog #flood
after:0 5` print five ascending lines starting at seq=1, watch `backlog #flood
latest 9999` come back as count=200 — the engine refusing the number it was
handed — and watch `backlog #flood around-hit 7` print the "flood line 250" hit
with 247 and 253 around it: jump-to-context, headless. Then watch a process
that never dials anything: session D over A's data dir resolves #supernaut from
the attach announcement alone and reads "the deployment failed" back out of
history a different process wrote, with no connection-state line anywhere in
its output. The script exits 0. cargo test --workspace green including the
four-anchor, cap, and attach-replay-leak tests; make check green with the
ratchets not worsened (measure session.rs).

Do not: read markers — set, event, or verb (prompt 9b: the write path, with its
own verb and drain hardening); any RequestBody addition, a buffer-list request
above all (§4.7's fence; §4.5 makes it an announcement instead); any new Event
or ResponseBody variant (unknown variants are not serde-tolerant, so it is a
real v1 break — stage 4's handshake is where additions become negotiable);
CHATHISTORY or any upstream backfill when a window runs off the start of local
history (stage 5 — the same FetchBacklog shape maps onto it, which is the
point, but a real bouncer is the only honest test); a has-more or total-count
field (a wire change to remove an ambiguity the CLI's count line already shows;
stage 2's scroll UX decides whether it earns one); snippet/highlight or kind
filtering inside a window (presentation, stage 2 — the window returns every
row, Joins included); a second read-only SQLite connection (still no
measurement, and the barrier is the read-your-writes guarantee); converting
search's Response+Event pair to try_direct or fixing `wait search` (both prompt
9b); new dependencies (nothing here needs one); and no async facade over
StorageClient — the read jobs copy the fire-and-forget lane ingest and search
already use.
```

**Status:** complete. Shipped per the JIT detail with one recorded deviation: the
implementer's warning about `core_search.rs` did not bite — that test spawns core with
an empty settings map, so nothing resolves and nothing is announced, and it passes
untouched. The announcement proof is the payoff: a process that never dials anything
reads another process's history back out of a window.

---

## Prompt 9b — Read markers and verb hardening

**Item:** Windowed backlog and read markers.
**Branch:** `prompt-09b-read-markers`

Read markers (`last_read_seq`) set and read per buffer, emitted as events, with a
`mark-read` verb — plus the session-verb debts the notes below carry: a real
drain on `quit`, kind-aware message counting, and the harness's contiguity check
kept intact. No reconciliation between clients (stage 6, blocked on the
Still-open) and no `draft/read-marker` upstream.

**Split from the original prompt 9**: markers alone are small, which is exactly
the room the accumulated verb/drain hardening needs.

```
Read markers, for real, and the delivery policy that stopped being optional:
after this prompt a person marks a buffer read in one process, watches the
change come back as broadcast state, quits, and a second process that never
dials anything is *told* where the marker is at attach — and no client, healthy
or wedged, can stall or be silently starved by the engine on the way there.

Four places the notes, the item, and the frozen wire pull apart, surfaced
rather than resolved silently:

- The PLAN item is the smaller half of this prompt. "`last_read_seq` set/read
  per buffer" is one job, one arm, one verb; the notes carry a delivery-policy
  rewrite that touches every path prompts 5-9a shipped. That is what the
  9a/9b split bought and it is spent here deliberately — but it means the
  line budget belongs to the lane rewrite, not to markers.
- This prompt supersedes a decision one prompt old: "read answers are
  dropped, not awaited: `Bus::try_direct`" (BUILD-LOG, 2026-08-10). Its own
  Revisit-if names this prompt, so this is sanctioned rather than a
  violation, and its asymmetry argument survives intact — writes are never
  dropped, reads are re-askable. What changes is the mechanism: dropping a
  read at 64 slots was never the real bound, because a second writer (the
  replay task) could park on the same lane and make the drop
  nondeterministic. One writer, one rule, below.
- The wire cannot express a per-client marker at v1, so shipping markers ships
  the machine-wide one. `Event::ReadMarkerChanged { buffer, seq }` has no
  client field and `buffer.last_read_seq` is one nullable column
  (migration 0001) — the representable set was fixed by prompt 3, not
  chosen here. So this prompt broadcasts the marker and says so, and the
  Still-open on reconciliation stays open: it is answered by a schema and a
  wire that can hold two markers, neither of which exists yet.
- Kind-aware counting changes numbers in live-run.sh, and session D's window
  is fragile to exactly that. Sequencing, not conflict: D's read is
  re-anchored first, in the same commit, before any verb semantics move.

The work:

- No wire change. PROTOCOL_VERSION stays 1; `RequestBody::SetReadMarker`,
  `Event::ReadMarkerChanged`, and `BufferInfo.last_read_seq` are the berths
  prompt 2 froze and prompt 9a already reads. Doc comments do change (as 9a's
  addendum did for `BufferCreated`): `ReadMarkerChanged` states that it is
  broadcast core-owned state, one marker per buffer for the whole machine,
  and that per-client markers are a stage-6 schema change, not a field.
- `SetReadMarker` is a write with a deferred response, mirroring the
  `Search`/`FetchBacklog` arms in `crates/havoc-core/src/core.rs`: the arm
  enqueues and `return None`; a failed enqueue becomes an immediate
  `Error("storage unavailable: {e}")`, never a swallowed promise. The stale
  `error("SetReadMarker arrives in prompt 9")` string dies with the arm. On
  the outcome: `ResponseBody::Ack` on the requester's directed lane and
  `Event::ReadMarkerChanged` on the broadcast lane; on `Err`, the `Error`
  response and no event — search's rule, and the reason is the same (a failed
  write is not state).
- Broadcast, not directed, and it is not a leak: §4.5 puts read markers in the
  Core column, the value is the same one `announce` already hands every
  attaching client in `BufferInfo.last_read_seq`, and a marker moved by one
  client is a marker moved for the machine as long as the column is one
  column. `Bus::broadcast`'s `debug_assert` list stays `SearchResults`-only —
  `ReadMarkerChanged` carries no `RequestId`, which is the structural tell.
  The Ack and the event travel different lanes and are therefore unordered
  relative to each other; that is documented, and it is why the CLI's wait
  counts responses (below) rather than events.
- Marker semantics, decided: a marker may move backward. The client is the
  authority on where a person has read to, and scrolling back to an unread
  point is a real product action; a monotonic clamp in the engine refuses a
  legitimate request with no way to report it, and "highest wins" is precisely
  the last-write-wins reconciliation rule the Still-open owns — pre-deciding it
  here inside an `UPDATE` is how that question gets answered by accident.
  Last write wins, no clamp.
- `seq < 1` is an `Error("read marker seq must be at least 1")` — asking to
  mark nothing read is a client bug and answering it politely hides it, the
  same call `limit == 0` got in 9a. The seq is not checked for existence
  and never clamped to `MAX(seq)`: a marker is a position, not a row
  reference, and retention (stage 6) will make gaps real — the same reasoning
  `AroundSearchHit` over a vanished seq already ships with.
- Unknown buffer is an `Error("unknown buffer N")`, decided by the `UPDATE`'s
  own `changes()` rather than a preceding `EXISTS` — one statement, precise,
  and cheaper than the read path's check.
- `Job::SetReadMarker { buffer, seq, client, request, reply }`, fire-and-forget
  like `Job::Backlog`, answering on the existing `reads_tx` as a third
  `ReadOutcome::MarkerSet { client, request, buffer, seq, result }`. One reply
  lane, not a fourth `mpsc` in core's select loop for one row's work; the
  enum's doc widens from "a finished read" to "a finished job behind the flush
  barrier". It is a write and it deliberately does not batch with ingests:
  batching exists to amortize fsync over history at IRC's line rate, a marker
  is human-paced and one row, and delaying it 100ms would make the Ack a lie.
  Falling into `exec.rs`'s existing `Some(other)` arm gives the barrier for
  free — pending ingests flush before the `UPDATE`, so the persisted marker
  is never ahead of persisted history, which is what makes it safe to read
  back after a crash. No new trace `eprintln`: a one-row write is not a
  measurement, and the event and the Ack are already observable.
- SQL in `set_read_marker` beside `ensure_buffer` in
  `crates/havoc-core/src/storage/exec.rs` — the other `buffer`-table write,
  and the single writer's writes belong together. exec.rs is 366/400 against
  the longest-file ratchet: if it crosses, the split to take is
  `storage/rows.rs` for the three non-batch row writes (`ensure_network`,
  `ensure_buffer`, `set_read_marker`), not a cut.
- The fate of both awaits, decided together: the directed lane's capacity
  rises 64 -> `DIRECTED_LANE_CAPACITY = 4096`, `Bus::try_direct` and
  `Bus::lane` are deleted, and `Bus::direct` becomes the one synchronous
  primitive — `pub fn direct(&mut self, id, message) -> bool`, implemented
  on `try_send`: never awaits, never blocks, never silently drops a message
  while the lane lives. `Closed` removes the lane silently (an ordinary
  detach). `Full` at 4096 removes it after one loud line naming the
  `ClientId`. Why this and not the alternatives:
  - The deadlock dies structurally, not by care: after this change no path
    in the core loop awaits a client, so no client can stall the select
    loop, and the storage thread's `blocking_send` on the bounded reply lanes
    can never be held behind a reader. `search_tx`/`reads_tx` stay bounded at
    64 — that backpressure is between storage and core, and both ends now
    always drain.
  - "What does half a delivered pair mean" is answered by making it
    unrepresentable: `handle_search_outcome` becomes non-async and pushes
    Response-then-`SearchResults` through two synchronous calls with no
    await point between them, so the ordered pair either both lands or the
    lane is already gone and the session is over. Search keeps its pair.
  - The replay stops being a task: `announce` in
    `crates/havoc-core/src/core/reads.rs` delivers inline from the core loop,
    which deletes the second writer — and with it the zombie-vs-loud-kill
    nondeterminism, because the map entry is once again the only aliveness
    token and only the core loop touches it.
  - Rejected: an unbounded lane with a depth watermark (tokio's
    `UnboundedSender` has no `len()` — depth lives on the receiver — so the
    watermark needs receiver-side tracking machinery, bought for zero
    behavioral difference against `try_send` at the same threshold); a
    per-session writer task with an overflow deque (the right answer when
    bytes are the bound, and it is stage 4's, where clients become plural and
    untrusted); dropping single reads at the threshold instead of the session
    (a client that has ignored 4096 answers is broken, and dropping message
    4097 silently leaves it broken and undiagnosed).
  - Honest about the bound: 4096 counts messages, not bytes, and one message
    can be a 200-row window. It is chosen to sit far above any legitimate
    attach replay (one message per buffer; a human does not have 4096 buffers)
    and far below hurting a laptop for `BufferInfo`-sized traffic. Byte-based
    accounting goes to PLAN stage 4 with the socket server.
  - A consequence worth keeping in the doc comment: because removal only
    drops the `Sender`, a killed session still drains what is already queued
    and then sees `Closed` — the client observes everything up to the
    message that found the lane full, which the loud line names.
  - Mechanical fallout: `Session.directed` keeps its exact type
    (`mpsc::Receiver<Directed>`), so `wiring.rs` and the existing dispatch
    tests are untouched; `EVENT_CHANNEL_CAPACITY` stays what it is (broadcast
    lane, different concern); `run()`'s immediate-response arm and
    `handle_read_outcome` lose their `.await`/`try_direct` split and use the
    one primitive.
- CLI request bookkeeping becomes one structure with one insert site and one
  remove site: `SessionState.outstanding: HashMap<RequestId, Awaited>`
  replaces `search_count`'s event counting and `backlog_pending`/two ad-hoc
  counters. `request()` classifies the body itself
  (`Awaited::from(&RequestBody)` -> `Search | Backlog | Marker | Other`), so a
  new verb cannot forget to register; `handle_incoming` removes the id and
  bumps the matching counter before the body match, so an `Error` still
  ends a wait with something printed. `print_event`'s `SearchResults` arm
  stops counting and only records `last_hits`. Consequence to check while
  editing the script: `wait search N` now returns on the response, so hit
  lines may print after `waited search N` — every `hit` assertion in
  live-run.sh already sits in the post-quit phase, and the quit drain below is
  what makes that deterministic rather than lucky. Re-verify against the
  script as it exists now.
- `quit` and stdin EOF drain. Both paths call one `finish()`: wait until
  `outstanding` is empty or a deadline (`quit [secs]`, default 10) expires,
  printing everything as usual; then a 50 ms quiet-period sweep so an event
  queued behind its own response (a `SearchResults`, a `ReadMarkerChanged`)
  is printed rather than discarded. A timeout is an `Err(...)` naming the
  count, so the process exits non-zero and the harness notices — an engine
  that did not answer is a finding, not a shrug. The 50 ms is a quiet period
  on a queue we own, not a race hidden by a sleep, and zero would be racy
  against the adapter task's one hop.
- What the drain does not buy, stated where the composer will read it:
  `Ack` means dispatch accepted it, and `ActorCommand`s issued while
  disconnected are still dropped during the backoff sleep in
  `crates/havoc-core/src/connection/actor.rs`. Decided: documented
  at-most-once, made loud — the drop site's silent handling becomes one
  `eprintln` naming the network and the command, and `ActorCommand`'s doc plus
  `RequestBody::SendText`/`Join`'s docs state at-most-once outright. An
  actor-side outcome was rejected because it has nowhere to go: the Ack has
  already been sent, correlation is gone, and saying "not sent" needs a wire
  berth — a per-request delivery outcome, which is a variant addition and
  therefore a real v1 break (the same refusal 9a made twice), owed by stage
  4's handshake. The deeper reason it is the right answer anyway: the echo
  is the real confirmation. `echo-message` is in the requested caps
  (`connection/caps.rs`), so our own PRIVMSG comes back as a `MessageAdded`
  from the authority that matters; on a server without it, "sent" is
  unconfirmable by anyone, which is what at-most-once means. Stage 2 gets the
  note that the composer clears pending-send state on the echo.
- Kind-aware counting. `msg_counts` becomes per-buffer `{ chat, total }`.
  `wait message <name> [count] [secs]` counts privmsg and notice only —
  what a person means by a message — and the new `wait rows <name> [count]
  [secs]` counts every `MessageAdded` kind, which is what the store counts
  and therefore the verb that belongs anywhere the script is reasoning about
  seq. Script edits: carol's own drain becomes `wait message #flood 500` (her
  join is no longer a message); A's `wait message #flood 500` now means 500
  privmsgs rather than 499 plus a join; the post-restart re-join sync becomes
  `wait rows #supernaut 4`, whose comment can finally say what it means — the
  thing being awaited is a Join row. The `COUNT(*) == MAX(seq)` contiguity
  assert in the post-mortem block is not touched: it is the
  dedup-consumes-no-seq invariant, and `wait rows` exists so a row-level claim
  stays expressible in the CLI beside it.
- `mark-read <buffer> <seq>` in `crates/supernaut/src/session.rs`'s dispatch,
  resolving the name through the same projection `send` and `backlog` use, and
  `wait marker [count] [secs]` counting responses like `wait backlog`. The
  `event read-marker buffer= seq=` printer already exists (prompt 5) and is
  reused unchanged. `print_event`'s `BufferCreated` line grows
  `last_read=<seq|->`: without it the announcement's marker is invisible to
  the harness, which is the whole proof in session D.
- File sizes: session.rs is 375/400 and this adds a verb, the drain, and two
  wait targets. The `wait`/drain machinery moves to a new
  `crates/supernaut/src/session_wait.rs` — the verbs' whole waiting-and-
  draining story in one file, which is also where the response-counting
  unification lives — for the same reason session_print.rs and
  session_backlog.rs exist. Measure session.rs afterwards; the ratchet ceiling
  stays 400 (9a declined to tighten it precisely so this prompt's file had
  room).
- live-run.sh, in this order. First, the anchoring, before anything else
  moves: session D reads `backlog #supernaut after:0 50` instead of
  `latest 5`, so no later traffic can push "the deployment failed" out of the
  window and make an announcement failure out of a window failure. Then
  the marker segment, after the backlog windows and before A quits:
  `mark-read #supernaut 3` + `wait marker 1 10`, with the arithmetic written
  above it (A join 1, B join 2, B's deployment line 3, A's re-join 4,
  xyzzysearchtoken 5 — and the assertion is that the value round-trips, so
  it survives a traffic change even if seq 3 stops being that line).
  Asserted: `event read-marker buffer=[0-9]* seq=3` in a.out (broadcast state
  observed by the process that set it), then in d.out `event buffer-created
  .* name=#supernaut last_read=3` — a marker one process set, read back by a
  process that never dialled anything, out of the attach announcement.
  Then the drain's proof, which is a count, not a feature: three `^ok `
  lines in b.out (connect, join, and the send whose Ack raced the runtime
  drop today) and 502 in c.out (connect, join, 500 sends) — the exact
  regression the note describes, where c.out stopped near `ok 192`. Carol's
  echo wait stays: it proves the server saw the lines, which no Ack does.
  And in the post-mortem sqlite3 block, beside the contiguity assert:
  `SELECT last_read_seq FROM buffer WHERE name='#supernaut'` is 3 — the
  marker on disk, from outside the process, as the FTS assert already does.
- Tests. Storage: the marker writes the column and `run_list_buffers` returns
  it; unknown buffer is an error; `seq = 0` is an error; a backward move wins
  (set 5, set 3, read 3). Core, new
  `crates/havoc-core/tests/core_markers.rs`: `SetReadMarker` answers `Ack` to
  the requester and `ReadMarkerChanged` on a second session's broadcast
  receiver (the lane is the claim under test); an unknown buffer answers
  `Error` and emits no event. Bus: the capacity test replaces the Full-drop
  test — fill the lane to 4096 with nobody draining, and the next `direct`
  removes the lane once, loudly, and later calls return false; the
  closed-lane-silent test stays. Core, in
  `crates/havoc-core/tests/core_backlog.rs` (same subject): the 65-buffer
  attach — seed 65 buffers, attach a bare `Session` with no pump,
  `FetchBacklog`, and assert all 65 announcements then the correlated
  Response arrive with the lane alive. That test is the one that fails against
  today's code, and it is not tradeable. While in that file: its
  `last_read_seq.is_none()` assertion and the comment "nothing writes
  last_read_seq until prompt 9b" are now stale — 9a's own review lesson was
  that the comment is the defect.
- Three decision entries in BUILD-LOG.md, because each has a rejected
  alternative argued above: the lane policy (bounded 4096 + try_send,
  superseding the `try_direct` entry, and saying so — include the coordinator
  amendment about UnboundedSender lacking len()); read-marker write semantics
  (broadcast, backward allowed, no existence check, its own job behind the
  barrier); and what "sent" means (at-most-once plus the quit drain, with the
  echo as the real confirmation). PLAN.md gains the deferrals in the same
  turn: stage 2 (composer renders pending-send until the echo; the
  response-before-event ordering), stage 4 (a per-request delivery outcome
  berth; byte-based lane accounting), stage 6 (`ReadMarkerChanged` is
  broadcast today and becomes per-client when the schema can hold two
  markers). All seven carry-forward notes are deleted from STAGE-1-PROMPTS.md
  and recorded under `**Carry-forward consumed:**` as one act.
- The diff stays inside the 800-line tripwire, and unlike 9a that is a
  budget, not a hope: the lane rewrite is the expensive part. If it pushes
  past, what trails into stage 2's first prompt, in this order, is (1) the
  storage-level marker edge cases beyond unknown-buffer, and (2) `wait rows`
  (leaving `wait message` all-kinds with its semantics documented). Never the
  live run, never the 65-buffer test, never the capacity test.

Acceptance: run scripts/live-run.sh. Watch A mark #supernaut read — `ok N`
for the write, then `event read-marker buffer=… seq=3` arriving as broadcast
state in the same process that asked for it. Then watch session D, which
issues no `connect` at all, be told `event buffer-created … name=#supernaut
last_read=3`: a read position one process set, handed to another at attach out
of a database on disk, with `sqlite3` agreeing from outside both. In the same
run, count the responses nobody used to get: three `ok` lines in b.out and 502
in c.out, because quit now waits for its outstanding requests instead of
dropping the runtime on top of them. The script exits 0 with its assertions up
from 35. cargo test --workspace green, including the 65-buffer attach that
deadlocks against today's code and the capacity test that replaces the
Full-drop one; make check green with the ratchets not worsened (measure
session.rs after the session_wait.rs split, and exec.rs).

Do not: per-client markers or any reconciliation between attached clients
(stage 6 item 1 — and the Still-open stays open: shipping the one marker the
single nullable column can represent is not answering which of two wins);
`draft/read-marker` propagation upstream (stage 5's upstream work, and whether
at all is the same Still-open); any wire change — no new `RequestBody`,
`ResponseBody`, or `Event` variant and PROTOCOL_VERSION stays 1 (unknown
variants are not serde-tolerant, so a "not sent" signal or a batched replay is
a real v1 break; stage 4's handshake is where additions become negotiable);
clearing or unsetting a marker (the wire has no `Option<Seq>` berth and
"mark unread" is a stage-2 product question); checking that the marker's seq
exists, or clamping it to `MAX(seq)` (retention will make gaps real; a marker
is a position, not a row reference); queueing or retrying commands dropped
while disconnected (a resend queue is a decision about duplicates after
reconnect — stage 5's resync seam); byte-based accounting or a per-session
writer task for the directed lane (stage 4, where the socket makes clients
plural and untrusted); unbounding `search_tx`/`reads_tx` (both ends always
drain now, and that backpressure is core↔storage, not core↔client); a third
storage trace line for the marker write (one row is not a measurement); unread
counts, activity state, or anything rendering a marker (stage 2's buffer
list); editing the `COUNT(*) == MAX(seq)` contiguity assert (it is the dedup
invariant, and `wait rows` exists so the script never needs to); and new
dependencies (nothing here needs one).
```

**Status:** complete. Shipped per the JIT detail, with the lane rewrite as the
expensive half: `Bus::direct` is the one synchronous primitive at 4096 slots, so no
path in the core loop awaits a client, and the attach replay is inline rather than a
task. Markers set, broadcast, persisted, and handed to a process that never dialled
anything; `quit` drains, proved by counting the responses that used to be discarded
(3 in b.out, 502 in c.out). Four ratchet-forced file splits, all pure moves along
named seams — and the review caught the 65-buffer attach test passing against the
engine it was written to pin, which is recorded in the addendum as the lesson of the
prompt.

---

## Prompt 10a — Network config file

**Item:** Network config and credentials.
**Branch:** `prompt-10a-config`

The TOML config file (§5.8): networks (host, port, tls_ca, caller-assigned ids,
stable names), nick, autojoin — as **seed data only**: the
config-vs-runtime-state question is answered (BUILD-LOG, 2026-08-10 — the
database owns runtime state and the program never writes the config file). The
debug CLI runs from config alone for everything except credentials. **Deliberately no
credential surface of any kind in this half**: SASL stays on the
`SUPERNAUT_SASL_PASSWORD` env bridge until 10b replaces it — the original
prompt's fence warned that config-without-credentials ships the
plaintext-password trap, and the answer is a config format that never holds or
references a password in any version, not an unsplit prompt.

```
The user's file becomes the authority on network identity: after this prompt a person
writes one TOML file by hand, runs `supernaut session` with no connection flag of any
kind, and watches it register and land in a channel it was never told to join — while
a malformed file is refused, by key and by network, before anything dials.

Four places the stub, the notes, and the north star pull apart, surfaced rather
than resolved silently:

- The stub says "caller-assigned ids"; §2.1 says a product that needs config-file
  editing to work has failed, and hand-numbering networks is exactly that ceremony.
  Resolved: ids **are** caller-assigned in the sense core.rs means — assigned
  outside havoc-core, distinct from the storage row id — but assigned by the
  *loader*, not typed by a human: networks sorted by name, `NetworkId(1..N)`, no
  `id` key in the schema. The precondition was checked, not assumed: no wire
  `NetworkId` is ever persisted (`ensure_network` in
  crates/havoc-core/src/storage/rows.rs keys the `network` table on name, and
  `buffer.network_id` references the storage row), so renumbering across runs is
  unobservable. If a wire NetworkId ever gets persisted or cached across
  restarts, the id becomes a config field, with the uniqueness rule and the
  migration story that implies.
- §3.1 "works well before configuration" against a prompt that makes config
  mandatory. Resolved: mandatory for the `session` subcommand only. The
  no-subcommand path (`supernaut --data-dir <p>`) still opens the store and
  reports with no config file anywhere, which is the works-before-configuration
  property stage 1 can honestly claim; the product's answer is stage 2 item 7's
  first-run, and it can be neither a flags fallback nor a silent config write (the
  governing decision forbids the second). Keeping the flags path alongside config
  is the real trap: live-run.sh would keep exercising the flags, and config —
  the surface every later stage builds on — would be the decorative one.
- Note 6 asks for the loopback-only plaintext rule to be rehomed, but §2.3 only
  demands a loud opt-in, so that rule has always been stricter than the north
  star. Resolved: keep it, in config validation, because it is the debug-grade
  rule stage 1 shipped and relaxing it is a product call belonging to the
  stage-2 first-run, where a user with a plaintext LAN bouncer actually appears.
  And `--allow-plaintext` is **deleted**, not kept beside the config key:
  security is per-network, so a process-global flag cannot say which network it
  blesses — the opt-in belongs where the host is.
- Note 9b's quit hazard presupposes a config-seeded autoconnect. This prompt
  deliberately builds none: `connect` stays an explicit verb, because session D's
  entire proof is a process that dials nothing while holding a config that names
  the network it would dial. The note is discharged by absence, and its warning
  travels to PLAN stage 2's embedded wiring, which is where autoconnect lands and
  where a startup-issued request nobody typed becomes real.

The work:

- The schema, one file, `$XDG_CONFIG_HOME/supernaut/config.toml` (or
  `$SUPERNAUT_CONFIG_DIR/config.toml`, or `$HOME/.config/supernaut/config.toml`):

      # Seed data. Supernaut never writes this file.
      nick = "alice"

      [networks.libera]
      host = "irc.libera.chat"
      autojoin = ["#supernaut"]

      [networks.ergo-local]
      host = "localhost"
      port = 6667
      plaintext = true

  The **table key is the stable network name** — the same string
  `ensure_network` keys the `network` table on, which is what makes config
  identity and storage identity one thing rather than two that must agree.
  Per-network keys: `host` (required), `port` (optional, defaulting to 6697 with
  TLS and 6667 with plaintext — opinionated defaults are the product, §2.1),
  `tls_ca` (optional PEM path, resolved relative to the config file's own
  directory so a generated fixture directory is portable), `autojoin` (optional,
  default empty), `plaintext` (optional bool, default false). `nick` is
  top-level and global; `username` and `realname` stay derived from it in the
  lowering, as session.rs derives them today. No per-network nick override, no
  `server_name` key for an SNI/connect-host split: both are one field each when a
  second network or a bouncer makes them real (PLAN stage 6), and neither is
  observable in a one-network stage.
- The map form is chosen over `[[network]]` + `name` **for the 9a note**: two
  `[networks.libera]` tables are a TOML-level error, so name uniqueness is
  enforced by the file format and is unrepresentable in the parsed type
  (`BTreeMap<String, NetworkEntry>`) rather than being a validation pass we could
  forget to write. `announce` in crates/havoc-core/src/core/reads.rs keeps
  building its `HashMap<&str, NetworkId>` unchanged, and gains one doc sentence
  saying the silent-collapse hazard is now a config-load error — there is nothing
  to assert, because there is nothing to represent. Names are case-sensitive, to
  match the `network.name TEXT UNIQUE` column's default collation; `Libera` and
  `libera` are two networks and two rows, deliberately and consistently.
- Where it lives, and the seam: `crates/havoc-core/src/config.rs` (new, added to
  lib.rs's module list) owns the schema, validation, and the lowering
  `Config::into_networks(self) -> HashMap<NetworkId, NetworkSettings>` —
  havoc-core owns config per PLAN's module table and §4.5. It does **no file
  I/O**: `config::parse(text: &str, base_dir: &Path) -> Result<Config, String>`,
  so every rule is testable without touching disk, and `String` errors rather
  than an enum because nothing branches on the variant — the binary prints it and
  exits, exactly as `search::parse` already does. Path resolution and the file
  read stay in the binary beside `default_data_dir` in
  crates/supernaut/src/main.rs (`default_config_path`: `SUPERNAUT_CONFIG_DIR`,
  then `XDG_CONFIG_HOME`, then `$HOME/.config`, else an error naming the env
  var) — locating files is the binary's job, parsing them is core's. Note the
  name collision with the shipped `connection::Config`: core.rs already aliases
  that as `ConnectionConfig`; alias at an import site if it bites, do not rename a
  shipped type.
- Validation, all of it before anything dials, every message naming the network
  and the key: `nick` non-empty and whitespace-free (whitespace breaks the NICK
  line itself, which is our bug, not the server's policy); `host` non-empty;
  network name non-empty; `plaintext = true` with a non-loopback host is an error
  naming the host (the same `localhost`/`127.0.0.1`/`::1` string check
  session.rs's `is_loopback` does today, moved here and deleted there — it is a
  string check, not name resolution, and that is unchanged); `plaintext` together
  with `tls_ca` is an error, because it asks for a trust anchor on a connection
  with no trust. Deliberately **not** validated: channel names in `autojoin`
  (CHANTYPES comes from ISUPPORT, so we would be guessing, and the server's
  refusal is already loud) and `tls_ca`'s existence as a file (the TLS connector
  reports that, once, where the failure actually is).
- `#[serde(deny_unknown_fields)]` on both structs, plus a **named refusal for
  credential-shaped keys** (`password`, `pass`, `sasl_password`,
  `nickserv_password`): the generic unknown-key error already rejects them, but a
  message that says credentials never live in the config file (§5.8) and points at
  the keyring 10b brings is what makes the prompt's hard constraint executable
  rather than aspirational. This is also why `deny_unknown_fields` is not
  optional: a silently-ignored typo'd key is the config bug class that costs an
  evening, and it is what will make 10b's `sasl_account` addition detectable
  instead of ambiguous.
- The dependency: `toml` in havoc-core, `default-features = false, features =
  ["std", "parse", "serde"]`, plus `serde` with `derive` (havoc-core has no serde
  edge today; havoc-ipc's is separate). **Both are already on `DEP_ALLOWLIST`**,
  seeded from NORTH-STAR §5 — so no allowlist edit and therefore no
  scripts/test-checks.sh fixtures are owed; the BUILD-LOG decision entry still
  is. Argue the choice there: hand-rolling TOML is rejected outright (quoting,
  escapes, dotted keys, datetimes — a parser is a commodity and ours would be the
  buggy one), and dropping the `display` feature means the *serializer is not
  compiled in*, so "the program never writes the config file" is a capability the
  binary does not have rather than a rule it obeys. Name the transitive set
  (`toml_parser`, `winnow`, `toml_datetime`, `serde_spanned`, `serde_core`) —
  the allowlist check reads declared deps only, so hygiene here is an argument,
  not a check.
- CLI surface, crates/supernaut/src/session.rs: `--host`, `--port`, `--nick`,
  `--join`, `--tls-ca`, and `--allow-plaintext` are **deleted** (replace, don't
  deprecate). New: `--network <name>`, optional when the file names exactly one
  network and an error naming the candidates when it does not. `--sasl <account>`
  and `SUPERNAUT_SASL_PASSWORD` stay untouched — 10b deletes both with the
  keyring, and deleting the bridge here would leave no way to authenticate.
  `--data-dir` and `--trace-irc` stay: a data-dir key in config would make "where
  is my history" answerable only by reading two files, and a trace flag is
  diagnostics, not seed data. `const NETWORK: NetworkId = NetworkId(1)` dies;
  `SessionState` carries the resolved `network: NetworkId`. Core is spawned with
  **every** configured network, not just the selected one — that is the
  `HashMap<NetworkId, _>` §6.9 asks for, it costs nothing while nothing connects,
  and it is what lets `announce` resolve buffers belonging to a configured network
  this process never dialled (`send` then answers "buffer's network is not
  connected", which is already the right sentence). The SASL config is injected
  into the selected network's `NetworkSettings.connection.sasl` after lowering,
  in one place, marked as the site 10b replaces — config lowers with `sasl: None`
  always, because config has no credential surface to lower from.
- One loud line at session start, as prompt 6 shipped: the plaintext warning
  now names the network as well as the host, and the extra-trust-anchor line names
  the network and the resolved PEM path.
- `debug-*` rows: **abandoned, not migrated** — and the abandonment is made
  visible and is reversible. No migration, because a `debug-<host>` row would
  have to be *guessed* onto a config network by host string, and guessing wrong
  merges two networks' histories irreversibly, which is the one unrecoverable
  outcome available here; and because no user has irreplaceable history in a
  temp-dir debug row. Visible: `announce`'s silent `continue` over an
  unconfigured network name becomes one greppable line per skipped network
  (`orphan network <name>: N buffers not announced (not in config)`), collected
  during the loop and printed after it. Reversible: because storage keys networks
  on the same stable name config now supplies, a person gets those buffers back
  by adding `[networks."debug-localhost"]` to the file — which is precisely why
  no migration is owed. Honest about the remaining hole: orphan history is still
  reachable through `Search`, which has no network filter, so a hit can carry a
  BufferId the client cannot name — the same advisory-not-a-boundary gap PLAN
  stage 4 already owns for `FetchBacklog`; it is recorded there, not fixed here.
- `in:` **stays unscoped**, and stops being an accident: the accretion *cause*
  dies with `debug-<host>` (stable names mean one row per network, not one per
  host string), while the cross-network union of one channel name remains real and
  now rare. Scoping is deferred to stage 2's `/search`, which owns the grammar
  question (`in:net/#chan` versus a `network:` filter) and is the only consumer
  that can render which network a hit came from — the debug CLI prints
  `buffer=<id>`, so a scope filter would be unobservable here. What lands instead
  is the honest form of a deferral: the union is stated in the doc comments on
  `SearchSpec.buffer` (crates/havoc-core/src/search.rs) and `run_search`
  (crates/havoc-core/src/storage/query.rs), and **pinned by a test** beside
  `search_hits` in crates/havoc-core/src/storage/tests.rs — two network rows, one
  buffer name, `in:#x` returns both — so stage 2 changes a documented behaviour
  with a failing test rather than discovering an assumption.
- Tests, in a new crates/havoc-core/tests/config.rs (integration, because every
  rule is on the public surface, and it keeps config.rs well under the 400-line
  ratchet): a full file parses and lowers; ids are 1..N in name order, pinned; a
  duplicate `[networks.x]` table is a parse error (the 9a note's invariant, and
  the test is what makes "the format gives it to us" a claim rather than a hope);
  an unknown key errors; `password = "..."` errors *by name*; `plaintext` with a
  non-loopback host errors and with `tls_ca` errors; a missing or whitespace
  `nick` errors; the two default ports; `tls_ca` resolved relative to
  `base_dir`. No storage schema change and no migration: nothing about network
  identity on disk moves.
- scripts/live-run.sh, in this order. First a `write_config <dir> <nick>
  [autojoin]` helper emitting `$dir/config.toml` with `[networks.liverun]`,
  `host = "localhost"`, the run's `$TLS_PORT`, and `tls_ca = "$WORK/fullchain.pem"`
  (watch `set -e`: a trailing `[ -n "${3:-}" ] &&` test must not be the function's
  last command). Every session then runs with `SUPERNAUT_CONFIG_DIR=$WORK/config-<x>`
  and **no `--host`, `--port`, `--nick`, `--join`, `--tls-ca` or
  `--allow-plaintext` anywhere in the file**; A and D pass `--network liverun`
  explicitly because that name coupling is the proof, B and C omit it to exercise
  the single-network default. A's `#supernaut` join becomes config autojoin, which
  deletes both its `join #supernaut` verb and the explicit re-join after the ergo
  restart (prompt 6's autojoin re-fires on the fresh machine) — and the row
  arithmetic the marker assertions depend on is *preserved exactly*: A's join is
  still row 1 and A's post-restart re-join still row 4, so `wait rows #supernaut
  4 10` is untouched. Per note 9b, the autojoin sync is `wait rows #supernaut 1`,
  never `wait message`, which would hang until a human speaks. `join #flood`
  stays an explicit verb, so both paths are exercised and the flood segment is
  unchanged. Session D's `--host localhost` coupling is deleted in this same
  commit (note 9a): D resolves #supernaut because its config names the network A's
  config named, and the comment says so instead of explaining a host-string
  accident. New session E, after D, over A's data dir with a config naming
  `[networks.elsewhere]`: its `wait buffer #supernaut 3` times out (invocation
  tolerated with `|| true`), and the assertions are that e.out carries `orphan
  network liverun` and no `buffer-created .* name=#supernaut` — abandonment,
  observed. `quit` deadlines stay as they are: with no autoconnect and with
  autojoin issuing no `Request`, `outstanding` contains exactly what each script
  segment named, so note 9b's early-quit failure mode is not reachable — and the
  comment above B's `quit` says that, so a later autoconnect does not silently
  re-open it.
- Docs, in the same PR. Four BUILD-LOG decision entries, one per rejected
  alternative argued above: the schema shape and derived ids (over `[[network]]`
  with an explicit `id`, over name-hashed ids); the `toml`/`serde` dependency
  (over hand-rolling, over default features — the missing serializer is the
  point); the debug CLI's connection surface moving wholly into config (config
  mandatory for `session`, the six flags deleted, plaintext as a per-network key —
  over flags-as-fallback, over auto-writing a default file, over keeping
  `--allow-plaintext` beside the key); and network identity after config (orphaned
  rows abandoned-but-reversible, `in:` left unscoped and pinned — over a
  host-string migration, over grammar invented here). PLAN.md gains the deferrals
  in the same turn: stage 2's search UI (`in:` network scoping, naming the pinned
  test), stage 2 item 7 (first-run may not inherit "config is mandatory" and may
  not write the file — the governing decision), stage 2's embedded wiring
  (autoconnect at startup, carrying note 9b's warning forward), stage 6
  multi-network (per-network nick, SNI `server_name`, and an explicit config `id`
  if a NetworkId is ever persisted). Prompt 10b's carry-forward gains two notes:
  the SASL injection site in session.rs that its keyring replaces, and that
  `sasl_account` must move out of the credential-shaped-key refusal list and into
  the schema in one commit. All seven of this prompt's notes are deleted and
  recorded under `**Carry-forward consumed:**` as one act. Status lines: the
  queue's `11/12 complete. Next: prompt 10b.` and README's 10a row, together.
- Budget: this is comfortably inside the 800-line tripwire (which counts
  `crates/` only, so the live-run rewrite is free) — config.rs plus its tests is
  the bulk and session.rs shrinks. If it does push past, what trails, in order,
  is (1) session E and the orphan line, (2) the credential-shaped-key refusal
  list (`deny_unknown_fields` still refuses them, just less kindly), and (3) the
  `in:` union pinning test. Never the live-run config rewrite, never the
  duplicate-table test, never session D's config. Measure the ratchets after:
  longest-file ceiling stays 400.

Acceptance: by hand, write a config into an isolated SUPERNAUT_CONFIG_DIR — `nick`,
one `[networks.liverun]` table with the local ergo's host, port and `tls_ca`, and
`autojoin = ["#supernaut"]` — and run `supernaut session --network liverun
--data-dir <tmp>` with no connection flag of any kind: watch it register and land
in #supernaut without typing `join`, the seed observed as a Join row (`wait rows
#supernaut 1`). Then break the file four ways and read what comes back before
anything dials: a second `[networks.liverun]` table (a parse error from the format
itself — which is why name uniqueness is an invariant here and not an assumption
inside `announce`), a `password = "..."` key (refused by name, pointing at the
keyring 10b brings), `plaintext = true` against a non-loopback host, and no file
at all (the resolved path and SUPERNAUT_CONFIG_DIR both named, exit non-zero).
Then run scripts/live-run.sh: every session runs from a generated config with no
`--host` anywhere in the script, session D resolves #supernaut because its config
names the same network A's did rather than because both were pointed at one host
string, and session E — same data dir, a config naming a different network — is
told about no buffers and says `orphan network liverun` out loud. Separately,
`supernaut --data-dir <tmp>` with no config file anywhere still prints the
name/version and history lines and exits 0. cargo test --workspace and make check
green, ratchets not worsened.

Do not: add any credential surface — no keyring, no encrypted-file fallback, no
password field, and not even an account-name reference (10b; and the refusal list
exists so the schema cannot acquire one by accident in any version); delete
`--sasl` or `SUPERNAUT_SASL_PASSWORD` (10b deletes both with the keyring that
replaces them — deleting the bridge here leaves no way to authenticate at all);
write the config file, ever, from the program or from a test (the governing
decision; fixtures write their own files, and the parser is compiled without its
serializer so the capability is absent rather than merely unused); autoconnect
configured networks at startup (stage 2's embedded wiring — and session D's proof
requires a process that dials nothing while holding a config that names what it
would dial); add network scoping or any new filter to the search grammar (stage
2's `/search` owns the grammar and is the only consumer that can render a hit's
network; the union is pinned by a test here instead); migrate `debug-*` rows,
touch the storage schema, or add a migration (network identity on disk is already
the name config supplies, and a host-string guess merges histories
irreversibly); put anything but network seed data in the file — no theme, no
keybindings, no ignore lists or aliases, no `data_dir`, no retention (stage 2's
theme file, paths stay flags/XDG, retention is a PLAN Still-open); add a
`config`/`config check` subcommand or hot reload (the session's own refusal is
the diagnostic today; live reload is §7.8's, with the theme file); add a
`--config` flag or any new env knob (SUPERNAUT_CONFIG_DIR is the override
CLAUDE.md names, and one knob per location is the whole point); grow the CLI
grammar to multi-network verbs like `connect <name>` (one network per session
process; multi-network drive is stage 6's, and the map core already holds is the
part that must exist now); change the wire — PROTOCOL_VERSION stays 1 and no
Request/Response/Event variant is added or altered (unknown variants are not
serde-tolerant; stage 4's handshake is where additions become negotiable); add
platform detection to live-run.sh (still the dogfood Mac, still waiting on CI);
and add any dependency beyond `toml` and `serde` in havoc-core.
```

**Status:** complete. Shipped per the JIT detail; the six connection flags are gone
rather than deprecated, and the live run's seq arithmetic survived A's join moving into
config autojoin untouched — marker still seq 3, re-join still row 4, 42/42 assertions.
Two deviations are recorded in the log entry: the multi-network deferral landed under
PLAN **stage 5** item 1, where multi-network actually lives, not the stage 6 the detail
named; and `parse` deserializes the text twice (once as a `toml::Table` for the
credential-key scan, once as `Config`) so unknown-key errors keep their spans. Nothing
was deliberately left untested except `default_config_path`'s env precedence, which has
no unit test but is exercised by every by-hand and live-run session through
`SUPERNAUT_CONFIG_DIR`.

---

## Prompt 10b — Credentials and the stage acceptance run

**Item:** Network config and credentials.
**Branch:** `prompt-10b-credentials`

Credentials done right: `keyring` where available with an encrypted-file
fallback; the config references an account name only; **the
`SUPERNAUT_SASL_PASSWORD` env bridge is deleted** (replace, don't deprecate) —
never plaintext secrets in the config file, ever (§5.8). Ends with the stage-1
acceptance run: connect to a real network from config alone, join, log, kill,
restart, search — the "Done when" of the stage, driven end to end.

```
The secret leaves the process's environment for the OS keychain: after this prompt a
person writes `sasl_account = "alice"` in their config, pipes the password once into
`supernaut credential set <network>`, and every later session authenticates with SASL
from a config file plus a keychain entry — no flag, no env var, and no path by which a
password can be typed into the config file or printed into a trace.

Four places the plan, the stub, and reality pull apart, surfaced rather than resolved
silently:

- **PLAN item 10 and §5.8 both promise "`keyring` with encrypted-file fallback"; this
  prompt ships the keyring half only, deliberately.** A fallback file needs a key, and
  every honest answer to "from where" is out of stage 1's reach: a passphrase needs a
  no-echo prompt (the terminal is stage 2's, item 7), and a KDF + AEAD pair
  (`argon2` + `chacha20poly1305`) is two dependencies off the allowlist bought for a
  consumer that does not exist — the dogfood machine is a Mac with a working login
  keychain. The off-the-shelf option was checked and is worse, not better:
  keyring 4's `db-keystore` store is Turso-backed (a *second* SQLite implementation
  beside rusqlite) and still takes its data-encryption key as a hex string from the
  application, so the key problem is unsolved and the dependency is large. A file
  "encrypted" with a key stored beside it is obfuscation labelled encryption, which is
  worse than no fallback because it reads as safe. What ships instead is a loud
  unavailability error (below) that names the deferral, so the gap is spoken rather
  than discovered. Amend PLAN stage 1 item 10's wording in this PR — a stale doc is
  fixed in the commit that stales it — and file the fallback as a carry-forward note on
  **PLAN stage 4 item 3** (a daemon started at login on a box with a locked Secret
  Service is its first real consumer), with stage 6 item 3 (Release) as the deadline,
  since that is where "runs on Linux" becomes a user promise. One decision entry, with
  the trigger to revisit stated: the first target without a working platform store.
- **Stage 1's "Done when" requires SASL against Libera.Chat; whether a registered
  Libera account exists is the user's call, not this session's.** Prompt 6 set the
  precedent and recorded the honest entry ("TLS + registration verified, SASL
  ergo-only"). The coordinator has answered for this run: no account exists, so the
  Libera leg is TLS-without-SASL, SASL-from-the-keyring is proven against ergo in the
  scripted run, and BUILD-LOG's Live run section says exactly that — the stage closes
  with a named gap rather than a claimed run.
- **The status bump to 12/12 mechanically fuses the stage retrospective into this
  prompt's PR.** Check 10 in scripts/check-docs.sh fails every commit once
  `**Status:** 12/12` is true and `## Retrospective — Stage 1` is absent, and check 6
  ties the README's ✅ row count to the same number — so there is no green state in
  which 10b is merged, the README says done, and the retrospective is still owed.
  Decided: **the bump, the README row, and the retrospective entry are the last commit
  of this PR**, with the cold-start drill dispatched against the finished branch (it has
  every file the fresh sub-agent should see). The alternative — leave the status at
  11/12 and open a second `stage-1-retrospective` PR carrying bump + README +
  retrospective — is rejected because it parks the repo in a state where the queue says
  "Next: prompt 10b" over merged 10b code, which is precisely the lie the cold-start
  drill exists to catch. The retrospective is still stage-boundary work with its own
  five sections (template in BUILD-LOG.md); it is not this prompt's work, it merely
  rides this prompt's PR.
- **"Kill and restart the process losing nothing" meets the ~100ms batch timer.** Writes
  are batched (prompt 7), so `kill -9` can lose the batch in flight. Do not quietly
  redefine the promise: in the acceptance run, record the row count observed before the
  kill and the count read out of sqlite3 after the restart, and state the difference —
  zero, or at most one in-flight batch, whichever was actually measured.

The work:

- **Config gains exactly one credential-adjacent key: `sasl_account`.**
  `NetworkEntry` in crates/havoc-core/src/config.rs takes
  `pub sasl_account: Option<String>` — "authenticate with SASL PLAIN as this account;
  the secret comes from the credential store, keyed by this network's name". Add it to
  the module-doc schema block in the same edit, and **do not add it to
  `CREDENTIAL_KEYS`**: an account name is not a secret, and that list's message says
  credentials, which would then be a lie (carry-forward). The list's four names stay,
  and its message gets one sentence upgraded from aspiration to instruction — it now
  names the replacement: set `sasl_account` here and run
  `supernaut credential set <network>`. `deny_unknown_fields` means the schema addition
  and the refusal-list tests move in one commit, which is the point: the tests in
  crates/havoc-core/tests/config.rs must be updated in this commit or the build says so
  loudly.
- **Validation, stating the boundary rather than the exclusion** (prompt 10a's Learned
  (2)): do **not** police what a network permits as an account name — that is the
  network's business and we would be guessing, exactly as with channel names. What is
  ours is the payload we frame: SASL PLAIN sends `authcid\0authcid\0password`, so a NUL
  or control character in the account splits or corrupts our own payload, and
  whitespace is a quoting mistake we can name for free. Refuse empty-after-trim,
  whitespace, and control characters, with the message naming the network and the key.
- **`sasl_account` together with `plaintext = true` is a config error.** SASL PLAIN puts
  the password on the wire in cleartext; the loopback-only rule bounds the exposure to
  the box, but §2.3's whole shape is that you say what you trust and never switch trust
  off — and 10a's review flagged this pairing under the old `--sasl` flag. The refusal
  names both keys and says the mechanism sends the secret in cleartext. It is stricter
  than §2.3 demands, like the loopback rule beside it, so it travels as a carry-forward
  note to PLAN stage 2 item 7 where the plaintext-LAN user first appears and it becomes
  a product call.
- **A credential store module in the binary, not in havoc-core:**
  crates/supernaut/src/credentials.rs, with `keyring = "4.1.6"` (default features —
  `default = ["v1"]`) declared in crates/supernaut/Cargo.toml. `keyring` is already on
  DEP_ALLOWLIST, so no allowlist edit and no scripts/test-checks.sh fixture is owed; a
  decision entry is, for two rejected alternatives. (1) *havoc-core owning the store* —
  rejected: core's tests would depend on the developer's login keychain and on OS state,
  and the credential store is an OS integration belonging to whichever process
  assembles the core (today `supernaut`, at stage 4 `havocd` — a binary either way).
  The type that crosses the seam is already `SaslCredentials`, which core owns.
  (2) *`keyring-core` + `apple-native-keyring-store` directly* — rejected because we
  want precisely the platform default, the store crates are target-gated so a Mac build
  compiles only the Apple one, and `keyring` is the name already on the allowlist. The
  trigger to revisit is the day we need a *non*-default store — which is the deferred
  file fallback, and is exactly what keyring-core exists for.
- **Keyed by service `"supernaut"` and user = the config network *name*.** Never
  `NetworkId` (carry-forward: `into_networks` renumbers `1..N` from sorted order, so a
  keyring entry keyed by id would be the first thing to persist a wire id and would
  silently re-point one network's credentials at another's after an unrelated config
  edit). Not `<network>:<account>` either: one entry per network is the minimum
  ambiguity, `credential set <network>` then needs no account argument because config
  already holds it, and editing `sasl_account` does not silently orphan a secret you
  set five minutes ago — a mismatched account fails loudly at the server instead, which
  is fail-closed and honest. Record the rejected alternative. Say out loud in the
  module doc what this costs: one keychain namespace is shared by every
  `SUPERNAUT_CONFIG_DIR`, so a network named `liverun` in two config dirs is one
  secret. Hashing the config path into the service string would fix that and make the
  entry unfindable in Keychain Access and unshareable with stage 4's daemon — rejected.
- **`supernaut credential set <network>`**, a second `Command` variant in
  crates/supernaut/src/main.rs, reading the secret from **stdin only**. Never argv:
  `ps` is world-readable, which is the reason prompt 6 refused `--sasl-pass`. It
  resolves the config file the same way `session` does and refuses a network the file
  does not name, reusing `resolve_network` from session.rs (make it `pub(crate)`) so
  there is one such sentence in the binary; it refuses a network with no `sasl_account`
  ("add it first, or the secret would never be read"); it refuses an empty secret; it
  trims exactly one trailing newline and nothing else; and it prints one confirmation
  line naming service and account and **never the secret**. When stdin is a TTY it
  prints one stderr line saying the input is not hidden — suppressing echo means either
  a new dependency or raw-mode terminal code, and the terminal belongs to stage 2 item
  7's first-run, which is where the no-echo prompt is filed as a carry-forward note.
- **The lookup replaces the env read at the one existing injection site**, `run()` in
  crates/supernaut/src/session.rs, after lowering (carry-forward: the shape does not
  move, only where the two halves come from; `into_networks` keeps lowering
  `sasl: None`, because config still has no secret to lower). Note the ordering trap:
  `into_networks` consumes the config, so capture
  `config.networks.get(&selected).and_then(|e| e.sasl_account.clone())` *before* it.
  Resolve for the **selected network only** (carry-forward) — the map deliberately
  holds every configured network, and walking it would touch the keychain for networks
  this process will never dial; leave a comment saying so, or someone will "fix" it into
  a loop. When there is no `sasl_account`, do not touch the keychain at all: sessions
  B–E of the live run must be keychain-free. `mechanisms` stays hardcoded
  `vec![SaslMechanism::Plain]`: a mechanism list in config is a knob with one legal
  value until stage 6's CertFP.
- **Three distinct failures, all before anything dials, all non-zero exits.** (1) Config
  names an account and the store has no entry (`keyring::Error::NoEntry`): name the
  service, the account, and the literal `supernaut credential set <network>` to run.
  (2) The store itself is unavailable (any other error): say the OS keyring is
  unavailable, quote the platform error, and say there is no encrypted-file fallback
  yet, naming the PLAN deferral — a Linux headless user must get the true sentence, not
  "no secret". (3) `Ambiguous`: two matching items exist — say which service/account
  pair to de-duplicate in Keychain Access. Verify the variant names against the
  keyring 4.1.6 docs rather than from memory; the `v1` module re-exports keyring-core's
  `Error`.
- **`--sasl` and `SUPERNAUT_SASL_PASSWORD` are both deleted**, not deprecated
  (carry-forward, and 10a's precedent for the six connection flags): a fallback is what
  keeps live-run.sh exercising the fallback and leaves the real surface decorative. No
  env knob replaces it — a secret in the environment is the same plaintext §5.8 forbids
  in the file, visible in `ps eww`, and inherited by every child. Update session.rs's
  module doc in the same commit: it currently promises this prompt's keyring by name.
- **Redact the outbound AUTHENTICATE payload in the IRC trace.**
  crates/havoc-core/src/connection/actor.rs `eprintln!(">> {line}")`s every line it
  writes, so `--trace-irc` today prints the base64 SASL payload — the secret, base64 is
  not redaction — and scripts/live-run.sh copies that trace to `.cache/last-a.trace`,
  i.e. it persists outside `$WORK`. Add a small helper applied at the outbound trace
  sites: a line whose command is `AUTHENTICATE` and whose argument is neither `+` nor a
  `SaslMechanism::as_str()` name prints as `AUTHENTICATE <redacted>`. At the trace only
  — never at `send_line`, or the state-machine corpus in
  crates/havoc-core/tests/state_machine.rs (which asserts the real payload from the
  hand-written `sesame` fixture) breaks, correctly. Unit-test the helper in actor.rs's
  existing `#[cfg(test)]` module: mechanism line untouched, `+` untouched, payload
  redacted, `PRIVMSG` untouched. Inbound stays verbatim: what we read is a server
  challenge, never our password.
- **scripts/live-run.sh: the harness's own credential, and this is the hard part.**
  `write_config` is fixed-shape (carry-forward), so extend it with a fifth optional
  `sasl_account` argument emitted from an `if` block — never a trailing `[ -n … ] &&`
  test, which under `set -e` makes an absent value the function's failing last command
  (the comment already there says why). Session A's config gains
  `sasl_account = "alice"`; its `--sasl alice` and `SUPERNAUT_SASL_PASSWORD` go. Getting
  `$FAKE_PASS` into the store, in order, after `cargo build` and before A launches:
  (1) `security delete-generic-password -s supernaut -a liverun` guarded with `|| true`,
  clearing any item left by a previous build — the debug binary is ad-hoc signed, so its
  cdhash changes every rebuild, and a keychain item whose ACL names last week's build is
  what turns A's read into a GUI prompt, i.e. a hang; (2)
  `printf '%s' "$FAKE_PASS" | "$BIN" credential set liverun` with
  `SUPERNAUT_CONFIG_DIR="$WORK/config-a"`, so the item is created by the very binary
  that will read it; (3) the same `delete-generic-password` in `cleanup()`, before the
  `KEEP_WORK` early return — the harness leaves no credential behind, kept or not. Wrap
  every `security` call and the `credential set` in a bounded watchdog helper (background
  the command, kill it after ~10s, fail loudly) because this script must never sleep and
  must never hang: a keychain dialog is invisible in CI and indistinguishable from a
  deadlock. Extend the existing "session A never autojoined" failure message to name the
  keychain as a candidate cause. The header comment must state plainly what the run does
  to the machine: it creates and removes one generic-password item, service `supernaut`,
  account `liverun`, holding the recognisably-fake `fake-livetest-passw0rd`, in the
  **login** keychain — because keyring's macOS store uses the User keychain by default
  and pointing it elsewhere needs the rejected keyring-core route. `security … -g` is
  never used anywhere: it prints the secret.
  **The one unverified assumption, to be settled by running it and recorded either way:**
  whether `security delete-generic-password` on an item created by another application
  prompts for authorization. If it does, the documented fallback is to seed with
  `security add-generic-password -A -U -s supernaut -a liverun -w "$FAKE_PASS"` (`-A`
  grants every application access without a dialog) at the cost of not exercising the
  product's own write path in the harness. Primary is the product path; whichever ran
  goes in BUILD-LOG's Learned with what was observed.
- **New live-run assertions**, beside the ones prompt 6 left in place (`>> AUTHENTICATE
  PLAIN` and the `903` line both stay — they are the regression net proving the keyring
  produced a real credential): `a.trace` contains `AUTHENTICATE <redacted>`; and neither
  `$FAKE_PASS` nor its PLAIN payload (`printf 'alice\0alice\0%s' "$FAKE_PASS" | base64`)
  appears anywhere in `a.trace`, `a.out`, or the copied `.cache/last-a.trace`. That last
  one is CLAUDE.md's never-in-logs rule made mechanical for two lines of shell.
- **The stage acceptance run against Libera.Chat, by hand, budgeted as its own step**
  (carry-forward: with the six flags gone and no `--config` flag by design, this is not a
  one-liner). In order: export `SUPERNAUT_CONFIG_DIR=$(mktemp -d)` and hand-write
  `config.toml` — `nick`, one `[networks.libera]` table with `host = "irc.libera.chat"`
  and no `port`, no `tls_ca`, no `plaintext` (6697 and webpki-roots are the defaults
  being proven), `autojoin` naming two channels: one quiet, one busy enough to produce
  inbound lines within a minute, which is what "watches traffic land" actually requires
  — do not speak in the busy one. Then (per the coordinator's Libera decision) no
  `sasl_account` and the honest entry. Then
  `supernaut session --network libera --data-dir <tmp>`: `connect`,
  `wait registered 30`, watch both channels arrive from autojoin with no `join` typed,
  `wait rows <quiet> 1 60`, `wait message <busy> 20 120`, and note the row count. Then
  `kill -9` the process from another terminal — SIGKILL, not `quit`; the Done-when says
  kill — and restart the identical command over the same `--data-dir`: the attach
  announcement resolves both buffers (`wait buffer <busy>`), `backlog <busy> after:0 50`
  reads lines written by the dead process, `search <token>`, `search from:<nick>
  <token>` and `search in:<busy> "<phrase>"` return hits with wall time recorded, and
  `sqlite3` row counts reconcile with the pre-kill number per the fourth conflict above.
  That whole sequence, verbatim as observed, is what BUILD-LOG's Live run section holds
  for this prompt; it is the stage's Done-when, run rather than asserted.
- **Docs that this change stales, fixed in the same commits:** session.rs's module doc
  and `SessionArgs` docs; config.rs's schema block, `CREDENTIAL_KEYS` comment and
  `into_networks`' `sasl: None` note (which should now say where the secret comes from);
  PLAN stage 1 item 10's fallback wording; PLAN stage 2 item 7 and stage 4 item 3
  carry-forward notes; the queue's status line and README's badge plus the 10b row,
  together, in the retrospective commit.
- **Decision entries owed in BUILD-LOG.md**, one per rejected alternative, written when
  the choice is made and not at the end: the keyring dependency (version, `v1` feature,
  and both rejected store routes); the encrypted-file deferral; keying by network name
  alone; stdin-with-echo over a no-echo prompt; and `sasl_account` + `plaintext` as a
  refusal.
- **Budget.** Estimate ~250 changed lines in `crates/` — config.rs and its tests are the
  bulk, credentials.rs is ~90, session.rs shrinks — against the 800-line tripwire, so
  nothing is planned to trail. If it somehow doubles, the only trailable item is
  `credential set`'s refusal when `sasl_account` is missing (a nicety: without it the
  stored secret merely goes unread), and it is deliberately kept **out** of the
  Acceptance paragraph so that trailing it cannot contradict a step that must happen —
  10a's Learned (1). Watch the ratchets: `longest-file` ceiling is 400 and
  tests/config.rs is already 317; if it crosses, split the test file, because raising a
  ceiling is a decision entry, not a convenience.

Acceptance: with `SUPERNAUT_CONFIG_DIR` pointed at a scratch directory, add
`sasl_account = "alice"` to a config naming the local ergo, run `supernaut session` and
watch it refuse to dial at all — the error naming service `supernaut`, account
`liverun`, and the exact `supernaut credential set liverun` to run; pipe a fake password
into that command and read back one confirmation line that names the entry and not the
secret; run the session again and watch it register with SASL. Set `plaintext = true`
beside the account key and watch `config::parse` refuse the pair by name before anything
dials. Then run scripts/live-run.sh: session A authenticates with a credential that
exists nowhere in its config, its argv or its environment — `>> AUTHENTICATE PLAIN` and
the `903` line still in a.trace, with the payload line now reading
`AUTHENTICATE <redacted>` and neither the fake password nor its base64 anywhere in
a.trace, a.out or .cache/last-a.trace — every other session runs without touching the
keychain, and the run leaves no keychain item behind (`security find-generic-password -s
supernaut -a liverun` finds nothing afterwards). Then the stage itself, by hand against
Libera.Chat from a config file alone: connect over TLS on the stock root store, land in
two channels nobody typed a `join` for, watch real traffic accumulate, `kill -9` the
process, restart it over the same data dir, and read the dead process's lines back out
of `backlog` and out of `search` with `from:`, `in:` and a phrase — with the row counts
before and after, the search wall time, and the SASL-at-Libera outcome recorded in
BUILD-LOG's Live run section as observed rather than as hoped. cargo test --workspace
and make check green, ratchets not worsened, and the final commit carries
`**Status:** 12/12`, the README row, and the `## Retrospective — Stage 1` entry whose
cold-start drill ran against this branch.

Do not: implement the encrypted-file fallback (deferred with its reasoning above and
filed on PLAN stage 4 item 3 — the boundary is that *absence must be loud*: the
store-unavailable error naming the deferral is required here, only the file is not);
add a second credential store of any kind, including an env-var or file store gated
behind a variable "for the harness" (that resurrects the deleted bridge under a new
name, and a path only the harness takes is a path the product never proves — the harness
uses the product's own `credential set`); add `credential get`, `credential status`, or
any verb that prints or checks a secret (a `get` prints what CLAUDE.md forbids printing,
and the session's own startup error is already the diagnostic; `rm`/`rotate` arrive when
a person needs them, not before); prompt interactively for a password, or suppress
terminal echo (the terminal is stage 2 item 7's, where the first-run wizard owns
credential entry; stdin is the seam that works for both a human and a pipe);
put a password, or anything that could hold one, into the config schema — `sasl_account`
is a name, `CREDENTIAL_KEYS` keeps refusing the four secret-shaped keys by name, and the
TOML serializer is still not compiled into havoc-core; add `sasl_account` to
`CREDENTIAL_KEYS` "for symmetry" (the message says credentials; an account name is not
one, and the lie would be in the error a user reads); keep `--sasl` or
`SUPERNAUT_SASL_PASSWORD` in any form (replace, don't deprecate — and one surface means
live-run.sh can only exercise the real one); walk the network map resolving secrets
(§6.9's map holds every configured network; the selected one is the only one this process
will dial, and the rest are somebody else's prompt); key anything in the keyring by
`NetworkId` (derived ids renumber on an unrelated config edit; the name is what storage
keys on too); add a mechanism key, SASL EXTERNAL, or CertFP (the mechanism list is shape
only until stage 6's menu item, which budgets the machine work); touch the connect-time
SASL failure policy in crates/havoc-core/src/connection/actor.rs (fail-closed, reported
once, never retried — prompt 6's design, unchanged by where the password came from, and
a keyring that retried on 904 would turn a wrong password into a lockout); redact at
`send_line` rather than at the trace (the wire must still carry the real payload, and
the corpus asserts it); autoconnect a configured network (stage 2's embedded wiring, and
its early-quit hazard is already noted there); change the wire — PROTOCOL_VERSION stays
1, no Request/Response/Event variant added or altered; touch the storage schema or add a
migration (no credential ever goes near the database, which is the §5.8 rule the
schema's absence of a column enforces); add platform detection to live-run.sh (still the
dogfood Mac, still waiting on CI); and add any dependency beyond `keyring` in
crates/supernaut.
```

**Status:** complete. Shipped per the JIT detail: `sasl_account` is the only
credential-adjacent config key, the secret lives in the OS keyring keyed by the network's
name, `supernaut credential set <network>` reads it from stdin only, and `--sasl` plus the
`SUPERNAUT_SASL_PASSWORD` bridge are deleted rather than deprecated. Two deviations are
recorded in the log entry: the encrypted-file half of §5.8 is deferred to PLAN stage 4
item 3 with the store-unavailable error naming the deferral out loud, and **the stage
acceptance run ran against OFTC, not Libera.Chat**, because Libera network-banned this IP
mid-prompt — so the Done-when's SASL clause closes with a named gap (no registered
account; OFTC advertises no `sasl` at all; keyring-SASL is proven against ergo on every
live run) rather than a claimed run. Deliberately left untested: the `Ambiguous` keyring
arm, which only the secret-service store can construct and which therefore cannot fire on
the dogfood platform at all.

---

<!--
Once a prompt completes, append its outcome under the block, in this shape. Keep it to
two or three sentences and put the reasoning in BUILD-LOG.md, not here:

**Status:** complete. <What shipped differently from the block above, and where that was
recorded.> <Anything deliberately left without a test, and why.>
-->
