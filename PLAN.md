# Supernaut — Build Plan

A terminal IRC client: headless core owning connections/history/state, Ratatui frontend
owning only the viewport, one binary with embedded and daemon modes. **`NORTH-STAR.md`
holds the why and the destination and is the arbiter of scope disputes; this plan holds
the how and the order — it references the north-star rather than restating it.**
"Finished" looks like NORTH-STAR §3: works before configuration, history as a database,
invisible reconnect, modern protocol, a real compiler-enforced boundary, and a face.
Each numbered item is sized to be roughly one prompt / one focused work session, and
each stage ends at a point where the thing is genuinely usable.

---

## 0. Architecture & Decisions

### Module layout

The crate graph *is* the architecture (NORTH-STAR §4.2). The forbidden edges are
enforced by a mechanical check on `Cargo.toml` files, not by review. If
`supernaut-tui` ever needs a type living in `havoc-core`, the type moves to
`havoc-ipc`. Naming (NORTH-STAR amendment, 2026-08-09): **Supernaut** is the app and
everything user-facing; **havoc** is the headless engine family.

| Crate | Responsibility | I/O? |
|---|---|---|
| `havoc-ipc` | Wire types: requests, responses, events, IDs, buffer/network models; protocol version + capability constants | none |
| `havoc-core` | Connection actors, cap negotiation, SQLite storage, event bus, request handling, config | network + disk |
| `supernaut-tui` | Rendering, input handling, layout, theme, viewport state | terminal only |
| `havoc-transport` | Framing and transport impls (in-process mpsc, UDS later); the trait both sides code against | sockets; no business logic |
| `supernaut` | Binary: arg parsing, wiring, mode selection | — |

### Key technical choices

Reasoning lives in NORTH-STAR §5; each bullet is the decision plus the one-line why.

- **SQLite + FTS5 via `rusqlite`** (§5.1) — one embedded store that does append-heavy
  writes, `(buffer, seq)` range reads, and full-text search transactionally. Storage
  runs on its own thread behind a channel (§6.6).
- **Own the connection state machine; `irc-proto` for parse/serialize only** (§5.2) —
  cap negotiation, SASL, and resync are the product; the parser is a commodity.
- **`seq` / `msgid` / `server_time` never conflated** (§4.6) — `seq` orders, `msgid`
  dedups, `server_time` displays. Timestamps structurally unavailable to sort paths
  (§6.1).
- **Windowed backlog API only** (§4.7) — `FetchBacklog { buffer, anchor, limit }`,
  limit capped server-side; there is no "give me the buffer" request, ever (§6.3).
- **Embedded-first process model** (§5.4) — same core, same typed messages, mpsc
  swapped for a socket later. The daemon must cost "a few hundred lines" (§8 M5) or
  the boundary was wrong.
- **Actors on tokio; channels; no shared mutable state across subsystems** (§5.5) —
  `HashMap<NetworkId, _>` from commit one (§6.9).
- **UDS + filesystem permissions as the auth model; CBOR in length-prefixed frames;
  capability handshake, no version lockstep** (§4.8, §5.3).
- **Ratatui + crossterm, immediate mode; we own the wrapped-line cache** (§4.10, §5.6).
- **`rustls`; TOML config; `keyring` for credentials; `nucleo` for all fuzzy matching**
  (§5.8).

### Settled

Repo layout: cargo workspace, crates under `crates/`, Rust stable. Discipline:
warnings-as-errors, clippy strict, rustfmt enforced, zero-warning builds. Dependency
policy: the allowlist in `scripts/check-docs.sh`; adding a dependency requires a
`BUILD-LOG.md` decision entry plus the allowlist edit. Crate boundary: NORTH-STAR §4.2's
forbidden edges, checked mechanically on `Cargo.toml` files. Workflow: one prompt per
branch (`prompt-NN-slug`), one PR, squash-merged green; the status line in the queue
file is machine-read. The north-star is amended by dated appendix, never silently
edited. Design reasoning is in NORTH-STAR §5; process reasoning accumulates as decision
entries in `BUILD-LOG.md`.

