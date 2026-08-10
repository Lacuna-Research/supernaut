# Supernaut

![stage 1 progress](https://img.shields.io/badge/stage%201-6%2F10-blue)

A terminal IRC client for people who want the client to already be good, not to
become good after three weekends of configuration.

A headless engine — **havoc** (`havoc-core`) — owns connections, history, and
state; a Ratatui frontend (`supernaut-tui`) owns nothing but the viewport. Modern
IRC — TLS, SASL, IRCv3 — as the normal path. Every line you have ever seen lands
in SQLite and is searchable in milliseconds.

**Status: pre-alpha.** Nothing runs yet; the design baseline is written and the
build is being stood up. Everything below describes what is being built, not what
you can download today.

## The ethos

Spiritual heir to BitchX, minus the 1998: batteries included, not batteries
available. The default configuration is the product — a feature that requires
config-file editing to discover has failed. Opinionated defaults you can turn off
beat settings you have to discover. Loud, dense, legible output with a point of
view. The war scripts, DCC, and `screen`-as-persistence stay in 1998.

The full argument — including every design tradeoff and the roads not taken — is
in [`NORTH-STAR.md`](NORTH-STAR.md).

## Features (by design)

- **Search that is actually search.** Full-text over every network, every buffer,
  all time, in milliseconds — `/search from:nick in:#channel after:2024-03 "the
  thing"` — with jump-to-context. SQLite + FTS5. Arguably the flagship; nothing in
  the terminal does this.
- **Reconnect is invisible.** Close the laptop, open it: IRCv3 `CHATHISTORY`
  resync, `msgid` dedup, no duplicated lines, no gaps, no `/lastlog` archaeology.
- **Encrypted and authenticated by default.** TLS via rustls and SASL as the
  normal login path; credentials in the OS keyring, never plaintext. Plaintext IRC
  is a loud, explicit opt-in.
- **Your history is one file.** A SQLite database you can back up, query, or copy
  to a USB stick — not a directory of grep-hostile log files.
- **One binary, three modes.** Embedded by default; `--daemon` and `--connect` to
  split core from UI without a bouncer — attach, detach, lose nothing. Remote
  access is an SSH-forwarded Unix socket, so there is no auth code to get wrong.
- **The terminal is capable now, so use it.** Truecolor, mouse, OSC 8 hyperlinks,
  OSC 52 clipboard-through-SSH, Kitty keyboard protocol.
- **Fuzzy everything.** One engine (`nucleo`) behind the command palette, buffer
  switching, and nick completion.
- **A face.** Dense, colored, differentiated output; hot-reloadable themes; at
  least one shipped theme that is unapologetically loud.

## Roadmap

The living roadmap is [`PLAN.md`](PLAN.md) — stages, open questions, and
reasoning live there, and it wins when this summary drifts. The table below is
machine-checked against the work queue on every commit, so it cannot quietly rot.

### Stage 1 — headless core (in progress)

The havoc engine alone: connect over TLS with SASL, log everything to SQLite,
answer search and backlog requests — driven by a debug CLI, no UI yet.

| # | Prompt | Status |
|---|---|---|
| 1 | Workspace scaffold and build discipline | ✅ done |
| 2 | IPC wire types | ✅ done |
| 3 | Storage schema and migrations | ✅ done |
| 4 | Connection state machine, offline | ✅ done |
| 5 | Event bus, request handler, and debug CLI | ✅ done |
| 6 | Live connection, TLS, and reconnect | ✅ done |
| 7 | Message ingestion, identity, and batched writes | ⬜ todo |
| 8 | Full-text search | ⬜ todo |
| 9 | Windowed backlog and read markers | ⬜ todo |
| 10 | Network config and credentials | ⬜ todo |

### Stage 2 and beyond

2. **TUI** — a daily-usable client: scrollback, buffer list, input, themes.
3. **Dogfood month** — only client, nothing new ships; the state boundary earns
   trust before anything depends on it.
4. **Daemon and attach** — the embedded/daemon split goes live over a Unix socket.
5. **Multi-network, `CHATHISTORY` resync, notifications, command palette.**
6. **Polish and release** — multi-client attach, the §7 feature menu (scripting,
   plain-IRC listener for other clients, inline images), packaging.

## Orientation

| File | What it is |
|---|---|
| `NORTH-STAR.md` | The 30k-foot view: ethos, architecture, tradeoffs, pitfalls. The arbiter of scope disputes. |
| `PLAN.md` | The living roadmap, and the single list of open questions. |
| `STAGE-1-PROMPTS.md` | The current work queue. |
| `BUILD-LOG.md` | Append-only record of decisions, deviations, and corrections. |
| `METHOD.md` | Why the working method looks the way it does. |

## Building

Rust stable via [rustup](https://rustup.rs). Then:

```
make build test lint    # cargo, warnings-as-errors
make check              # documentation discipline (also runs on pre-commit and CI)
```

New clones: `make hooks` once, to install the git hooks.
