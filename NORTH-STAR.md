# HAVOC — North Star

> **Working codename.** `havoc` (TUI) / `havocd` (core daemon). Name is a placeholder; the ethos below is not.

**Status:** Design baseline, v0. Written before first commit.
**Purpose:** This document is the arbiter of scope disputes and architectural drift. If a proposed change contradicts something here, either the change is wrong or this document needs an explicit, dated amendment. Nothing gets silently reinterpreted.

---

## 1. What we are building

A terminal IRC client for people who want the client to already be good, not to become good after three weekends of configuration.

Concretely: a headless core that owns connections, history, and state, plus a Ratatui frontend that owns nothing but the viewport. It runs as a single process by default and can be split into a daemon plus one or more attached clients without a rewrite. It speaks modern IRC — TLS, SASL, IRCv3 — as the normal path rather than as a compatibility afterthought. It stores every line you have ever seen in SQLite and makes it searchable in milliseconds.

---

## 2. Ethos

### 2.1 The inheritance

BitchX beat irssi for reasons that are still true and still unaddressed by anything in the terminal:

**Batteries included, not batteries available.** irssi shipped a bare client and a scripting API. BitchX shipped flood protection, nick coloring, `/scan`, notify lists, and ban management already turned on. The user typed the binary name and had a working, opinionated client. We inherit this directly: **the default configuration is the product.** A feature that requires config-file editing to discover has failed.

**Opinionated defaults beat infinite configurability.** Every "make it a setting" decision is a decision deferred onto the user. We will make choices, ship them on, and let people turn them off. When we cannot decide, that is a signal the feature is not ready — not a signal to add a toggle.

**Loud, legible output.** BitchX's dense colored formats told you at a glance what kind of event you were looking at. irssi's default theme was gray mush. Information density and visual differentiation are features, not noise.

**Personality.** ASCII art, quit reasons, a startup banner, a voice. Software made by people, for people, with a point of view. This is not decoration — it is what made the thing memorable enough to have this conversation about thirty years later.

### 2.2 What we deliberately drop

The 1998 culture around the client is not the client. We drop the war scripts, CDCC/fserve, bot-linking, and DCC-as-primary-file-transfer. DCC is a NAT nightmare and a security liability; the modern equivalent is upload-and-paste-a-link. Flood *protection* stays. Flood *tooling* goes.

We also drop the assumption that the client is the persistence layer. `screen` + a 24/7 shell was the answer in 1998. It is not the answer now.

### 2.3 What "modern and progressive" means here

Not "has rounded corners." It means:

- **Encrypted and authenticated by default.** TLS via rustls, SASL as the standard login path, CertFP supported. Plaintext requires an explicit, loud opt-in.
- **The network is unreliable and that is normal.** Reconnect-and-resync is the designed-for case, not the exception. History merges cleanly. Nobody sees a wall of duplicated lines after a laptop lid closes.
- **History is a database, not a log file.** Full-text search across every network, every buffer, all time, instantly. This is the single most useful thing no C-era client has.
- **The terminal is capable now.** Truecolor, mouse, OSC 8 hyperlinks, OSC 52 clipboard, Kitty keyboard protocol, inline images. Use it.
- **Bouncers exist.** Interoperate with soju/ZNC properly rather than pretending the world is a single long-lived TCP connection.

### 2.4 Non-goals

Explicitly out of scope, permanently or until stated otherwise:

- BitchX script (`.bx`) compatibility. We are not reimplementing a 21k-line bespoke interpreter.
- Being a general-purpose chat client (Matrix, Discord, Slack bridges). IRC only. Bridges are somebody else's daemon.
- A GUI in v1. The architecture permits one later; we are not building it.
- Protocol-level encryption schemes (OTR, FiSH). Historically interesting, cryptographically dubious, enormous surface area.
- Serving as an IRC daemon (ircd). We may *speak* IRC to third-party clients (§7.9) but we do not implement a network.

### 2.5 Target user

Someone who has used irssi or WeeChat for a decade, is on 2–5 networks, keeps a bouncer or wants to stop needing one, lives in a terminal, and is quietly annoyed that searching their own chat history is harder than searching a stranger's tweets.