### Still open

**This is the single list of open questions.** Anything awaiting a decision belongs
here, whatever `BUILD-LOG.md` entry first raised it — that log is append-only, so
questions buried in it are questions nobody finds. Delete an item from this list when it
is answered, and record the answer as a decision entry.

The blocking annotation is machine-read: `*(blocking: prompt N)*` or
`*(blocking: stage 2 prompt 5)*`, or `*(not blocking)*`. (The first example is
deliberately not a literal prompt number — the checker greps this file, and a
documentation example must not block a real prompt.) `make check` refuses to let a
blocked prompt become the next one. Downgrading blocking → not blocking is a decision,
and gets a decision entry.

- **Read marker reconciliation across attached clients.** *(not blocking)* Last-write-
  wins on timestamp is probably fine; also decide whether to propagate upstream via
  IRCv3 `draft/read-marker` (NORTH-STAR §9). Prompt 9b ships the one marker the single
  nullable column can represent — machine-wide, broadcast, last-write-wins with no
  clamp — which is deliberately *not* an answer to which of two clients wins; that
  needs a schema and a wire that can hold two markers. Settle before stage 6 item 1
  (multi-client attach).
- **Retention policy.** *(not blocking)* Default "never delete", but a vacuum/archive
  story is owed (NORTH-STAR §9). Nothing in the schema forecloses it. Settle by stage 6
  item 3 (release), where it becomes a documented user promise.
- **License.** *(not blocking)* The public repo currently ships no license, and the
  Cargo manifests deliberately omit the `license` field — prompt 1's review caught the
  field landing as a silent decision and it was stripped. Genuinely the user's call
  (MIT OR Apache-2.0 is the Rust-ecosystem default). Must be settled by stage 6 item 3
  (Release): the field plus LICENSE texts land together.

### Testing strategy

- Unit: table-driven transcript tests for the connection state machine — inline
  `Step` tables (server line → expected client lines) in
  `crates/havoc-core/tests/state_machine.rs`, covering cap negotiation, SASL,
  registration, `CAP NEW`/`CAP DEL` (§6.8). Hand-written first; live captures are
  converted into Step rows by `scripts/trace-to-steps.sh` and appended to the same
  corpus (one format, deliberately — amended at prompt 6 from the original
  "fixture files" wording). Storage tested against real
  temp-file SQLite — never mocked; it is embedded and fast.
- Integration: the connection actor runs against a scripted fake implementing the
  line-transport trait; the whole core is driven through the typed `havoc-ipc` boundary
  exactly as the TUI will drive it.
- Property tests: the wrapped-line cache under random resize sequences (stage 2, §6.7).
- **A scripted live run per prompt.** A local `ergo` IRC server — single static binary,
  trivially scriptable: `scripts/live-run.sh` (born in prompt 5) generates a config in
  a temp dir, boots ergo on a random port, and drives the debug CLI against it, with a
  second debug-CLI instance as the second party. Stage 1 is headless, so live runs mean
  driving the debug CLI. Occasional Libera.Chat spot-checks (prompt 6, and at stage
  boundaries) stay out of the per-prompt loop. A prompt with nothing observable records
  an honest N/A with the reason — the requirement is honesty, not ritual.

### Carry-forward notes

Items below may carry a `### Carry-forward` block, appended when earlier work turns up
something that item needs to know. Consume and delete it when the item is built, and
record in `BUILD-LOG.md` that you did. This is the same convention the prompt files use,
extended to stages that have no prompt file yet.

---

## Stage 1 — Headless core (connect, log, search; no TUI)

Target: the core connects to a real network over TLS with SASL, logs everything it sees
into SQLite, and answers search, backlog, and read-marker requests — all driven through
the typed boundary by a debug CLI. This merges NORTH-STAR §8 M1+M2: a core that
connects but stores nothing is not usable, while a headless logger with instant search
genuinely is.

