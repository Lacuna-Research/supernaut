# Supernaut — Project Instructions

A terminal IRC client: headless havoc engine, Ratatui TUI, SQLite history. The why:
`NORTH-STAR.md` · `PLAN.md` roadmap · `STAGE-1-PROMPTS.md` queue · `BUILD-LOG.md` log.

## Build standards

- Rust stable, cargo workspace under `crates/`. Warnings are errors, clippy strict,
  `rustfmt` enforced, `cargo test`. `make build test lint fmt` are the entry points.
- **Dependencies come from the allowlist in `scripts/check-docs.sh`.** Adding one
  requires a decision entry in `BUILD-LOG.md` and an edit to that allowlist, which
  fails the build otherwise. Vendored fixtures must record their upstream commit SHA.
- **Crate dependency boundaries (NORTH-STAR §4.2) are machine-enforced** on the
  Cargo.tomls: `supernaut-tui` never touches rusqlite/rustls/irc-proto, `havoc-core`
  never touches ratatui/crossterm, `havoc-ipc` stays near zero-dep. If the TUI needs a
  core type, the type moves to `havoc-ipc`.

## Working method

One prompt from the current stage's queue per branch (`prompt-NN-slug`), per PR; never
commit to `main` directly. Every `PLAN.md` item is attached to a prompt. **Merge your
own PRs** — squash-merge, delete the branch once CI is green. Stop and ask only if CI
is red, work diverged from its prompt, or a decision is genuinely the user's.

**Starting a prompt:** re-read it in the queue, including any carry-forward block. If
its detail is unwritten, write it now — in a **fresh sub-agent** given the `PLAN.md`
item, its notes, and the relevant source, never the previous session — never early.

**Finishing a prompt, before reporting done:**

1. Append a `BUILD-LOG.md` entry — deviations, deferrals, surprises, measurements.
   `make check` requires the Shipped, Learned, and Live run sections by name.
2. Run the **post-prompt review** — one sub-agent dispatch (brief in
   `SUBAGENT-BRIEFS.md`) doing both the adversarial check and the carry-forward
   harvest. Record dispositions in the entry's Review and Carry-forward raised
   sections. For a trivial change, skip it and write `**Review:** skipped, trivial
   (<reason>)` — the honesty is the requirement, not the ritual. Consumed notes get
   acted on, deleted, and recorded in `**Carry-forward consumed:**` — as a pair.
3. Push anything deferred into `PLAN.md` at the stage where it belongs.
4. Run it live, not only under test: the debug CLI (headless stages) or the client,
   against local `ergo` or a public network, isolated `SUPERNAUT_CONFIG_DIR`. A
   change with no observable behavior may record `**Live run:** N/A (<reason>)` —
   claiming a run that did not happen is the one dishonesty that matters most here.
5. Bump the `**Status:**` line, then `make check`, merge, and leave the worktree — a
   prompt ends at the repo root, not in its worktree.

**Between prompts.** Record decisions *at the moment they are made*, never deferred:

- A choice with a rejected alternative → a decision entry in `BUILD-LOG.md`, with the
  reasoning and what would justify revisiting it.
- A change to scope or approach → edit `PLAN.md` in the same turn, never "later".
- A question left open → the **Still open** list in `PLAN.md`, marked with the
  machine-readable blocking annotation. `make check` refuses to start a blocked prompt.

**Sub-agents advise; artifacts decide.** Output lands as a proposal in an existing
artifact — never a new channel or authority — and its disposition is recorded there.

## Enforced mechanically

`make check` (pre-commit + CI) enforces: the cap on this file, `BUILD-LOG.md`
append-only, a substantive log entry per `crates/` change, status/README/plan
agreement, carry-forward notes consumed and recorded as one act, blocked prompts not
starting, oversize changes justified, corrections closing the mechanization loop,
recurring failure categories raising a rule, ratchets never worsening, stage
retrospectives, the dependency allowlist, and the crate boundaries. Git hooks and a
Stop hook guard the rest: no commits to `main`, no unrenamed worktree branch pushed,
no worktree outliving its merged PR. Local failures append to `discipline-stats.txt` —
telemetry the retrospective's rule review argues from, instead of memory. When a
convention proves important, make it mechanical rather than more emphatic — **a new or
changed check lands with fixtures in `scripts/test-checks.sh` in the same PR**; a
broken check fails open, and open is silent.

## Stage boundaries

When a stage's status reads N/N, `make check` fails until `BUILD-LOG.md` carries a
`## Retrospective — Stage N` entry (template in the log): this file re-read and pruned,
docs audited, the rule review argued from `discipline-stats.txt`, the failure register
reviewed, and the **cold-start drill** — a fresh sub-agent, given only the repo, states
what is next and why; every gap between its answer and reality is a documentation bug
to fix before the next stage starts. Before a planned context reset, run the same
audit as a handoff entry.

## Maintaining these documents

Keep docs current without being asked; fix a stale doc in the same commit as the code
that staled it. Reasoning belongs in `BUILD-LOG.md`, not here — that split keeps this
file under the 100-line cap, which is not to be raised. Read the log's last entries or
search it, never front to back; open questions live only in `PLAN.md`'s **Still open**
list. `PLAN.md` is a living roadmap: reorder, rescope, delete freely; reference items
by name, never number. Propose structural changes here rather than making them
silently; prunes need none.

## Where things live · Secrets

Config `$XDG_CONFIG_HOME/supernaut` (TOML), data `$XDG_DATA_HOME/supernaut` (SQLite),
socket `$XDG_RUNTIME_DIR/supernaut/core.sock` — overridable via `SUPERNAUT_CONFIG_DIR`;
the program never writes to its own source tree, not even gitignored. Config paths are
public API. SASL/NickServ credentials live in the OS keyring (encrypted-file fallback
deferred — PLAN stage 4 item 3), never in config, logs, or the database. Fixture
credentials must be recognisably fake, never a real-shaped token.