---

## 3. Product pillars

These are the things that must be true for the project to have been worth doing.

1. **Works well before configuration.** First run: pick a network, type a nick, you are on IRC with sane colors, working search, and TLS.
2. **History is a first-class database.** Instant full-text search with filters. Backlog survives everything.
3. **Reconnect is invisible.** Close the laptop, open it, the buffer looks right. No duplicates, no gaps, no manual `/lastlog` archaeology.
4. **Modern protocol support is not optional.** IRCv3 caps negotiated aggressively; `server-time`, `message-tags`, `chathistory`, `echo-message`, `batch`, `labeled-response` all supported.
5. **The core is headless and the boundary is real.** Enforced by the compiler, not by discipline.
6. **It has a face.** Distinctive, dense, colored, opinionated output that looks like nothing else in the terminal.

---

## 4. Architecture overview

### 4.1 Shape

```
                    ┌─────────────────────────────────┐
                    │            havoc-core           │
                    │                                 │
   IRC networks ◄──►│  net actor (per network)        │
   (TLS/rustls)     │      │                          │
                    │      ▼                          │
                    │  event bus ──► storage (SQLite) │
                    │      │                          │
                    │      ▼                          │
                    │  session/request handler        │
                    └──────────┬──────────────────────┘
                               │  havoc-ipc  (typed messages, serializable)
                    ┌──────────┴──────────┐
                    │                     │
              in-process mpsc      Unix domain socket
                    │                     │
             ┌──────▼──────┐       ┌──────▼──────┐
             │  havoc-tui  │       │  havoc-tui  │
             │ (embedded)  │       │  (attached) │
             └─────────────┘       └─────────────┘
```

### 4.2 Crate layout

The dependency graph *is* the architecture. This is not a convention we agree to follow; it is a constraint the compiler enforces.

| Crate | Owns | Must NOT depend on |
|---|---|---|
| `havoc-ipc` | Wire types: requests, responses, events, IDs, buffer/network models. Serde derives. Protocol version + capability constants. | Everything. Near-zero deps: `serde`, `time`, maybe `bitflags`. |
| `havoc-core` | Connection actors, IRCv3 cap negotiation, SQLite storage, event bus, request handling, config, scripting host. | `ratatui`, `crossterm`, anything terminal. |
| `havoc-tui` | Rendering, input handling, layout, theme, viewport state. | `rusqlite`, `tokio::net`, `rustls`, `irc-proto`. |
| `havoc-transport` | Framing and transport impls (in-process, UDS, optional TLS/TCP). Trait both sides code against. | Business logic of any kind. |
| `havoc` | Binary. Arg parsing, wiring, mode selection. | — |

If `havoc-tui` ever needs a type that lives in `havoc-core`, that type belongs in `havoc-ipc` instead. That rule alone prevents most architectural decay.

### 4.3 Process modes

One binary, three modes, one code path:

```
havoc                    # embedded: core + TUI in one process, mpsc transport
havoc --daemon           # core only, listening on UDS
havoc --connect <path>   # TUI only, attaches to a running core
```

Embedded mode is not a special case or a "simple mode" — it is the same core and the same TUI communicating over the same typed messages, with `tokio::mpsc` substituted for a socket. If embedded mode can take a shortcut that attached mode cannot, we have introduced the bug we designed this to prevent.

### 4.4 Connection actor

One task per network. It owns its socket, its connection state machine, and its view of network state (channels joined, nick, ISUPPORT tokens, negotiated caps). It communicates exclusively by channels — no shared mutable state with anything.

Its state machine covers: DNS → TCP → TLS → `CAP LS` → SASL → `CAP END` → registration → ISUPPORT parsing → autojoin → steady state → (disconnect) → backoff → resync via `CHATHISTORY`.

This is written with one network configured, but the actor is instantiated from a `HashMap<NetworkId, _>` from commit one. Multi-network is the single most commonly bolted-on feature in this category of software, and bolting it on means threading a network ID through every function already written.

### 4.5 State ownership contract