1. **Workspace scaffold and build discipline.** Cargo workspace; five crates under
   `crates/` with minimal `lib.rs` each and the §4.2 dependency edges declared;
   Makefile `build`/`test`/`fmt`/`lint` placeholder bodies replaced with real cargo
   commands, warnings-as-errors; `/target` ignored; CI's build job un-skipped.
2. **IPC wire types.** `havoc-ipc`'s stage-1 surface: newtype IDs, Request/Response/
   Event enums, `Anchor`, `MessageKind`, buffer/network models, protocol version
   constant. `ServerTime` deliberately exposes no ordering (§6.1). CBOR round-trip
   tests prove the evolution story early.
3. **Storage schema and migrations.** `rusqlite`, WAL on, the §4.9 schema; versioned
   migrations run at startup from this first commit, including the initial one (§6.10);
   storage on a dedicated thread behind a channel (§6.6). No FTS yet — that arrives as
   a migration, which is the point of having migrations first.
4. **Connection state machine, offline.** The per-network actor (`HashMap<NetworkId,_>`
   from commit one, §6.9): cap negotiation, SASL, registration, ISUPPORT, autojoin as
   an explicit state machine over a line-transport trait, table-driven-tested against
   transcripts (§6.8). `irc-proto` for parse/serialize only (§5.2). No real I/O.
5. **Event bus, request handler, and debug CLI.** Core event broadcast and request
   dispatch with correlation ids; `havoc-transport`'s trait plus the in-process mpsc
   impl; a debug CLI driving the core over the same typed messages the TUI will use
   (§4.3). First live run, against a local ergo.
6. **Live connection, TLS, and reconnect.** `rustls`, real DNS, live SASL against ergo
   and a Libera.Chat spot-check; disconnect detection and jittered backoff through the
   state machine's seam; live transcripts captured into item 4's corpus. TLS is the
   default; plaintext is a loud explicit opt-in (§2.3).
7. **Message ingestion, identity, and batched writes.** Actor events flow into storage:
   `seq` assigned at insert, `msgid` dedup enforced by the unique index at the storage
   layer, content-hash fallback for tagless servers, batched transactions on a ~100ms
   timer (§4.6, §6.4, §6.5).
8. **Full-text search.** A self-contained FTS5 table added by migration and
   trigger-synced (external-content cannot bind to the WITHOUT ROWID message
   table — amended at prompt 8); `Search` with structural filters (`from:`,
   `in:`, time range) per §7.1; results delivered as correlated events; a CLI
   `search` command against a seeded corpus.
