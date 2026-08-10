# Stage 1 — The Prompts

**Status:** 4/10 complete. Next: prompt 5.

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

Wire the core together and give it its first driver: the event bus (broadcast to every
attached client), request dispatch with correlation ids, the client/core trait plus
in-process mpsc impl in `havoc-transport`, and a debug CLI in the `supernaut` binary that
drives the core over the typed boundary — connect, join, send, tail events. Live
connect is plain TCP to a **local ergo only**, behind a loud explicit flag (§2.3's
opt-in); this is the prompt that creates `scripts/live-run.sh`, and the project's first
live run.

**Examined for a split and left whole**, because bus, dispatch, transport trait, and
CLI are one wiring seam — none is testable in anger without the others. Revisit if the
plain-TCP connector balloons; it can move wholesale into prompt 6.

Do not: TLS, DNS to real networks, SASL against a live server (prompt 6), persistence
of anything seen (prompt 7), or CBOR framing on the transport (stage 4 — in-process is
typed messages, no serialization).

*To be written out before it starts.*

### Carry-forward

- From prompt 1: **no Cargo edge exists between havoc-core and havoc-transport, in
  either direction.** Both depend only on havoc-ipc (see their `Cargo.toml`s); only
  the `supernaut` binary depends on both. Either shape the wiring so core exposes
  plain channels and `supernaut` adapts them to havoc-transport's trait, or add the
  edge deliberately — with the `scripts/check-docs.sh` boundary-check amendment and
  its fixtures in the same change — not as a mid-session surprise.
- From prompt 2: **`Event` is contractually broadcast, but `SearchResults` is
  request-correlated.** `Event::SearchResults { request, .. }` in
  `crates/havoc-ipc/src/lib.rs` sits inside a type documented "broadcast to every
  attached client" — as shipped, every client would receive every other client's
  search hits. Decide the bus semantics deliberately (broadcast-and-filter vs
  routed correlated events) and record it, or stage 4's multi-client attach
  inherits an information-leak-shaped accident.
- From prompt 2: **`Ack` is fire-and-forget; the debug CLI cannot await outcomes
  via Response.** `ResponseBody::Ack`'s contract is "accepted; resulting state
  lands as Events" — connect/join/send success is only observable by tailing
  correlated events. Design the CLI's command flow around the event stream, not
  the reply.
- From prompt 4: **the line-transport trait was never built — the machine is a
  sync pure function and there is no actor yet.** `Machine::handle_line(&str) ->
  Vec<String>` in `crates/havoc-core/src/connection/mod.rs` is the entire seam
  (deliberate sans-I/O deviation, recorded in the log). This prompt creates the
  actor loop and the transport seam wholesale — and inherits the prompt-1
  constraint the deleted note carried: no Cargo edge between havoc-core and
  havoc-transport in either direction; any line-transport trait lives in
  havoc-core or havoc-ipc.
- From prompt 4: **`handle_line` returns outbound lines only — state changes
  and received messages are silent.** Emitting `ConnectionState` events means
  diffing `Machine::phase()` after every line or widening the return type, and
  the nine-transcript corpus in `crates/havoc-core/tests/state_machine.rs`
  asserts on exactly `Vec<String>` — widening churns it all. Also `phase()`
  folds `State::Failed`'s reason away; a CLI that must print why a connection
  died needs that surfaced. Decide the shape in the prompt text.
- From prompt 3: **supernaut's `main` is open-print-exit behind a closed arg
  grammar.** `data_dir_from_args` in `crates/supernaut/src/main.rs` rejects
  everything except `--data-dir`, which is prompt 3's documented acceptance
  behavior and must survive the CLI rewrite. Its "one flag, no dependency
  justified" rationale expires the moment subcommands exist — schedule the
  arg-parsing dependency decision (entry + allowlist) in the prompt text, not
  mid-session.

---

## Prompt 6 — Live connection, TLS, and reconnect

**Item:** Live connection, TLS, and reconnect.
**Branch:** `prompt-06-live-connection`

The real network path: rustls (TLS by default), real DNS, live SASL against local ergo
plus a Libera.Chat spot-check; disconnect detection and jittered exponential backoff
driven through the state machine's reconnect seam. Capture live transcripts from ergo
and Libera into prompt 4's fixture corpus — that corpus is the durable asset this
prompt feeds.

**Examined for a split (TLS vs reconnect) and left whole**, because both live in the
same connect path; splitting them means two prompts editing one function. Revisit if
the SASL Still-open decision put EXTERNAL/CertFP in scope — that would justify its own
session.