**The core owns everything semantic. The client owns only the viewport.**

| Core | Client |
|---|---|
| Connections, network state, ISUPPORT | Focused buffer |
| Buffers and their membership | Split/pane layout |
| Scrollback and search index | Scroll position |
| Read markers, activity, highlight state | Theme + rendering |
| Ignore lists, aliases, config | Terminal, input widget state |
| Input history | Composer draft text (synced, but client-authoritative while typing) |
| Nick lists, away state, account info | Which columns are visible |

Attach becomes trivial: a fresh client asks what buffers exist and where its read markers are, then renders. Detach loses nothing. If this table becomes ambiguous in practice, that ambiguity is a P0 design bug, not a feature request.

### 4.6 Message identity

The thing most likely to poison everything downstream if done wrong. Three separate concepts, never conflated:

- **`seq`** — monotonic `INTEGER` per buffer, assigned by us at insert. This is the primary key, the sort order, and the pagination cursor. It is ours, it never changes, and it is correct even when everything else lies.
- **`msgid`** — IRCv3 message ID from the server, when present. Used *only* for deduplication. Nullable.
- **`server_time`** — from the `server-time` tag, or local receipt time as fallback. Used *only* for display and for merging history batches. Never used for ordering within a buffer.

Server clocks are wrong. `CHATHISTORY` returns out-of-order batches. Bouncers replay. If ordering derives from timestamps, all three of those become bugs.

Dedup rule: on insert, if `msgid` is present and already exists for this buffer, skip. If absent, fall back to a content hash over (nick, text, coarse timestamp bucket) — imperfect, but it only matters for pre-IRCv3 servers where nothing better exists.

### 4.7 Backlog API

There is no "give me the buffer" request. Ever. Only:

```rust
FetchBacklog {
    buffer: BufferId,
    anchor: Anchor,      // Before(Seq) | After(Seq) | Latest | AroundSearchHit(Seq)
    limit: u32,          // capped server-side
}
```

This shape costs nothing today and is the difference between a client that works and one that hangs when a user has 400k lines in `#archlinux`. It also maps directly onto IRCv3 `CHATHISTORY`, so the same request shape works whether the lines come from our disk or from upstream.

### 4.8 Transport and wire protocol

**Default transport: Unix domain socket** at `$XDG_RUNTIME_DIR/havoc/core.sock`. Filesystem permissions are the auth model. No tokens, no TLS, no port, no code written by us that can get authentication wrong.

**Remote access: SSH-forward the socket.** `ssh -L` / `-R` against a UDS endpoint. Zero auth code, and it is what the target user already does.

**TCP is explicit opt-in only**, and when enabled requires TLS plus a pre-shared token or client certificates. Never plaintext TCP, not even on loopback — this is a full read/write channel into someone's private messages and a local-listener default is how that leaks.

**Framing:** length-prefixed frames (`tokio-util` `LengthDelimitedCodec`), CBOR bodies via `ciborium`.

**Two logical channels multiplexed over the one connection:**
- *Requests* — carry a correlation id, get exactly one response.
- *Events* — unsolicited, broadcast to every attached client.

**Handshake negotiates capabilities.** Client and core exchange feature lists and operate on the intersection. Thematically on-brand, and it means upgrading the daemon does not require simultaneously upgrading every attached client. Version-lockstep is a trap we can simply decline to enter.

### 4.9 Storage

SQLite via `rusqlite`, with FTS5 for full-text search. Schema migrations from the first commit (`refinery` or hand-rolled versioned SQL), run at startup.

Sketch:

```sql
CREATE TABLE network (id INTEGER PRIMARY KEY, name TEXT UNIQUE, ...);
CREATE TABLE buffer  (id INTEGER PRIMARY KEY, network_id INTEGER, name TEXT,
                      kind TEXT,            -- channel | query | server | special
                      last_read_seq INTEGER,
                      UNIQUE(network_id, name));

CREATE TABLE message (
  buffer_id   INTEGER NOT NULL,
  seq         INTEGER NOT NULL,       -- monotonic per buffer, ours
  msgid       TEXT,                   -- IRCv3, nullable, for dedup
  server_time INTEGER NOT NULL,       -- unix millis
  kind        INTEGER NOT NULL,       -- privmsg/notice/join/part/mode/...
  nick        TEXT,
  account     TEXT,
  text        TEXT,
  tags        BLOB,                   -- CBOR, remaining message-tags
  PRIMARY KEY (buffer_id, seq)
) WITHOUT ROWID;

CREATE UNIQUE INDEX msg_msgid ON message(buffer_id, msgid) WHERE msgid IS NOT NULL;
CREATE INDEX msg_time ON message(buffer_id, server_time);

CREATE VIRTUAL TABLE message_fts USING fts5(
  text, nick, content='message', tokenize='unicode61 remove_diacritics 2'
);
```

Write path is batched — accumulate for ~100ms or N messages, then one transaction. WAL mode on. A busy channel must never cause an fsync per line.

### 4.10 TUI

Ratatui + crossterm. Immediate-mode: rebuild the frame each tick from a local projection of core events. This deletes the entire class of incremental-repaint bugs that consume a large fraction of BitchX's `screen.c`/`window.c`.

The one thing Ratatui does not give us: **scrollback with wrapping.** We own a wrapped-line cache keyed by (buffer, width), invalidated on resize, holding pre-rendered `Line` values for a window around the viewport. This is the largest single piece of original UI work in the project and should be isolated in its own module with real tests.

The TUI is explicitly the most disposable part of the codebase. Rewriting a render function is an afternoon. We optimize the core for longevity and the TUI for iteration speed.

---

## 5. Design tradeoffs and roads not taken

Every decision below has a real alternative and a real cost. Recording the reasoning is the point — in eight months somebody will propose one of the rejected options and this section is the answer.

### 5.1 Storage: why `rusqlite` + FTS5

**Chosen:** SQLite via `rusqlite`, FTS5 for search.

**Why:** The workload is append-heavy writes plus range queries by (buffer, seq) plus full-text search. SQLite does all three in one embedded file with no daemon, no schema service, and a query planner. FTS5 is the decisive factor — it is a genuinely good full-text engine, already there, transactionally consistent with the data it indexes. The `content=` option means the index does not duplicate the message text. And SQLite's durability story under crash and power loss is the most thoroughly validated of anything in this space, which matters because this file is the user's irreplaceable personal archive.

**`sled` — rejected.** Long-standing beta status, historically heavy memory use, and — decisively — no full-text search. We would be building the search engine ourselves or bolting `tantivy` alongside, which means two stores that can disagree after a crash.

**`redb` / `fjall` — rejected.** Both are better-engineered embedded KV stores than sled. Same disqualifier: pure KV. We would hand-roll secondary indices and search, and hand-rolled indices drift out of sync with the data they index. That is a bug class we can decline to have.

**`sqlx` instead of `rusqlite` — rejected.** Compile-time-checked queries are genuinely nice, but they require a live database at build time (or a checked-in offline query cache that must be regenerated on every query change), and it pulls the async machinery into what is fundamentally a fast local synchronous operation. `rusqlite` on a blocking thread pool (`tokio::task::spawn_blocking`, or a dedicated storage thread with a channel) is simpler and faster. Revisit only if the storage layer grows beyond one person's head.

**PostgreSQL — rejected.** Requiring a database server for a terminal chat client is a category error. Also loses the "your history is one file you can copy to a USB stick" property, which is a real user-facing feature.

**Flat files / JSONL — rejected.** This is what WeeChat and irssi do, and it is precisely the thing we are trying to beat. Search becomes `grep`, which is O(n) over gigabytes and cannot filter structurally by nick, event kind, or time range. Read markers and dedup on top of append-only text files means writing a database badly.

**`tantivy` alongside SQLite — rejected for v1.** Better search than FTS5 (BM25 tuning, faceting, better tokenizer options) but a second store to keep consistent, a second thing to corrupt, and materially more memory. FTS5 is sufficient until proven otherwise. If we ever want fuzzy or semantic search, revisit — the schema does not prevent adding it.