9. **Windowed backlog and read markers.** `FetchBacklog` with all four anchors
   including `AroundSearchHit`, limit capped server-side regardless of the request
   (§4.7, §6.3); `last_read_seq` set/read per buffer. Still no "give me the buffer"
   — and, added at prompt 9a, the other half of §4.5's attach contract: the core
   *announces* the buffer set to each attaching client (a replay of
   `BufferCreated` on that session's lane), so a client over a data dir an
   earlier process wrote can resolve buffers no event of its own introduced.
   Prompt 9b's marker is machine-wide and broadcast (one nullable column), and it
   may move backward — the client is the authority on where a person has read to.
10. **Network config and credentials.** TOML config for networks/nick/autojoin as seed
    data; `keyring` with encrypted-file fallback for SASL secrets — never plaintext in
    the config file (§5.8). Ends with the stage acceptance run driven from config
    alone.

Stage 1 is broken into 12 prompts in **`STAGE-1-PROMPTS.md`**, which is authoritative
for grouping, ordering, and status; each numbered item above is attached to exactly one
prompt there. The prompt details are deliberately not duplicated here — two copies of
one list drift, and the copy nobody edits is the one that gets read.

**Done when:** a person with a config file runs the debug CLI, connects to Libera.Chat
over TLS with SASL, joins a channel, watches traffic land in the database, searches all
history instantly with filters, and can kill and restart the process losing nothing.

---

## Stage 2 — TUI (a daily-usable client)

Covers NORTH-STAR §8 M3.

1. **Embedded-mode wiring and event loop.** `supernaut` runs core + TUI in one process over
   the in-process transport; render loop rebuilds each tick from a local projection of
   core events (§4.10); input becomes typed requests. No shortcut embedded mode can
   take that attached mode could not (§4.3).

   ### Carry-forward
   - From stage 1 prompt 10a: **autoconnect lands here, and it re-opens prompt
     9b's early-quit hazard.** 10a deliberately built none — `connect` is still an
     explicit verb in `crates/supernaut/src/session.rs`, because session D's proof
     is a process that dials nothing while holding a config that names the network
     it would dial. When startup issues a `Connect` nobody typed, `finish()` in
     `crates/supernaut/src/session_wait.rs` blocks on `outstanding` and exits
     non-zero on timeout, so any path that quits early can now fail where it used
     to succeed silently. Reach a quiescent point before quitting, or pass a
     deadline. Config autojoin itself is safe: it issues no `Request` at all.
   - From stage 1 prompt 5: **the event stream is duplicated and cross-lane
     unordered.** The actor emits ConnectionState on every internal state change
     (repeated `phase=connecting`), and `wiring.rs` merges directed and broadcast
     lanes via select! with no ordering between an Ack and the event it caused.
     The TUI's projection must dedupe phase transitions and never assume
     response-before-event.
   - From stage 1 prompt 9b: **`around-hit`, and anything shaped like it, depends
     on search's Response and its correlated Event riding the same directed lane in
     order.** `wait search` counts *responses* while `last_hits` is filled by the
     *event*; the invariant holds only because both travel one lane. If
     `SearchResults` ever moves lanes, every response-counted wait races its own
     data — live-run.sh carries the same warning above its `around-hit` line.
   - From stage 1 prompt 9b: **a `SetReadMarker` `Ack` and its
     `ReadMarkerChanged` are unordered relative to each other** — the Ack rides
     the directed lane, the event the broadcast lane, and the live run has been
     observed printing the event *first*. The projection must not treat the Ack as
     the marker's arrival, nor the event as confirmation of its own request.
   - From stage 1 prompt 9a: **`BufferCreated` can arrive AFTER a `Backlog`
     response naming the same buffer.** Announcements go out on a spawned task
     (`announce` in `crates/havoc-core/src/core/reads.rs`) while responses go out
     inline, so the projection must tolerate a *Response* — not just an Event —
     naming a `BufferId` it has never been told about. The note above covers the
     opposite ordering; this is the one the attach path introduces.
2. **Wrapped-line cache.** The largest single piece of original UI work (§4.10): keyed
   on (buffer, width), invalidated on resize, pre-rendered `Line` window around the
   viewport. Its own module, property-tested over random resize sequences (§6.7).
3. **Scrollback viewport and message rendering.** Scrollback over the cache and the
   windowed backlog API; dense, loud, differentiated-by-kind formatting (§2.1).

   ### Carry-forward
   - From stage 1 prompt 2: **the time-crate debt lands here.** The havoc-ipc
     dependency decision (BUILD-LOG.md, 2026-08-09) deferred `time` until something
     formats timestamps for display; `ServerTime::as_unix_millis` in
     `crates/havoc-ipc/src/lib.rs` is the sole accessor. Adding `time` to
     supernaut-tui needs its own decision entry, and must not grow comparison
     helpers around `ServerTime` — its no-ordering rule is deliberate.
4. **Buffer list, activity, and switching.** Buffer list with activity/highlight state
   from core; switching; unread positioning from read markers.
5. **Input widget and command line.** Composer (client-authoritative while typing,
   §4.5), command parsing (`/join`, `/msg`, `/search`, …), input history from core,
   nick completion.

   ### Carry-forward
   - From stage 1 prompt 9b: **`SendText`/`Join` are documented at-most-once, so
     the composer must clear pending-send state on the *echo*, not the `Ack`.**
     A command issued while the network is reconnecting is dropped in
     `crates/havoc-core/src/connection/actor.rs` (loudly on stderr, with no
     outcome on the wire — a per-request delivery outcome is stage 4's). Our own
     PRIVMSG returns as a `MessageAdded` because `echo-message` is requested, and
     that is the only confirmation that exists; on a server without it, "sent" is
     unconfirmable by anyone.
   - From stage 1 prompt 10a: **`in:` is a buffer-*name* filter with no network
     scope, and `/search` owns the grammar that would give it one.** Prompt 10a
     killed the accretion *cause* (stable config names mean one `network` row per
     network, not one per `debug-<host>` string) but left the union: two networks
     with a `#rust` each are one `in:#rust` result set. Scoping is a grammar
     question — `in:net/#chan` versus a separate `network:` filter — and this is
     the only consumer that can render which network a hit came from; the debug
     CLI prints `buffer=<id>`, so a scope filter is unobservable there. The
     current behaviour is documented on `SearchSpec.buffer`
     (`crates/havoc-core/src/search.rs`) and `run_search`
     (`crates/havoc-core/src/storage/query.rs`), and **pinned** by
     `in_filter_unions_one_buffer_name_across_networks` in
     `crates/havoc-core/src/storage/tests.rs` — so changing it here is a
     documented behaviour change with a failing test, not a discovered assumption.
   - From stage 1 prompt 8: **bare hyphenated search terms are FTS5 column-
     filter syntax** (`xyzzy-quicksilver` → "no such column") — the error
     returns cleanly, but the `/search` UX here should quote bare terms
     containing FTS5 operator characters before they reach the wire.
   - From stage 1 prompt 7: **QUIT and NICK are not ingested — they fan out to
     every shared channel, which needs the membership state nick completion
     builds here.** The classifier in
     `crates/havoc-core/src/connection/ingest.rs` skips them deliberately;
     when membership tracking lands, extend the classifier (and decide whether
     to backfill or accept the gap in history).
6. **Theme file and nick coloring.** Semantic slots, truecolor, data-file themes; ship
   two or three, one unapologetically loud (§5.8). Nick coloring on.
7. **First-run experience.** Pick a network, type a nick, you are on IRC with TLS,
   SASL, sane colors, and working search (§3.1). The default configuration is the
   product (§2.1).

   ### Carry-forward
   - From stage 1 prompt 10a: **first-run may not inherit "config is mandatory",
     and may not answer it by writing the file.** 10a made
     `$XDG_CONFIG_HOME/supernaut/config.toml` mandatory for the `session`
     subcommand only; the no-subcommand path (`crates/supernaut/src/main.rs`'s
     `open_store_and_report`) still runs with no config file anywhere, and that is
     the whole of §3.1's works-before-configuration property stage 1 can honestly
     claim. The product's answer is *this item*, and it can be neither a flags
     fallback (the six connection flags were deleted, deliberately, so config is
     not the decorative path) nor a silent config write — the 2026-08-10
     config-vs-runtime-state decision forbids the program ever writing that file,
     and havoc-core is compiled without a TOML serializer, so it cannot. An
     explicit, user-confirmed "save this to config" action is the shape that is
     allowed, and it needs the `display` feature back plus its own decision entry.
   - From stage 1 prompt 10a: **the loopback-only plaintext rule is stricter than
     NORTH-STAR §2.3 asks, and relaxing it is this item's call.** §2.3 demands a
     loud opt-in; `NetworkEntry::plaintext` in
     `crates/havoc-core/src/config.rs` additionally refuses any non-loopback host.
     It was kept at 10a because it is the debug-grade rule stage 1 shipped — but a
     user with a plaintext LAN bouncer first appears here.

**Done when:** you can spend a full day in it as your only client on one network and
never reach for irssi.

---

## Stage 3 — Dogfood month

Covers NORTH-STAR §8 M4. Not optional, and nothing new ships during it (§6.11).

1. **Dogfood fixes.** Use it as the only IRC client for one month; fix what daily use
   surfaces, as many small prompts as needed. Fixes only — the queue for this stage is
   grown, not planned.
2. **State-boundary audit.** Record in `BUILD-LOG.md` whether the §4.5 ownership table
   held under real use. Any ambiguity found is a P0 design bug to resolve *before* the
   daemon depends on the boundary.

**Done when:** the month has passed, its findings are fixed or explicitly deferred into
this plan, and the boundary audit is recorded.

---

## Stage 4 — Daemon and attach

Covers NORTH-STAR §8 M5. The stage-level budget is itself the acceptance test: if this
is not a few hundred lines, stop and reexamine stage 1 (§5.4).

1. **UDS transport and CBOR framing.** `LengthDelimitedCodec` + `ciborium` over a Unix
   socket at `$XDG_RUNTIME_DIR/supernaut/core.sock`, filesystem permissions as the auth
   model (§4.8); requests and events multiplexed on one connection.

   ### Carry-forward
   - From stage 1 prompt 5: **the two-lane merge and the Lagged signal are
     binary-local and have no wire story.** The merge of
     `Session { directed, broadcast }` and the `Lagged(n)` conversion live in
     `crates/supernaut/src/wiring.rs`; `Incoming`/`TransportError` are
     transport-local by design. The UDS server must perform the same merge
     core-side and give lag/close a frame representation — otherwise the
     loud-lag guarantee silently fails to cross the socket.
   - From stage 1 prompts 9a and 9b: **no response has an exactly-one-Response
     guarantee once a lane fills, and nothing on the wire says so.** Every
     directed message now rides `Bus::direct`
     (`crates/havoc-core/src/bus.rs`), which removes the whole session at
     `DIRECTED_LANE_CAPACITY`, recorded only by an engine-stderr `eprintln`.
     Embedded mode masks this behind `wiring.rs`'s 256-deep pump; a socket writer
     will not. The frame protocol needs either a dropped-message signal or a
     documented at-most-once rule for responses — decided alongside the
     Lagged/Closed frame representation the note above already owes.
   - From stage 1 prompt 9b: **the attach replay is now inline and unbuffered — a
     client with >4096 buffers is killed at attach rather than served slowly.**
     `announce` in `crates/havoc-core/src/core/reads.rs` loops `bus.direct` with no
     backpressure path, and the replay is the one burst whose size the *engine*
     chooses — so it is now the burst most likely to trip
     `DIRECTED_LANE_CAPACITY`. A per-session writer task must handle the replay
     first, before ordinary traffic.
   - From stage 1 prompt 9b: **`Full` and `Closed` are indistinguishable at the
     client — both look like a closed transport.** The dropped-for-not-reading
     reason lives only in engine stderr naming the `ClientId`. The frame protocol
     needs the distinction if a client is ever to log "you were dropped" rather
     than "the server went away" (extends the dropped-message note above).
   - From stage 1 prompt 9b: **the core loop can still park on the storage thread,
     and `reads_tx` now has three producers.** `connect()` in
     `crates/havoc-core/src/core.rs` awaits a `spawn_blocking(ensure_network)`
     round trip to the thread whose bounded (64) reply lanes only the core loop
     drains, and `SetReadMarker`/`Backlog`/`ListBuffers` all answer on `reads_tx`.
     Untrusted pipelining clients make a deep `reads_tx` reachable; decide with the
     socket server whether `connect` stops being a blocking round trip.
   - From stage 1 prompt 9b: **`DIRECTED_LANE_CAPACITY` counts messages, not
     bytes, and one message can be a 200-row window.** 4096 was chosen to sit far
     above any legitimate attach replay and far below hurting a laptop; byte-based
     accounting (and, if bytes are the bound, a per-session writer task with an
     overflow deque instead of a bounded channel) belongs here, where the socket
     makes clients plural and untrusted.
   - From stage 1 prompt 9b: **"sent" has no berth, so `SendText`/`Join` are
     at-most-once.** A command dropped while disconnected cannot be reported: the
     `Ack` has already gone out and the correlation with it, and a per-request
     delivery outcome is a *variant* addition — a real v1 break. This handshake is
     where variant additions become negotiable, so it owes the berth.
   - From stage 1 prompt 9a: **the announcement's unconfigured-network skip is
     advisory, not a boundary.** `FetchBacklog` in
     `crates/havoc-core/src/core.rs` is not gated on `state.buffers`, so a client
     can fetch (and enumerate by probing ids, distinguishing `[]` from "unknown
     buffer N") history of buffers the announcement deliberately withheld.
     Harmless under single-user filesystem auth; decide whether the skip rule
     becomes a real check once the socket makes clients plural.
   - From stage 1 prompt 10a: **`Search` is the second hole in that same skip, and
     it needs no id probing.** `run_search` in
     `crates/havoc-core/src/storage/query.rs` has no network filter at all, so a
     hit can come back carrying a `BufferId` on a network the client's config does
     not name — history the attach announcement deliberately withheld (10a made
     that skip loud: `orphan network <name>: N buffers not announced`). Same
     advisory-not-a-boundary decision as the note above, and it wants the same
     answer at the same time.
2. **Capability handshake.** Feature lists exchanged, intersection operated on; no
   version lockstep (§4.8). The constants live in `havoc-ipc`.

   ### Carry-forward
   - From stage 1 prompt 2: **serde tolerance covers unknown struct fields only —
     unknown enum variants are decode errors.** `unknown_struct_fields_are_tolerated`
     in `crates/havoc-ipc/tests/roundtrip.rs` is the entire proven evolution story;
     adding a variant to `Event`/`RequestBody` breaks any older peer at decode. The
     handshake must gate *variants*, not just features, behind `havoc_ipc::caps`
     constants, and never send a variant outside the negotiated intersection.
   - From stage 1 prompt 9a: **`BACKLOG_MAX_LIMIT` is deliberately
     undiscoverable.** `crates/havoc-core/src/storage/query.rs` keeps the cap
     core-side on the reasoning that discovery is this handshake's job, and there
     is no has-more berth — so a client cannot distinguish "capped" from "that is
     all". The handshake owes the cap as a negotiated *value*, not just feature
     flags.
3. **Daemon and attach modes.** `supernaut --daemon` / `supernaut --connect <path>`
   (whether the daemon additionally brands as `havocd` is decided here); socket
   lifecycle and orphan cleanup; detach loses nothing, attach renders from buffers +
   read markers alone (§4.5).

**Done when:** kill the TUI while the daemon holds the connection, reattach, and the
buffer looks right.

---

## Stage 5 — Multi-network, resync, palette, notifications

Covers NORTH-STAR §8 M6.

1. **Multi-network.** Config and UI for 2–5 networks; the actor map has existed since
   stage 1, so this is surface, not surgery (§6.9).

   ### Carry-forward
   - From stage 1 prompt 10a: **three config fields were left out because a
     one-network stage cannot observe them, and this item is where each becomes
     real.** In `crates/havoc-core/src/config.rs`: (1) a **per-network nick**
     override — `nick` is top-level and global today, and `username`/`realname`
     are derived from it in `into_networks`; (2) **`server_name`**, for an
     SNI/connect-host split — `Security::Tls.server_name` is currently always the
     dialed `host`, which a bouncer or a round-robin front end breaks; (3) an
     explicit **`id` key**, but *only* if a wire `NetworkId` ever gets persisted or
     cached across restarts. Ids are assigned by the loader today — networks
     sorted by name, `NetworkId(1..N)` — and renumbering is unobservable precisely
     because nothing persists them (`ensure_network` keys the `network` table on
     the name). Persisting one turns renumbering into data corruption and makes
     the id a config field, with a uniqueness rule and a migration.
2. **`CHATHISTORY` resync.** Reconnect fetches only what was missed, merges by `msgid`,
   renders in order with original timestamps (§7.2). Tested against a bouncer/ergo
   replaying (§6.4) — the laptop-lid test, with zero duplicates.

   ### Carry-forward
   - From stage 1 prompt 6: **the actor keeps no memory across attempts — resync
     must be core-driven off the second Registered.** The attempt loop in
     `crates/havoc-core/src/connection/actor.rs` discards the Machine per
     attempt and commands are dropped while disconnected, so resync fired at
     the Disconnected edge dies silently; key it off the Registered transition
     in core, tolerant of the command-drop window.
3. **Notifications.** `notify-rust` on highlight and PM, rate-limited, suppressed when
   focused; OSC 9/777 fallback over SSH (§7.6).
4. **Command palette and fuzzy everything.** `nucleo` across commands, buffers, nicks,
   recent URLs (§7.3).

**Done when:** daily driver on multiple networks; closing the laptop stops being an
event.

---

## Stage 6 — Polish & release

NORTH-STAR §8 M7+ is a menu, not a commitment (§7); this stage picks from it and ships.

1. **Multi-client attach and read-marker reconciliation.** Settle the Still-open
   reconciliation question first, then N attached clients.

   ### Carry-forward
   - From stage 1 prompt 3: **`last_read_seq` is one nullable column on `buffer` —
     single-marker-per-buffer is now disk shape**
     (`crates/havoc-core/migrations/0001_init.sql`). Per-client markers owe a
     migration to a per-client table; make the reconciliation decision knowing the
     current shape can only represent the merged result, never per-client inputs.
   - From stage 1 prompt 9b: **`Event::ReadMarkerChanged` is broadcast to every
     attached client today, and that is not a leak — it is the honest shape of one
     nullable column.** Making markers per-client changes the *event's* audience
     as well as the schema: it becomes directed, which means the variant's
     documented meaning ("core-owned state, one marker per buffer for the whole
     machine") changes with it. A wire that can carry a client-scoped marker is a
     variant or field addition, so it is gated on stage 4's handshake.
   - From stage 1 prompt 9a: **the read path already hands `last_read_seq` to
     every attaching client, one value per buffer.** `run_list_buffers`
     (`crates/havoc-core/src/storage/query.rs`) selects it into `BufferRow` and
     `announce` copies it into each client's `BufferInfo`. A per-client marker
     table changes this *read*, not only the write: the announcement becomes
     per-client, and `run_list_buffers` needs the `ClientId` it currently ignores.
2. **From the §7 menu, as separate items when chosen.** Candidates: scripting host
   (`mlua`, §5.7), plain-IRC listener (§7.9), inline images (§7.5), OSC 8/52 niceties
   (§7.4), stats (§7.7), live theme reload (§7.8), SASL EXTERNAL / CertFP (§2.3's
   "CertFP supported" promise — needs client-cert plumbing in rustls + keyring).

   ### Carry-forward
   - From stage 1 prompt 4: **the SASL "mechanism slot" is shape only —
     `begin_sasl` in `crates/havoc-core/src/connection/caps.rs` consults
     `mechanisms.first()` and failure is terminal.** EXTERNAL does not "drop in":
     it requires mechanism iteration, per-mechanism fallback semantics, and
     RPL_SASLMECHS (908) handling. Budget that machine work into the item; do not
     plan against the slot working as-is. Each choice gets a decision entry
   and its own numbered item here before any prompt exists.
3. **Release.** Banner and personality (§2.1); packaging, docs, distribution; the
   retention-policy answer documented as a user promise. (The name is settled —
   Supernaut, with havoc as the headless engine; NORTH-STAR amendment 2026-08-09 —
   so no rename pass is owed here.)

---

## Suggested order of attack

Types before storage before protocol keeps every later session coding against a stable
vocabulary. The protocol state machine is built offline first because protocol bugs and
I/O bugs are different classes and the transcripts are the durable asset. The debug CLI
lands mid-stage-1 so every subsequent prompt has a live-run harness; config lands last
in the stage because a config surface designed before its features exist calcifies
flags nobody wants. The TUI waits until the core is provable headless, the daemon waits
until the dogfood month has tested the state boundary (§6.11), and `CHATHISTORY` resync
waits for stage 5 even though the actor's reconnect seam exists from stage 1 — there is
nothing real to resync against until multi-network life makes reconnects routine.