### Carry-forward

- From prompt 4: **the transcript corpus is inline Rust, not the fixture files
  PLAN promised.** The corpus is `Step(&str, &[&str])` tables in
  `crates/havoc-core/tests/state_machine.rs`. "Capture live transcripts into this
  corpus" means either generating Rust from captures or migrating to data files
  first — pick one in the prompt text, or capture produces a second, divergent
  corpus format.
- From prompt 4: **`phase()` folds `Failed` and `Disconnected` together — a
  backoff loop keyed on the phase retries fatal SASL failures forever.**
  Reconnect policy must read `Machine::state()` and treat `State::Failed`
  (fail-closed SASL) as no-retry. There is also no re-arm path: `on_disconnect`
  only marks state and `Machine::start` builds a fresh machine — reconstruct per
  attempt; do not add a `reset()` mid-session.

Do not: CHATHISTORY resync on reconnect (stage 5 — nothing exists to merge into yet),
storage writes (prompt 7), or any TCP listener/inbound anything (§2.4, stage 6 menu at
most).

*To be written out before it starts.*

---

## Prompt 7 — Message ingestion, identity, and batched writes

**Item:** Message ingestion, identity, and batched writes.
**Branch:** `prompt-07-ingestion`

The write path: actor events flow through the bus into the storage thread. `seq`
assigned at insert and returned on the MessageAdded event; `msgid` dedup enforced by
the unique index at the storage layer, not the application layer (§6.4); content-hash
fallback over (nick, text, coarse time bucket) for tagless servers; batched
transactions on a ~100ms/N-message timer with WAL verified (§6.5). The live run floods
a channel via ergo and measures that writes batch instead of fsync-per-line.

**Examined for a split (identity vs batching) and left whole**, because both are the
single insert path, and the flood harness that proves batching is the same one that
proves dedup. Revisit if the content-hash fallback design stalls the session — it can
trail as its own small prompt.

Do not: FTS indexing (prompt 8), backlog reads (prompt 9), or replay/resync dedup
scenarios beyond what ergo can produce today (stage 5 tests against a real bouncer).

*To be written out before it starts.*

### Carry-forward

- From prompt 2: **`msgid` has no berth on the wire `Message`.** The `tags` doc in
  `crates/havoc-ipc/src/lib.rs` excludes msgid as "lifted into fields", but
  `Message` has no msgid field — excluded from tags *and* absent. The
  actor→storage ingest path therefore cannot ride `havoc_ipc::Message`; dedup
  needs msgid at insert. Plan a core-internal ingest type (rows are core's private
  business per prompt 3's fence) or amend the wire type and its doc — decide in
  the prompt text, not mid-session.
- From prompt 3: **`ensure_buffer` silently discards `kind` on conflict.** In
  `crates/havoc-core/src/storage/mod.rs` it does `ON CONFLICT ... DO NOTHING`
  then selects the existing id — a buffer first created as `query` stays `query`
  even when later ensured as `channel`, with no error. Ingest creates buffers
  from live traffic: define re-kind semantics in the prompt or forbid ingestion
  from using this helper.
- From prompt 3: **the `Storage` handle is synchronous — every method parks the
  calling thread.** All methods block on `std::sync::mpsc` `recv()`; actors are
  tokio tasks, and calling this inline from one stalls the executor. Decide the
  bridge (async facade over the job channel, or `spawn_blocking`) in the prompt
  text, not when the flood test hangs. Also: the flood harness owns `PRAGMA
  synchronous` tuning — `Storage::open` deliberately leaves it at the default.
- From prompt 4: **the machine parses and then discards every non-protocol
  message — ingestion cannot tap it.** `handle_message` in
  `crates/havoc-core/src/connection/mod.rs` drops PRIVMSG/NOTICE/JOIN, and the
  parse happens inside `handle_line`, so the actor cannot reach the parsed
  `irc_proto::Message` (tags included) without parsing twice. Decide in the
  prompt text whether the actor parses once and feeds the machine a `&Message`,
  or the machine grows a message output — either restructures the prompt-4 API,
  and the seam choice may pull forward into prompt 5's actor design.
- From prompt 3: **`message.tags` is declared CBOR on disk, but havoc-core has no
  CBOR dependency.** `crates/havoc-core/migrations/0001_init.sql` commits the
  column; ciborium is only havoc-ipc's dev-dep. Writing tags means a runtime
  ciborium dependency in havoc-core — decision entry + it is already on the
  global allowlist — so budget it into the prompt.

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