**`libsql` / `duckdb` — rejected.** libsql buys us replication we do not need. DuckDB is analytical/columnar; wrong shape for point lookups and small transactional appends.

**Retained risk:** SQLite is a C dependency. If a fully-static, no-C build ever becomes a hard requirement, this is the thing that blocks it. Accepted knowingly.

### 5.2 IRC library: why we own the connection

**Chosen:** `irc-proto` for `Message` parse/serialize only. Our own connection state machine, our own TLS setup, our own cap negotiation, our own reconnect.

**Why:** Everything that makes this client different from a 2010 client lives in the connection state machine — IRCv3 capability negotiation, SASL mechanism selection, `CHATHISTORY` resync-on-reconnect, backoff policy, and the interaction between all four. That is the crown jewels. It must be ours.

**`irc` crate's `Client` — rejected.** Its model fuses connection, config, and message stream into one type. Building on it means fighting it the moment we want to control cap negotiation ordering or drive a resync. Upstream is also slow-moving; crates.io currently carries forks that exist specifically to patch rustls behavior, flush timing, and flood protection — which is itself the signal. We use its protocol half and skip its client half.

**Writing the parser from scratch — rejected.** IRC message parsing with IRCv3 message-tags has more edge cases than it looks (tag escaping, trailing parameters, the `@`/`:`/space grammar). `irc-proto` has had years of exposure to real servers. Low value in redoing it, and the abstraction cost is nil since `Message` is a plain data type.

**Risk:** if `irc-proto` stagnates, we vendor it. It is small and self-contained — this is a cheap escape hatch, which is exactly why depending on it is safe.

### 5.3 IPC serialization: why CBOR over length-prefixed frames

**Chosen:** `ciborium` (CBOR) in `LengthDelimitedCodec` frames.

**Why:** Self-describing, schema-evolution-friendly, compact enough, serde-native, and debuggable — you can dump a frame and read it. Unknown fields can be tolerated, which is what makes the capability handshake work.

**gRPC / `tonic` — rejected.** Heavy dependency tree, protobuf codegen in the build, and it fights us on the exact thing we need most: a long-lived bidirectional event stream with server-initiated broadcast to N clients. We would spend real time working around gRPC's request/response ergonomics for something a raw socket does natively. Also, gRPC over a Unix socket to a local process on the same machine is ceremony without benefit.

**JSON-RPC — rejected.** Tempting for debuggability. Rejected on: no binary type (message tags and future inline content become base64 bloat), noticeably slower to parse at backlog-fetch volumes, and no meaningful ecosystem advantage here since both endpoints are ours. We recover the debuggability with a `havoc debug tap` command that pretty-prints the CBOR stream.

**`postcard` — rejected, narrowly.** Faster and smaller than CBOR. But it is non-self-describing, so schema evolution requires strict version lockstep between daemon and client — the precise failure mode we designed the capability handshake to avoid. Reconsider only if profiling shows serialization is hot, which it will not be.

**MessagePack — near-tie with CBOR.** CBOR chosen for having an actual RFC and slightly better-maintained Rust support. Not a decision worth relitigating.

**Cap'n Proto / flatbuffers — rejected.** Zero-copy matters when you are doing millions of messages per second. We are doing hundreds per minute.

**Using the IRC protocol itself as the internal transport — rejected.** Cute, and WeeChat-relay-adjacent. But our event model (read markers, buffer lifecycle, search results, activity state) does not fit IRC's grammar, and forcing it would mean inventing custom commands until it is a worse binary protocol wearing IRC's clothes. We do plan to *speak* IRC to third-party clients (§7.9), but that is a separate, deliberately lossy public surface — not the internal one.

### 5.4 Process model: why embedded-first

**Chosen:** embedded single-process by default, daemon mode as a flag, both over the same typed boundary.

**Why:** The architectural benefit of the split is testability and future clients; the *cost* of the split is service management, socket lifecycle, orphan cleanup, and a much worse first-run experience. Embedded-first gets the benefit without paying the cost until we need it. Critically, since the boundary is enforced by the crate graph, the daemon is additive work rather than a refactor — if wiring UDS transport takes more than a few hundred lines, we will have learned that the boundary was wrong, cheaply.

