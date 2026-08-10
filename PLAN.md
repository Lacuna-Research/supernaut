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

The blocking annotation is machine-read: `*(blocking: prompt 5)*` or
`*(blocking: stage 2 prompt 5)*`, or `*(not blocking)*`. `make check` refuses to let a
blocked prompt become the next one. Downgrading blocking → not blocking is a decision,
and gets a decision entry.

- **Buffer identity across networks — is `#rust` on two networks one buffer or two?**
  *(blocking: prompt 3)* NORTH-STAR §9 leans "two", and the §4.9 sketch
  (`UNIQUE(network_id, name)`) already implies it, but merged-view is a foreseeable
  request and the schema hardens at prompt 3. Decide, and record whether a future
  merged view is a client/query-side projection — if it is, the schema stays untouched
  and this never reopens.
- **Migration mechanism: `refinery` or hand-rolled versioned SQL?** *(blocking:
  prompt 3)* NORTH-STAR §4.9 explicitly leaves this open. `refinery` is a dependency
  (allowlist + decision entry); hand-rolled is a hundred lines we own forever. Must be
  settled where migrations are born.
- **SASL mechanism set for stage 1.** *(blocking: prompt 4)* §2.3 promises SASL as the
  standard path *and* CertFP supported; §8 M1 says only "SASL". PLAIN-over-TLS only,
  or PLAIN + EXTERNAL from the start? The mechanism-selection shape in the state
  machine is easier to design with two mechanisms in view than to retrofit around one.
- **Config vs. runtime state.** *(blocking: prompt 10)* Where does "I joined this
  channel manually" live — config file or database? (NORTH-STAR §9: leaning database,
  config as seed only.) Prompt 10 ships the config file, so it settles here. Until
  then, earlier prompts must not persist join state anywhere the answer would have to
  migrate; if one does, name the symbol on this item.
- **Read marker reconciliation across attached clients.** *(not blocking)* Last-write-
  wins on timestamp is probably fine; also decide whether to propagate upstream via
  IRCv3 `draft/read-marker` (NORTH-STAR §9). Single-client markers ship in prompt 9
  without choosing; this must be settled before stage 6 item 1 (multi-client attach).
- **Retention policy.** *(not blocking)* Default "never delete", but a vacuum/archive
  story is owed (NORTH-STAR §9). Nothing in the schema forecloses it. Settle by stage 6
  item 3 (release), where it becomes a documented user promise.

### Testing strategy

- Unit: table-driven transcript tests for the connection state machine — fixture files
  of server lines → expected client lines and state, covering cap negotiation, SASL,
  registration, `CAP NEW`/`CAP DEL` (§6.8). Hand-written first; live-captured
  transcripts are appended to the corpus from prompt 6 on. Storage tested against real
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
8. **Full-text search.** FTS5 external-content table added by migration and kept in
   sync; `Search` with structural filters (`from:`, `in:`, time range) per §7.1;
   results delivered as events; a CLI `search` command against a seeded corpus.
9. **Windowed backlog and read markers.** `FetchBacklog` with all four anchors
   including `AroundSearchHit`, limit capped server-side regardless of the request
   (§4.7, §6.3); `last_read_seq` set/read per buffer. Still no "give me the buffer".
10. **Network config and credentials.** TOML config for networks/nick/autojoin as seed
    data; `keyring` with encrypted-file fallback for SASL secrets — never plaintext in
    the config file (§5.8). Ends with the stage acceptance run driven from config
    alone.

Stage 1 is broken into 10 prompts in **`STAGE-1-PROMPTS.md`**, which is authoritative
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
2. **Wrapped-line cache.** The largest single piece of original UI work (§4.10): keyed
   on (buffer, width), invalidated on resize, pre-rendered `Line` window around the
   viewport. Its own module, property-tested over random resize sequences (§6.7).
3. **Scrollback viewport and message rendering.** Scrollback over the cache and the
   windowed backlog API; dense, loud, differentiated-by-kind formatting (§2.1).
4. **Buffer list, activity, and switching.** Buffer list with activity/highlight state
   from core; switching; unread positioning from read markers.
5. **Input widget and command line.** Composer (client-authoritative while typing,
   §4.5), command parsing (`/join`, `/msg`, `/search`, …), input history from core,
   nick completion.
6. **Theme file and nick coloring.** Semantic slots, truecolor, data-file themes; ship
   two or three, one unapologetically loud (§5.8). Nick coloring on.
7. **First-run experience.** Pick a network, type a nick, you are on IRC with TLS,
   SASL, sane colors, and working search (§3.1). The default configuration is the
   product (§2.1).

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
2. **Capability handshake.** Feature lists exchanged, intersection operated on; no
   version lockstep (§4.8). The constants live in `havoc-ipc`.
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
2. **`CHATHISTORY` resync.** Reconnect fetches only what was missed, merges by `msgid`,
   renders in order with original timestamps (§7.2). Tested against a bouncer/ergo
   replaying (§6.4) — the laptop-lid test, with zero duplicates.
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
2. **From the §7 menu, as separate items when chosen.** Candidates: scripting host
   (`mlua`, §5.7), plain-IRC listener (§7.9), inline images (§7.5), OSC 8/52 niceties
   (§7.4), stats (§7.7), live theme reload (§7.8). Each choice gets a decision entry
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
