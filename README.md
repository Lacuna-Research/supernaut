# havoc

A terminal IRC client for people who want the client to already be good, not to
become good after three weekends of configuration.

Headless core (`havoc-core`) that owns connections, history, and state; a Ratatui
frontend (`havoc-tui`) that owns nothing but the viewport. Modern IRC — TLS, SASL,
IRCv3 — as the normal path. Every line you have ever seen lands in SQLite and is
searchable in milliseconds.

**Status: pre-alpha.** Nothing runs yet; the design baseline is written and the
build is being stood up. `havoc` is a working codename.

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