**Daemon-first — rejected for v1.** Correct end state, wrong starting point. It front-loads infrastructure work before we have used the thing daily enough to know if the state boundary is in the right place.

**No daemon ever — rejected.** Persistence, multi-client attach, and TUI-restart-without-dropping-connections are all real goals. `screen` is not an acceptable answer in 2026.

**Full bouncer (daemon holds connections, serves any client) — deferred, not rejected.** This is the natural end state and the strongest product story: one thing to install, no ZNC config. It is also a significant scope increase (per-network persistence semantics, multi-user considerations, the "daemon is down" story). The architecture above *is* the bouncer architecture — we simply are not shipping the standalone listener in v1. See §7.9.

### 5.5 Concurrency: why actors

**Chosen:** one task per network, communication by channel, no shared mutable state across subsystem boundaries.

**Why:** It makes the serializable boundary natural rather than aspirational, it makes multi-network free, and it makes reconnect logic local to the thing that reconnects.

**`Arc<Mutex<AppState>>` — rejected, emphatically.** This is the single decision that would make the daemon split a rewrite. It is also how you get lock-ordering deadlocks between the render loop and the network task at 2am. The moment core and UI can share a lock, the architecture is decorative.

**Actor framework (`actix`, `ractor`) — rejected.** `tokio::sync::mpsc` plus an enum is the whole pattern. A framework adds concepts without removing work at this scale.

**`tokio` vs `smol`/`async-std` — tokio.** Ecosystem gravity: `rustls`, `tokio-util`, `tokio-rustls`, and most of what we would want later assume it. Not a close call.

### 5.6 TUI: why Ratatui

**Chosen:** Ratatui + crossterm.

**Why:** Immediate mode is the correct model here — it eliminates the incremental-repaint bug class, and diffing against the previous frame means it is fast enough regardless. Crossterm gives us cross-platform terminal handling including Kitty keyboard protocol support.

**`cursive` — rejected.** Retained-mode widget tree with callbacks; less flexible for the dense custom rendering we want, and the callback model fights an async event stream.

**Hand-rolled on `termwiz`/raw ANSI — rejected.** This is `term.c` again: 2,592 lines of termcap poking that we get to simply not write.

**Accepted cost:** Ratatui has no scrollback primitive. We build the wrapped-line cache ourselves (§4.10). Unavoidable in any option.

### 5.7 Scripting: why Lua via `mlua`, and why later

**Chosen:** `mlua` (Lua 5.4, sandboxed), post-v1.

**Why Lua:** It is what the target user already knows from WeeChat and irssi. Familiarity is the entire value proposition of a scripting language in this niche. `mlua` is mature and its safety story is well-understood.

**Rhai — rejected.** Pure Rust and pleasant to embed, but nobody in this user base knows it. We would be asking people to learn a language to configure a chat client, which is exactly the irssi failure mode.

**WASM plugins — rejected for now.** Better sandboxing and any-language authorship, but the ergonomics for a fifteen-line "highlight when someone says my project name" script are terrible. Revisit if a plugin ecosystem ever materializes.

**No scripting at all — rejected, but note the ethos tension.** Scripting is the escape hatch, not the architecture. The design target is that 90% of users never open a script file. If we find ourselves answering feature requests with "write a script," we have become irssi and should stop.

### 5.8 Smaller calls

- **TLS: `rustls`, not `native-tls`/OpenSSL.** No C toolchain dependency, better defaults, no system-store surprises across platforms. Costs us some obscure legacy-server compatibility; acceptable.
- **Config: TOML.** Boring, universal, `serde`-native. KDL is nicer for nested/DSL-ish config and is worth reconsidering *only* for theme files, where the structure is genuinely tree-shaped. YAML rejected on principle.
- **Theme: data file, not 237 `/FSET` strings.** Named semantic slots (`nick.self`, `event.join`, `notice.server`), truecolor, hot-reloadable. Ship two or three, one unapologetically loud.
- **Fuzzy matching: one engine (`nucleo`) for everything** — command palette, buffer switcher, nick completion, search-within-scrollback. Consistency of behavior is the feature.
- **Credentials: `keyring` where available, encrypted file fallback.** Never plaintext passwords in the config file, ever, including for SASL.
- **Testing: heavy on core protocol/state transitions, none on TUI rendering.** The bugs that cost days live in cap negotiation, reconnect, and dedup. Those are testable precisely because the boundary is clean.

---

## 6. Pitfalls

Ordered roughly by how much they would hurt.

1. **Timestamp-based ordering.** Poisons scrollback, read markers, pagination, and history merge simultaneously. *Mitigation:* `seq` is the only ordering key. Make `server_time` structurally unavailable to sort paths — different type, if that is what it takes.
2. **The boundary erodes.** Someone adds `rusqlite` to `havoc-tui` "just for this one query." *Mitigation:* the crate graph forbids it. Add a CI check on dependency edges. This is not negotiable and should be caught by a machine, not a reviewer.
3. **Unbounded backlog fetch.** Works fine for three months, then someone has 400k lines and attach takes ninety seconds. *Mitigation:* windowed API from day one, `limit` capped server-side regardless of what the client asks for.
4. **Duplicate messages after reconnect.** The most visible possible failure, and it destroys trust in the history feature. *Mitigation:* `msgid` dedup with a unique index enforcing it at the storage layer, not the application layer. Test with a real bouncer replaying.
5. **fsync per message.** A busy channel turns the client into a disk-thrashing space heater. *Mitigation:* WAL mode, batched transactions on a ~100ms timer.
6. **Blocking the render loop on storage.** Search over a large FTS index freezes the UI. *Mitigation:* storage on its own thread behind a channel; search results arrive as events like everything else.
7. **Wrapped-line cache invalidation.** Resize during scrollback is where every terminal chat client has bugs. *Mitigation:* isolate in one module, key the cache on (buffer, width), test it directly with property tests over random resize sequences.
8. **Cap negotiation ordering.** SASL before `CAP END`, `CAP END` only after everything requested has resolved, handle `CAP NEW`/`CAP DEL` mid-session. Getting this subtly wrong produces failures that only appear on one specific network. *Mitigation:* explicit state machine, table-driven tests against captured real-server transcripts.
9. **Multi-network retrofitted.** *Mitigation:* `HashMap<NetworkId, _>` from commit one, even with one network.
10. **Schema migrations added later.** The first time we need to change the schema after users have data, without a migration framework, is a crisis. *Mitigation:* migrations from commit one, including the no-op initial one.
11. **Building the daemon before dogfooding.** We would be committing to a state boundary we have not tested with real use. *Mitigation:* the mandatory month in §8.
12. **Scope creep into "a chat client."** Someone will want Matrix. *Mitigation:* §2.4 exists.

---

## 7. Feature opportunities unlocked by this stack

Things that are cheap *because* of the choices above, and are genuinely novel in a terminal IRC client. Not commitments — a menu.

### 7.1 Search that is actually search — `rusqlite` + FTS5

`/search from:nickname in:#channel after:2024-03 "deployment failed"`. Cross-network, cross-buffer, instant, with structural filters on nick, event kind, and time range. Jump-to-context on a hit (`AroundSearchHit` anchor, §4.7) drops you into scrollback at that moment with surrounding lines. Nothing in the terminal does this. It is arguably the flagship feature.

Extension: saved searches as pinned virtual buffers — "everything anyone has said about `havoc` on any network, ever," live-updating.

### 7.2 Real backlog sync — IRCv3 `CHATHISTORY` + `server-time`

Reconnect fetches only what was missed, merges by `msgid`, renders in correct order with original timestamps. Works with soju out of the box. The user-visible property: closing your laptop stops being an event.

### 7.3 Command palette and fuzzy everything — `nucleo`

`Ctrl-K` opens a palette over commands, buffers, nicks, and recent URLs. Buffer switching by fuzzy name across networks. This single feature replaces a meaningful fraction of what people historically wrote scripts for.

### 7.4 Clickable, copyable terminal — crossterm + OSC 8/52

- OSC 8 hyperlinks: URLs in chat are click-to-open, no URL-grabber gymnastics.
- OSC 52: `y` on a selected line copies to system clipboard *through SSH*. This is a small feature that people find delightful out of proportion to its size.
- Mouse: click to focus a pane, scroll wheel in scrollback, click a nick for a context menu.
- Kitty keyboard protocol: `Ctrl-Shift-<key>` bindings that actually work, distinguishable from `Ctrl-<key>`.

### 7.5 Inline images — `ratatui-image`

Kitty/iTerm2 graphics protocol with Sixel and halfblock fallback. Image links preview inline (opt-in, size-capped, with a per-buffer toggle). Also: render a small activity sparkline per buffer in the buffer list.

### 7.6 Native notifications — `notify-rust`

Real desktop notifications on highlight and PM, with sane rate limiting, and suppression when the buffer is focused. Over SSH, fall back to OSC 9/777 so the *local* terminal notifies.

### 7.7 Analytics on your own history — SQLite

The data is already in a queryable store, so this is nearly free: `/stats #channel` for activity by hour and top talkers, first-seen/last-seen for a nick across all networks, "when is this channel actually awake" heatmaps. Fun, occasionally useful, and a strong demo of why the storage decision was right.

### 7.8 Live theme reload — `notify` (filesystem watcher)

Edit the theme file, see the change instantly, no reconnect. Makes theming pleasant enough that people actually do it, which serves the "loud and legible" pillar.

### 7.9 Speak IRC to third-party clients

Once the core exists, a plain-IRC listener is a modest addition (roughly a couple thousand lines) and turns `havocd` into a usable bouncer for people still on HexChat or a phone client. Two large benefits: it is an adoption wedge (try the daemon without switching clients), and it forces the core's state model to be honest, because a foreign client will exercise assumptions our own TUI silently shares.

### 7.10 Async AI assists — optional, sandboxed, off by default

The architecture makes this trivially safe to add later since it is just another event producer: summarize a busy channel's last two hours, or "what did I miss." Explicitly opt-in, explicitly network-egress-flagged, and never on by default. Noted for completeness; not a v1 concern and not a thing to let distort the design.

---

## 8. Roadmap

**M1 — Types and core skeleton.** `havoc-ipc` message types. `havoc-core` with connection actor (one network), TLS, SASL, cap negotiation. SQLite with migrations. Driven by a debug CLI, no TUI. Tests on the state machine.

**M2 — Storage and search.** Batched writes, FTS5, windowed backlog API, read markers. Still headless.

**M3 — TUI.** Ratatui frontend over in-process channels. Buffer list, scrollback with the wrapped-line cache, input widget, theme file, nick coloring. **This is a daily-usable client.**

**M4 — Dogfood for one month.** Not optional. Use it as the only IRC client. The purpose is to find out whether the state boundary is in the right place *before* anything else depends on it. Fix what this surfaces. Ship nothing new.

**M5 — Daemon.** Swap channel transport for UDS. Capability handshake. Attach/detach. If this is not a few hundred lines, stop and reexamine M1.

**M6 — Multi-network, `CHATHISTORY` resync, notifications, command palette.**

**M7+ — Scripting, multi-client attach, plain-IRC listener, images.**

---

## 9. Open questions

- **Buffer identity across networks.** Is `#rust` on two networks one buffer or two? (Almost certainly two, but merged-view is a real feature request waiting to happen — decide before the schema hardens.)
- **Read marker reconciliation** when two clients are attached. Last-write-wins on timestamp is probably fine; confirm before shipping multi-client. Also decide whether to propagate upstream via IRCv3 `draft/read-marker`.
- **Retention policy.** Do we ever delete? Default is "never," but a user with a decade of `#linux` may disagree. Needs at least a vacuum/archive story.
- **Config vs. runtime state.** Where does "I joined this channel manually" live — config file or database? (Leaning database, with config as seed only.)
- **The name.** `havoc` is a placeholder.

---

*Amendments to this document should be dated and appended, not silently edited.*
