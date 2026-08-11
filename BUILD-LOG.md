# Build Log

Append-only, chronological, newest at the bottom. Never edit a past entry — if something
here turns out to be wrong, correct it in a later entry and say so.

Four kinds of entry, interleaved in the order they happened:

- **Prompt entries** — what a unit of work produced. Template below.
- **Decision entries** — a choice made in conversation, outside any prompt. Written
  *when the decision is made*, not deferred to the next prompt's wrap-up. A decision
  that waits to be recorded is a decision that gets re-litigated in three weeks. Record
  the alternative rejected and why, not just the choice; the reasoning is the part that
  stops us going around again.
- **Correction entries** — a past entry was wrong. Say which, say why, say what is true.
  This file is append-only precisely so that the wrong answers stay visible; the
  sequence of corrections is the record of how this project actually goes wrong.
- **Retrospective entries** — one per finished stage. `make check` refuses to let
  anything else land until it exists.

The valuable content in this file is **not** what was built. Git already knows that, and
in more detail. What git cannot tell you is what we did differently from the plan and
why, what we chose not to do, what surprised us, and what we measured. Write those. If a
section has nothing worth saying, write "None" and move on — padding it makes the file
unreadable, and an unreadable log is the same as no log.

**Open questions do not live here.** They live in `PLAN.md`'s **Still open** list, all of
them, wherever they were first raised.

---

## Template

```
## Prompt N — <title>

**Commit:** <sha>  **Date:** <YYYY-MM-DD>

**Shipped:** One or two lines. Reference the commit; do not restate the diff.

**Deviations:** Where the implementation differs from the prompt as written, and why.
"None" is a valid and common answer.

**Deferred:** In scope but didn't land, and where it now lives in PLAN.md.

**Learned:** Gotchas, dead ends, surprising API behaviour, things the documentation got
wrong. This is the section that pays for the file existing.

**Measured:** Concrete numbers — benchmark results, timings, line counts. Omit if
nothing was measured; never estimate a number here.

**Live run:** What was exercised against a real target, and what it caught. Say
explicitly what could *not* be verified live and why, rather than leaving the gap
implicit.

**Review:** What the adversarial reviewer found, and what was done about each finding —
including "rejected, because". A finding with no disposition is a finding that gets
found again.

**Oversize:** Only if the change exceeded the size cap — why it could not be split, or
why the bulk is mechanical. `make check` requires this section when the cap is hit.

**Carry-forward consumed:** Notes from earlier prompts applied here. `make check`
requires this section whenever a carry-forward block was deleted in the same change.

**Carry-forward raised:** Notes appended to later prompts, with prompt numbers. Say
which came from the harvest sub-agent and which were rejected from it — a rejected
proposal is information too.
```

---

## Decision template

```
## Decision — <short title>
**Date:** <YYYY-MM-DD>  **Affects:** <files/prompts/stages>

**Chose:** what we're doing.
**Over:** the alternative(s) rejected.
**Because:** the reasoning. If this decision is ever revisited, this is the line that
gets argued with.
**Revisit if:** the condition that would change the answer. Omit if none.
```

---

## Correction template

Every correction must close its own loop — `make check` requires exactly one of the two
final lines. And it must be categorized: one occurrence is an accident, two a
coincidence, three a habit, and at three `make check` demands a `**Rule raised:**` line
on one of the category's entries, naming the CLAUDE.md rule or check it became. Keep
category names short, stable, and reused — the register only works if the same failure
gets the same name every time. Seed list: `asserted-not-tested`, `stale-doc-copy`,
`scope-leak`, `convention-not-checked`.

```
## Correction — <what was wrong>
**Date:** <YYYY-MM-DD>  **Supersedes:** <the entry being corrected>
**Category:** <short-stable-name>

**Claimed:** the wrong thing, quoted or paraphrased.
**Actually:** what is true, and how it was established — tested, not reasoned.
**Lesson:** recorded because it will recur.

**Mechanized as:** <the check or hook this became, added in this same change>
    — or —
**Not mechanizable because:** <the honest reason this stays a judgement call>

**Rule raised:** <only when this category hits its third occurrence: the rule or check
it became>
```

---

## Retrospective template

Written when a stage's status reaches N/N; `make check` blocks everything else until it
exists. This is the forced pause at the boundary — the four audits below are exactly the
ones momentum skips.

```
## Retrospective — Stage N
**Date:** <YYYY-MM-DD>

**CLAUDE.md:** re-read whole; what was pruned, what was added, line count after.

**Docs audit:** each artifact checked for staleness; what was fixed.

**Failure register:** the categories accumulated this stage, their counts, and whether
any is trending toward its limit.

**Rule review:** argued from `discipline-stats.txt`, not from memory. Which rules fired
zero times this stage — candidates for deletion, with a decision entry each. Which
fired constantly — either working hard or miscalibrated into compliance theater; the
ledger says where to look, not which. Any rule deleted or changed lands with its
fixtures updated in the same PR.

**Cold-start drill:** a fresh sub-agent, given only the repo, was asked what is next
and why. Its answer, verbatim or summarized; every gap between it and reality; and the
documentation fix each gap produced. A drill with no gaps is worth recording too — it
is the only direct evidence the system works.

**Ratchets:** current values vs ceilings; which were tightened.

**Next stage:** what was reordered, rescoped, or deleted in PLAN.md before opening it.
```

---

## Handoff template

Written before any planned context reset. Audit what is true only in the working
session rather than on disk.

```
## Decision — handoff hardening before a context reset
**Date:** <YYYY-MM-DD>  **Affects:** <files changed to close the gaps>

<What was audited, how many gaps were found, and each one closed — not merely noted.>

### State at handoff

<Branch and status line. Open PRs. Worktrees. What is implemented vs still stubbed.
What is next, and whether it has carry-forward notes waiting.>

### Things learned that are worth not relearning

- **<Toolchain or API trap, in bold.>** <The one-line consequence.>
- **My own recurring failure mode:** <read the Category tags first — the register is
  this section, computed instead of recalled>.
```

---

## Decision — bootstrap: adapt dev-template for HAVOC
**Date:** 2026-08-09  **Affects:** CLAUDE.md, scripts/check-docs.sh, scripts/measure-ratchets.sh, scripts/test-checks.sh, Makefile, README.md

**Chose:** `SOURCE_DIR=crates` (cargo workspace under `crates/`); dependency policy as
an allowlist in check 15 seeded with exactly the crates NORTH-STAR.md §5 commits to
for v1 plus the workspace's own crates; the NORTH-STAR §4.2 crate boundaries as
mechanical checks on the Cargo.tomls (check 16a) — blocklists for havoc-tui /
havoc-core / havoc-transport, a strict serde/time/bitflags allowlist for havoc-ipc.
Both checks parse *declared* dependencies with awk over Cargo.toml.
**Over:** seeding the allowlist with obviously-coming infra crates (thiserror,
tracing, clap); using cargo-metadata for a resolved dependency graph.
**Because:** the north-star is already the decision log for its named dependencies, so
they need no new entries — but anything it does not name should pay the designed cost
(decision entry + allowlist edit) at the moment it is actually needed, not be waved
through in bulk now. Declared-not-resolved parsing keeps the check free of a toolchain
dependency; the transitive graph is the compiler's job once a dep is used, and the
failure this check exists for — "rusqlite in havoc-tui just for one query" — is a
declaration.
**Revisit if:** a `package = "..."` rename ever hides a real dependency from the awk
parser (then move to cargo-metadata), or per-dep friction proves so high it invites
batch additions.

## Decision — bootstrap: Makefile stays unadapted until the workspace exists
**Date:** 2026-08-09  **Affects:** Makefile, .github/workflows/ci.yml, stage 1 prompt 1

**Chose:** leave the `build`/`test`/`fmt`/`lint` placeholder bodies in place for the
bootstrap PR; stage 1's first prompt scaffolds the cargo workspace and fills them in
the same change.
**Over:** filling in cargo commands now.
**Because:** with no Cargo.toml in the tree, adapted bodies make CI's build job fail
red on a docs-only PR, and the template's build job is designed to skip loudly —
keyed on the placeholder text — for exactly this window. Cargo commands without a
workspace would also be untestable on this machine (no Rust toolchain installed yet).
**Revisit if:** never — this resolves itself when prompt 1 lands.

## Decision — bootstrap: ratchet baselines and README apparatus
**Date:** 2026-08-09  **Affects:** ratchets.txt, README.md

**Chose:** keep the template's ratchets (`todo-count 0`, `longest-file 400`) as
starting ceilings, and a README without the stage badge / progress table.
**Because:** with zero source, measured values are 0 — a `longest-file 0` ceiling
would fail on the first file, so 400 is a policy cap, consistent with the north-star's
bias for small isolated modules; `todo-count 0` means deferred work lives in PLAN.md,
not in comments. The README progress apparatus is opt-in by design and a private
working repo does not earn its per-prompt upkeep; the badge can be added later, at
which point the checks engage on their own.
**Revisit if:** the first retrospective finds the ratchets never fired (tighten or
replace with Rust-specific rot metrics, e.g. unwrap-count outside tests).

## Decision — plan bootstrap: PLAN.md and the stage-1 queue adopted
**Date:** 2026-08-09  **Affects:** PLAN.md, STAGE-1-PROMPTS.md, scripts/check-docs.sh (STAGES), scripts/test-checks.sh

**Chose:** the Plan bootstrap sub-agent's proposal (inputs: NORTH-STAR.md plus the
template format references, per SUBAGENT-BRIEFS.md), adopted without edits. Its
judgment calls, each reviewed and accepted: stage 1 merges NORTH-STAR §8's M1+M2 (a
core that connects but stores nothing is not usable; a headless logger with instant
search is); the dogfood month is its own stage with a grown-not-planned queue; M7+ is
a menu inside stage 6, not commitments; storage-on-a-dedicated-thread treated as
settled by §6.6 rather than §5.1's looser either-or; the reconnect state ships as a
named seam in prompt 4 with resync deferred to stage 5, resolving §4.4 vs §8; two
Still-open items beyond §9 (migration mechanism, SASL mechanism set); prompt 5 gets a
plain-TCP-to-local-ergo connector behind a loud explicit flag one prompt before TLS,
read as §2.3-compliant because the opt-in is loud and the peer is localhost — accepted
to front-load the live-run harness; ergo is a test-harness binary, not a crate
dependency, so it stays off the allowlist; the extra `**Branch:**` line per prompt is
kept (no check parses it). STAGES total set to 10 and the fixture builder's sed synced
in the same change, per the queue file's own comment.
**Over:** folding live connect entirely into prompt 6 (rejected: every prompt from 5
on then lacks a live-run harness for one more session), and upgrading the later-stage
Still-open items to machine-readable blocking form (rejected for now: the checker only
enforces stages listed in STAGES, so the prose pointers are the honest form until
those queues exist).
**Because:** the proposal maps cleanly onto the north-star with no silent
resolutions — every ambiguity it found landed in Still open, which is exactly the
contract. From this moment `PLAN.md` is authoritative for the roadmap; NORTH-STAR.md
holds the why, and divergence gets a decision entry.
**Carry-forward consumed:** none in truth — the removed block was the placeholder
example in the template's queue file (`Prompt <K+1>`), deleted wholesale when the real
stage-1 queue replaced the placeholder. No live note existed to consume.

## Decision — the name: Supernaut, with havoc as the headless engine
**Date:** 2026-08-09  **Affects:** NORTH-STAR.md (dated amendment), CLAUDE.md, PLAN.md, STAGE-1-PROMPTS.md, README.md, scripts/check-docs.sh, scripts/test-checks.sh

**Chose:** the app is **Supernaut** (user's call, made in conversation): binary crate
`supernaut`, frontend crate `supernaut-tui`, user-facing paths
`$XDG_CONFIG_HOME/supernaut` / `$XDG_DATA_HOME/supernaut` /
`$XDG_RUNTIME_DIR/supernaut/core.sock`, env override `SUPERNAUT_CONFIG_DIR`. `havoc`
survives as the headless engine family — `havoc-core`, `havoc-ipc`,
`havoc-transport` — per the user's "if we have a headless part, we could call it
that"; whether the daemon additionally brands as `havocd` is deferred to stage 4's
daemon-modes item. Applied now, before any code exists, across all working docs and
the boundary/allowlist checks; NORTH-STAR.md amended by dated appendix per its own
rule. The "The name" Still-open item is deleted; stage 6's rename item collapsed into
plain "Release".
**Over:** keeping `havoc` as a placeholder until the stage-6 rename the plan carried,
or renaming every crate to the `supernaut-` prefix.
**Because:** pre-code, the rename is a docs-only diff; post-code it touches binary
names, socket paths, config dirs, and the crate prefix (exactly what the Still-open
item warned). The split prefix is semantic, not cosmetic: `supernaut` names what the
user touches, `havoc` names the engine the §4.2 boundary protects — the ipc and
transport crates sit on the engine side of that boundary, so they keep its name.
**Revisit if:** the stage-4 daemon work finds the two-name split confusing in
`--help`/docs; the fallback is `supernaut`-everything, which stays a mechanical
rename of two crate names while the engine crates are pre-1.0.

## Decision — README grows the checked progress apparatus, features, and roadmap
**Date:** 2026-08-09  **Affects:** README.md, every future prompt PR (badge + table upkeep)

**Chose:** opt in to the template's README progress apparatus — the stage badge and
per-prompt progress table that check 6 verifies against the queue's status line on
every commit — plus a features section (framed explicitly as design goals; nothing
runs yet) and a stages overview of one-liners pointing at PLAN.md as authoritative.
This partially supersedes the bootstrap decision that kept the README badge-less:
that reasoning was "a private working repo does not earn the per-prompt upkeep", and
the repo has since gone public at the user's direction — the public front door is
exactly the case the template says earns it. Every prompt PR now also bumps the badge
and flips a table row, or CI fails; that friction is the point.
**Over:** a prose-only roadmap copy of PLAN.md's stages (drifts, and the copy nobody
edits is the one that gets read), or linking to PLAN.md with no summary at all
(honest but useless as a front door).
**Because:** the only roadmap the README can carry honestly is one a machine keeps
honest; everything unchecked in it (features, ethos) is deliberately timeless prose
that does not go stale per-prompt.
**Revisit if:** the badge/table upkeep is repeatedly forgotten and CI-caught —
that is the signal to automate the bump in a hook rather than to remove the claims.

## Prompt 1 — Workspace scaffold and build discipline

**Commit:** PR #4 (squash)  **Date:** 2026-08-09

**Shipped:** the five-crate cargo workspace with the §4.2 edges declared, workspace
lints (`warnings = "deny"`, clippy all-deny), real Makefile bodies, `/target`
ignored, and an observable `supernaut` binary printing name + version. CI's build job
un-skips itself from this PR on (its guard keyed on the placeholder text).

**Deviations:** `make fmt` formats in place rather than the prompt's literal
`--check` mapping; the `--check` moved into `make lint`, which CI actually runs
(`make build test lint`) — the prompt's mapping would have left rustfmt unenforced in
CI. Crate doc comments run three lines, not one — the §4.2 pointer plus the naming
amendment does not fit one honest line. Rust toolchain installed on this machine via
Homebrew rustup (stable 1.97.1) as part of this prompt.

**Deferred:** licensing — the review caught `license = "MIT OR Apache-2.0"` landing
as a silent decision in workspace metadata; stripped, now a Still-open item on
PLAN.md aimed at stage 6 Release, where the field and LICENSE texts land together.

**Learned:** `[workspace.lints.rust] warnings = "deny"` plus per-crate
`[lints] workspace = true` puts warnings-as-errors in the repo rather than the shell,
which survives any CI or contributor environment; and `unused_crate_dependencies` is
allow-by-default, so empty placeholder edges compile without underscore-import
ceremony — the first draft's `use havoc_core as _;` lines were cargo-cult and the
review flagged them.

**Measured:** clean `make build` of the empty workspace: 0.8s. 313-line diff before
review fixes.

**Live run:** `cargo run -p supernaut` prints `supernaut 0.1.0` and exits 0; `make
build`, `make test`, `make lint` (fmt --check + clippy -D warnings), and `make check`
all green locally on stable 1.97.1. CI green is confirmed on the PR itself.

**Review:** three fence violations found and fixed — a personality tagline in
`main.rs` (the prompt's own fence: personality done twice is done badly), `pub use
havoc_ipc as ipc` aliases in three crates (API surface beyond "minimal lib.rs", and
they would have let call sites blur which crate owns the wire vocabulary), and the
unasked-for `license` field (stripped; see Deferred). Kept over the reviewer's
objection: `[workspace.lints.clippy] all = "deny"` (standing clippy-strict rule in
CLAUDE.md; the Makefile's `-D warnings` is deliberate belt-and-suspenders), and the
three-line doc comments (recorded as deviation instead). Adopted: the missing
prompt-outcome block the queue file's tail comment prescribes — now appended under
prompt 1.

**Carry-forward raised:** on prompt 4 — the line-transport trait cannot live in
havoc-transport (core has no edge to it; define the trait in havoc-core or
havoc-ipc). On prompt 5 — no core↔transport Cargo edge exists in either direction;
wire through the binary or add the edge deliberately with the boundary-check
amendment. Both from the review harvest. Rejected from the harvest: an alias-path
warning for prompt 2 (moot once the aliases were deleted) and a license-texts note
for stage 6 (superseded by the Still-open item).

## Decision — buffer identity: two buffers, merged view is a projection
**Date:** 2026-08-09  **Affects:** prompt 3 (schema), PLAN.md Still open

**Chose:** `#rust` on two networks is two buffers. The §4.9 schema stands as
sketched — `UNIQUE(network_id, name)` — and any future merged view is a
client/query-side projection over matching buffer names, never a storage-level
merge, so the schema stays untouched and this question never reopens.
**Over:** a cross-network buffer entity.
**Because:** networks are distinct protocol and trust domains — nick lists, modes,
accounts, and `msgid` uniqueness are all per-network, and per-buffer `seq` ordering
plus the `(buffer_id, msgid)` dedup index only stay coherent if a buffer belongs to
exactly one network. A projection delivers the merged-view feature request without
the ordering hazard. This was NORTH-STAR §9's lean; now it is settled.

## Decision — migrations: hand-rolled versioned SQL over refinery
**Date:** 2026-08-09  **Affects:** prompt 3 (storage), PLAN.md Still open

**Chose:** a hand-rolled migration runner keyed on `PRAGMA user_version`: numbered
SQL files embedded via `include_str!`, each applied in its own transaction in order
at startup, immutable once merged.
**Over:** the `refinery` crate.
**Because:** the dependency policy treats every dependency as surface, and the
runner this project needs is under a hundred lines against an embedded database
whose lifecycle we fully own — refinery brings macros and its own bookkeeping table
to solve exactly what `user_version` already provides. NORTH-STAR §4.9 left this as
an explicit either-or; the project's own dependency ethos decides it.
**Revisit if:** migrations ever need Rust-code steps (data rewrites beyond SQL) or
multi-writer checksum discipline — those are refinery's actual value.

## Decision — havoc-ipc dependencies: serde runtime, ciborium dev-only, no time crate
**Date:** 2026-08-09  **Affects:** crates/havoc-ipc/Cargo.toml, prompt 2

**Chose:** `serde` (derive) as havoc-ipc's only runtime dependency; `ciborium` as a
dev-dependency for the round-trip tests; **no `time` crate** — `ServerTime` wraps a
unix-milliseconds `i64` and implements no ordering.
**Over:** the prompt's sketch of "serde (derive) and a time type" as runtime deps.
**Because:** the wire needs a timestamp *value*, not calendar arithmetic; owning the
integer keeps the wire crate at one runtime dependency and makes the no-ordering
rule structural rather than conventional. The `time` crate enters the workspace when
something actually formats timestamps for display (supernaut-tui, stage 2), with its
own decision entry. The allowlist already carries serde/ciborium/time from the
bootstrap seeding, so no allowlist edit accompanies this.

## Prompt 2 — IPC wire types

**Commit:** PR #5 (squash)  **Date:** 2026-08-09

**Shipped:** havoc-ipc's stage-1 surface — newtype IDs, `ServerTime` with no
ordering, `Request`/`Response`/`Event`, `Anchor`, `MessageKind`, buffer/network
models, `PROTOCOL_VERSION`, the empty `caps` module — with CBOR round-trip tests for
every variant and the unknown-field-tolerance proof. Plus a check fix the prompt
surfaced: the ipc dependency cap now counts runtime and build deps but not dev-deps
(`runtime_deps_of` in check-docs.sh, three fixtures).

**Deviations:** no `time` crate (decision entry: `ServerTime` owns a unix-millis
i64); `Request`/`Response` are id+body wrapper structs rather than the order's
literal "enum carrying a RequestId" — wire-equivalent, one shape for correlation;
one combined dependency decision entry rather than one per crate name — it is one
dependency story. The two prompt-3 blockers were settled here (buffer identity,
migrations) because the status line cannot name a blocked prompt as next — the
mechanism working exactly as designed.

**Deferred:** None.

**Learned:** serde's default unknown-field tolerance is the whole CBOR evolution
story for structs, but enum variants are decode errors — the capability handshake
must gate variants, now recorded on stage 4's plan item. And the first draft of the
dev-deps check fix silently exempted [build-dependencies], which do compile on user
machines — the reviewer caught it; the cap now counts them, with a fixture.

**Measured:** havoc-ipc runtime tree: serde only (cargo tree -e normal). Six
round-trip tests, 767-line diff pre-review.

**Live run:** N/A — types only; nothing has behavior to observe. `make build`,
`make test` (6/6 in havoc-ipc), `make lint`, `make check`, and `./scripts/
test-checks.sh` (38 fixtures) all green locally; CI green on the PR.

**Review:** adopted — the `time` gap between decision and check (IPC_ALLOWLIST
tightened to serde/bitflags, fixture updated), the build-deps exemption (counted
now, fail fixture added), the missing prompt-outcome block (appended), and the
prompt-3 bullet rewrite so the session executes the recorded decisions instead of
re-deciding. Rejected: dropping Ord from NetworkId/BufferId (they are map keys and
list-sort keys; the §4.6 hazard is time-ordering of *messages*, which ServerTime's
shape forecloses); splitting the deps decision entry (one story, three names);
package-rename evasion of the allowlist (known limitation, recorded at bootstrap
with its revisit trigger); "fixture coverage one-sided" (the pre-existing
tokio-in-[dependencies] fail fixture exercises the strict side of the same
function — not visible in the diff the reviewer had).

**Carry-forward consumed:** none — no notes were attached to prompt 2 (the one
proposed against it at prompt 1 was rejected as moot when the aliases were
deleted).

**Carry-forward raised:** eight, all from the review harvest, all adopted: prompt 3
(schema units/discriminants fixed by havoc-ipc), prompt 4 (ConnectionPhase is a
3-variant projection), prompt 5 (SearchResults-on-broadcast leak; Ack is
fire-and-forget), prompt 7 (msgid has no berth on wire Message — ingest type
needed), prompt 8 (search wire shape frozen), prompt 9 (Backlog Vec has no
has-more/hit-marker), stage 2 item 3 (time-crate debt), stage 4 item 2 (variant
intolerance). Rejected from the harvest: none.

## Decision — SASL for stage 1: PLAIN over TLS, mechanism slot shaped for EXTERNAL
**Date:** 2026-08-10  **Affects:** prompt 4 (state machine), PLAN.md Still open, stage 6 item 2

**Chose:** stage 1 implements exactly one SASL mechanism — PLAIN, over TLS only —
but the state machine's mechanism selection is an ordered preference list with
per-mechanism states, so EXTERNAL (CertFP) is a new list entry later, not a
reshaping. EXTERNAL/CertFP lands as a stage 6 menu candidate, where client-cert
plumbing (rustls client auth + keyring storage) exists to support it.
**Over:** implementing PLAIN + EXTERNAL both in stage 1.
**Because:** the question's real worry was the *shape* — a machine designed around
one mechanism hardcodes it. Designing the shape for many costs a list and an enum;
implementing EXTERNAL costs client-cert configuration, storage, and UX that stage 1
has nowhere to put (config and keyring arrive in prompt 10; TLS in prompt 6). §2.3's
"CertFP supported" is a promise about the product, not about M1 — now scheduled
instead of implied. This also keeps EXTERNAL out of prompt 6's scope, per that
prompt's own revisit clause.
**Revisit if:** a target network requires EXTERNAL for registration during the
dogfood month — that pulls it forward from the menu.

## Decision — rusqlite with the bundled feature into havoc-core
**Date:** 2026-08-10  **Affects:** crates/havoc-core/Cargo.toml, prompt 3

**Chose:** `rusqlite` (0.40, `bundled`) as havoc-core's storage engine dependency.
**Over:** linking the system SQLite (no `bundled`).
**Because:** rusqlite itself is the NORTH-STAR §5.1 decision — the entry here is
for the *feature*: `bundled` compiles a pinned SQLite into the binary, so every
build on every machine and CI runner exercises the same SQLite version, and the
FTS5 module (prompt 8) is guaranteed present rather than dependent on the host
library's compile flags. Cost: the C build NORTH-STAR already accepts knowingly,
plus slower cold builds (~20s once per toolchain).
**Revisit if:** distribution packaging ever demands dynamic linking against a
distro SQLite.

## Prompt 3 — Storage schema and migrations

**Commit:** PR #6 (squash)  **Date:** 2026-08-10

**Shipped:** havoc-core's storage layer — the §4.9 schema via hand-rolled
`user_version` migrations (per the recorded decision), WAL + foreign keys, the
dedicated storage thread behind a job channel, and the supernaut binary opening the
store at startup with `--data-dir` / XDG default and printing migrate-vs-up-to-date.

**Deviations:** replies use fresh single-use `std::sync::mpsc` channels — std has no
oneshot; semantics identical. Migration runs on the caller's thread before the
connection moves to the storage thread, so failures surface before startup
continues. The smoke insert/read surface is `#[cfg(test)]`-gated rather than
shipped. `--data-dir` is hand-parsed (one flag; an arg-parsing dependency is not yet
justified — revisited at prompt 5 via carry-forward).

**Deferred:** None.

**Learned:** `message` being WITHOUT ROWID means FTS5's external-content pattern
(`content_rowid`) cannot apply as sketched — caught by the review, now a
carry-forward on prompt 8 where it changes the design, not a mid-session surprise.
Also: sqlite's `sqlite_autoindex_*` objects appear for UNIQUE constraints; the
schema test filters `sqlite_%` deliberately.

**Measured:** rusqlite bundled adds ~20s to a cold build (SQLite C compile);
incremental builds unaffected (0.5s).

**Live run:** `supernaut --data-dir <fresh>` twice: first prints `schema v1,
migrated v0 -> v1`, second prints `schema v1, up-to-date`; `sqlite3 .schema message`
shows the §4.9 shape (WITHOUT ROWID, partial unique msgid index, millis time
index). All 11 test targets green; lint clean.

**Review:** adopted — the schema-shape test was a silhouette (names + PK only) and
now asserts columns, both index column lists, and the `WHERE msgid IS NOT NULL`
predicate; the premature `synchronous = NORMAL` pragma was removed (prompt 7's
flood harness owns fsync tuning; a comment marks the seam). Rejected:
gating `ensure_network`/`ensure_buffer` behind cfg(test) (they are the deliberate
buffer-creation seam and the integration tests exercise them; their conflict
semantics get re-reviewed at prompt 7 via the note raised there);
removing `StorageError::FutureSchema` (refusing to touch a newer schema protects
the user's irreplaceable archive — fail fast); the claimed missing rusqlite
allowlist edit (rusqlite has been on `DEP_ALLOWLIST` since the bootstrap seeding —
not visible in the reviewer's diff; local + CI `make check` prove it).

**Carry-forward consumed:** both prompt-3 notes — the schema derives its units and
discriminants from havoc-ipc (`server_time` in millis; `kind_code`/`buffer_kind_str`
as deliberate pinned mappings with a stability test), and the two pre-settled
decisions were executed, not re-decided (no refinery; `UNIQUE(network_id, name)`).

**Carry-forward raised:** seven, all from the review harvest, all adopted:
prompt 5 (main's closed arg grammar + the arg-parsing dependency decision),
prompt 7 (ensure_buffer discards kind on conflict; the Storage handle blocks the
calling thread — decide the async bridge; tags-as-CBOR needs ciborium in
havoc-core), prompt 8 (WITHOUT ROWID vs FTS5 external-content; search queues
behind the single write FIFO), stage 6 item 1 (last_read_seq is single-marker disk
shape). Rejected from the harvest: none.

## Decision — irc-proto for Message parse/serialize only
**Date:** 2026-08-10  **Affects:** crates/havoc-core/Cargo.toml, prompt 4

**Chose:** `irc-proto` 1.1 as havoc-core's parser dependency — `Message`,
`Command`, `CapSubCommand`, `Response` as plain data types. The connection state
machine, cap negotiation ordering, SASL, and reconnect stay ours (NORTH-STAR §5.2:
that is the product). Base64 for the SASL PLAIN payload is sixteen hand-rolled,
RFC-vector-tested lines rather than a base64 crate — encode-only, one call site.
**Over:** the `irc` crate's `Client` half (fuses connection/config/stream — §5.2
rejected it); writing the tag/prefix/trailing parser ourselves (§5.2: years of
real-server exposure for a plain data type is not worth redoing); a base64 crate
(a dependency for sixteen lines).
**Because:** already argued in NORTH-STAR §5.2; this entry records the version and
the base64 sub-choice. Vendoring remains the escape hatch if irc-proto stagnates.
**Revisit if:** SASL ever needs base64 *decoding* (server challenges beyond "+") —
that is the moment the crate earns its place.

## Prompt 4 — Connection state machine, offline

**Commit:** PR #7 (squash)  **Date:** 2026-08-10

**Shipped:** the connection state machine in havoc-core — CAP LS 302 through
steady state with the §6.8 cap rules, SASL PLAIN (fail-closed), ISUPPORT parsing,
autojoin, PING/PONG, the named reconnect seam, and the `Networks` actor map — with
an eleven-transcript table-driven test corpus plus RFC-vector base64 tests.

**Deviations:** built **sans-I/O instead of trait-plus-scripted-fake**: the machine
is a pure lines-in/lines-out function (`Machine::handle_line`) and the transcript
tables are the scripted transport. The trait the order named was not built — the
actor loop and transport seam move wholesale to prompt 5, which now carries the
restated prompt-1 edge constraint. This is the strongest possible protocol/I-O
separation, but it is a different architecture than ordered and is recorded here as
such. Fail-closed SASL (`State::Failed` on 904/denial/not-offered when SASL is
configured) is added behavior the order's "tolerate any subset being denied" did
not carve out — NORTH-STAR §2.3's loud-opt-in-to-insecurity ethos decides it:
silently proceeding unauthenticated after the user configured SASL is the trap.
SASL is one explicit state, not per-mechanism states — with exactly one mechanism
implemented, per-mechanism states would be untestable structure; the honest record
is the stage-6 note that EXTERNAL requires mechanism iteration.

**Deferred:** None.

**Learned:** irc-proto 1.1 drags tokio/tokio-util/bytes into the tree transitively
— the async runtime is in Cargo.lock two prompts before we use it; harmless but
worth knowing when reading `cargo tree`. irc-proto's CAP variant packs the
continuation marker and cap list into two trailing Options — the `(Some("*"),
Some(caps))` shape is the multiline marker. And the reviewer caught a real §6.8
violation: CAP NEW arriving mid-negotiation produced a REQ that did not gate CAP
END — "every requested cap" has no timing exception; fixed with its transcript.

**Measured:** 22 tests across the workspace; the machine + caps modules are 262 and
280 lines against the 400-line ratchet.

**Live run:** N/A — no I/O exists by design; prompt 5 is where this machine first
touches a socket (the prompt block pre-declares this).

**Review:** adopted — the CAP NEW mid-negotiation gate bug (fixed + transcript);
the out-of-order transcript now runs through 001/376 and asserts it *ends
registered*, matching the acceptance literally; `SaslCredentials` got a manual
redacting `Debug` so the password cannot print through `{:?}` of a machine or an
actor log (fixed now rather than deferred to prompt 10 — a known secret leak does
not age well). Rejected: the missing irc-proto allowlist edit (irc-proto has been
on `DEP_ALLOWLIST` since bootstrap; local + CI `make check` prove it); dropping
nick tracking (ISUPPORT's "parsed into network state" covers nick state, and
prompt 6's reconnect needs the confirmed nick).

**Carry-forward consumed:** both prompt-4 notes — the trait-placement constraint
(honored by placing the machine in havoc-core with no havoc-transport edge;
restated onto prompt 5 with the deviation) and the ConnectionPhase projection
(`Machine::phase()` maps ~8 real states onto the 3-variant wire enum; the real
enum stays core-private).

**Carry-forward raised:** six, all from the review harvest, all adopted: prompt 5
(no actor/trait exists — build wholesale under the restated edge constraint;
handle_line returns lines only — decide the event-surfacing shape), prompt 6
(inline-Rust corpus vs fixture files — pick before capturing; phase() folds
Failed/Disconnected — backoff must read state() and never retry Failed, no reset()
path), prompt 7 (the machine discards non-protocol messages and parses inside
handle_line — decide the parse-once seam), stage 6 item 2 (the mechanism slot is
shape-only; EXTERNAL needs iteration + 908). Rejected from the harvest: the
prompt-10 credentials-Debug note — fixed in this PR instead, moot.
**Oversize:** 832 changed lines in crates/ against the 800 cap — 290 of them are
the transcript test corpus, which the prompt names as "the deliverable as much as
the code", and the split candidate (tests from machine) would sever the corpus
from the rules it pins. The mechanical bulk is tables, reviewed line by line.

## Decision — tokio lands, with explicit feature lists per crate
**Date:** 2026-08-10  **Affects:** havoc-core, havoc-transport, supernaut Cargo.tomls

**Chose:** tokio (NORTH-STAR §5.5's settled runtime, on the allowlist since
bootstrap) as a direct dependency where prompt 5 needs it: havoc-core
(sync,net,time,rt,io-util,macros — actors, channels, TCP), havoc-transport (sync —
channels only), supernaut (rt-multi-thread,macros,io-std,io-util,time,sync — the
runtime and stdin). Feature lists are explicit, never "full".
**Over:** `features = ["full"]` (pulls the process/signal/fs kitchen sink into
every build), or deferring the runtime again (prompt 5 is the actor prompt; there
is nothing left to defer to).
**Because:** §5.5 already argued tokio vs alternatives; this entry records the
arrival and the feature discipline. It was already in the tree transitively via
irc-proto since prompt 4 — now it is deliberate.

## Decision — clap for the CLI grammar
**Date:** 2026-08-10  **Affects:** crates/supernaut/Cargo.toml, scripts/check-docs.sh (allowlist)

**Chose:** `clap` (derive) in supernaut only, added to DEP_ALLOWLIST.
**Over:** extending prompt 3's hand-rolled parser.
**Because:** that parser's own recorded rationale — "the surface is one flag" —
expired the moment `session` landed with seven flags, and prompts 8/9/10 each add
verbs to this same grammar. The binary is the one crate where a CLI dependency
belongs; nothing else may grow one.
**Revisit if:** the grammar is still ~one subcommand at prompt 10 — then note in
that entry that this was premature.

## Prompt 5 — Event bus, request handler, and debug CLI

**Commit:** PR #8 (squash)  **Date:** 2026-08-10

**Shipped:** the wired core — two-lane bus (broadcast + per-session directed, with
the structural debug_assert and leak test), request dispatch with per-session
correlation, the IRC line-transport trait + TCP impl in havoc-core, the actor task
owning Machine and transport, `Networks` re-typed to actor handles, havoc-transport's
`ClientTransport` + `InProcess`, the binary's wiring adapter, the `session` debug CLI
with event-driven verbs and deterministic `wait`s, `--trace-irc` capture, and
`scripts/live-run.sh` with a pinned, sha-verified ergo. `ConnectionState` gained
`detail` (the one authorized wire change; roundtrip-tested both ways).

**Deviations:** `Bus::direct` carries `Directed` (responses + correlated events) not
the ordered bare `Event` — responses need the same lane; `recv()` returns
`Result<Incoming, TransportError>` not `Option` — the order's signature could not
carry the loud-lag it demanded elsewhere; `join` is fire-and-forget with
`wait buffer` as the completion verb (the module doc initially claimed otherwise —
review caught it, doc fixed); `LineTransport` has no `close` (drop is teardown);
`is_loopback` is a strict string check, refusing more than the ordered "resolves to
loopback", never less; tokio feature lists exceed the order where the code needs it
(`macros` powers `select!`, not just tests — reviewer's dev-dep suggestion rejected
on that ground); the session gained `--data-dir` (storage must open somewhere);
caller-assigned wire NetworkIds with a core-private row-id mapping (documented in
core.rs — the alternative leaks storage identity onto the wire); live-run.sh polls
twice with sleep where no event exists yet to wait on (prompt 7's note deletes them)
and keeps its guarded `rm -rf` on a mktemp dir as the one sanctioned use.

**Deferred:** None.

**Learned:** ergo has no Homebrew formula — the pinned-download path in live-run.sh
is the real path, verified against the release checksums; a minimal ergo yaml needs
accounts/limits/history sections present even when unused. `Storage` grew a
clonable `StorageClient` for `spawn_blocking`; the review's "replace, don't
deprecate" catch removed the mirrored delegators — `client()` is the one route.

**Measured:** live run end-to-end (fetch ergo, boot, two sessions, six assertions):
~8s warm. 2323-line diff pre-review (mostly Cargo.lock + the JIT detail itself).

**Live run:** passed, first try, all six assertions: A saw
connecting→registered→buffer-created, the trace captured `>> CAP LS 302`, B's
PRIVMSG landed in A's capture, B registered. Re-ran after review fixes: passed.
`supernaut --data-dir <tmp>` with no subcommand still prints name/version + history
and exits 0 (clap now also answers --help/--version — grammar superset, invocation
behavior preserved).

**Review:** adopted — the false `join` doc claim (fixed), the duplicated Storage
delegators (removed; callers go through `client()`), the free-port check and
guarded cleanup in live-run.sh, and all eleven carry-forward proposals. Rejected:
`close` on the trait (drop-based teardown; add it when something needs early
close), dev-depping tokio `macros` (select! is runtime code), trimming the ergo
config's history/auth sections (stage 5 and prompt 6 use them; regenerating the
yaml per prompt is churn), and DNS-resolving loopback enforcement (strict strings
refuse a superset).

**Carry-forward consumed:** all six notes — no-edge constraint (adapter in the
binary; no core↔transport edge shipped), SearchResults leak (two-lane bus +
debug_assert + test), Ack semantics (event-driven CLI verbs), no-actor/no-trait
(built wholesale: io.rs trait in havoc-core, actor.rs task), handle_line shape
(not widened; actor diffs state() and fills detail from Failed), arg-parsing
decision (clap, with entry + allowlist).

**Carry-forward raised:** eleven, all from the review harvest, all adopted:
prompt 6 (backoff must live inside run() — the distinction never leaves the actor;
trace capture is stderr-multiplexed — specify the filter; live-run.sh is
one-machine), prompt 7 (our_join is the parse-twice outcome to subsume;
handle_report serializes storage through the select loop; two id spaces share one
type; the sleep polls await your event — grow `wait message`), prompt 8 (dispatch
discards ClientId before the handler — plan the signature), prompt 10 (hardcoded
NetworkId(1) and debug-<host> naming), stage 2 item 1 (duplicated, cross-lane
unordered event stream), stage 4 item 1 (the merge and Lagged have no wire story).
Rejected from the harvest: none.
**Oversize:** 1328 changed lines in crates/ against the 800 cap. The wiring seam is
one prompt by design ("Examined for a split and left whole" — bus, dispatch,
transport trait, actor, and CLI are not testable in anger apart), and 400 of the
lines are the ordered JIT prompt detail plus tests. Splitting would have produced
two PRs that cannot each pass their own live run.

## Decision — the TLS trust story: webpki-roots plus one named anchor, no off-switch
**Date:** 2026-08-10  **Affects:** crates/havoc-core (rustls, tokio-rustls, webpki-roots, rustls-pki-types), scripts/check-docs.sh (allowlist), session flags

**Chose:** rustls (ring provider) over tokio-rustls; roots from webpki-roots;
`--tls-ca <pem>` appends one extra anchor for the local ergo cert with
verification staying ON. webpki-roots and rustls-pki-types join the allowlist
under this entry. There is deliberately no `--tls-insecure-skip-verify` and never
will be — you say what you trust, you never stop verifying (§2.3's loud-opt-in
shape). SASL passwords enter debug sessions via `SUPERNAUT_SASL_PASSWORD` env,
never argv (world-readable in ps) — a bridge superseded by prompt 10's keyring.
**Over:** rustls-platform-verifier / native certs (OS-integration code and
per-machine nondeterminism for zero stage-1 benefit); a skip-verify flag (the
trap, permanently declined); aws-lc default provider (ring avoids a C/asm
toolchain wobble on this Mac).
**Revisit if:** an enterprise-CA user appears (platform verifier earns its keep)
or rustls deprecates the ring provider.

## Prompt 6 — Live connection, TLS, and reconnect

**Commit:** PR #9 (squash)  **Date:** 2026-08-10

**Shipped:** TLS as the default path (TlsLineTransport + AnyLineTransport enum in
io.rs, Security plumbed through NetworkSettings/ActorSpawn), live SASL against
ergo, reconnect via an attempt loop inside actor run() — fresh Machine per
attempt, exponential 1s→60s ±50% jitter, counter reset on registration, Failed
never retried — plus scripts/trace-to-steps.sh, the live-captured
live_ergo_registration corpus test, two paused-time reconnect tests, and the
extended live-run.sh (TLS listener, NickServ pre-registration, kill-and-restart
reconnect proof). PLAN's testing-strategy wording amended (inline Step tables,
converter — the "fixture files" promise was stale).

**Deviations:** ergo's mkcerts replaced with an openssl cert for the dialed name
(ergo rejects undotted server names, so mkcerts would mint the wrong CN — the
detail's own named fallback); the 903 assertion matches ergo's `*`-nick reply;
Connecting retry detail is "retry <n>" without the delay (the delay is jittered
after the report; naming it would lie by up to 50%).

**Deferred:** None.

**Learned:** ergo requires a dotted server.name; its SASL numerics use `*` for
the nick pre-registration; openssl self-signed certs default to CA:TRUE which
webpki rejects as an end-entity (CaUsedAsEndEntity) — basicConstraints CA:FALSE
required. rustls built with the ring provider avoids the aws-lc build dep.

**Measured:** full live run (fetch cached, boot, pre-register, TLS+SASL, message,
kill+restart, re-register): ~15s. Backoff observed live: retry 1..4 while ergo
was down, then re-registered.

**Live run:** all eleven assertions passed — TLS registration with SASL 903, B's
message landing, and the invisible-reconnect sequence
(disconnected → retry → registered again) with no operator action. Libera.Chat
spot-check: registered over TLS with stock webpki roots
(`session --host irc.libera.chat --port 6697`); SASL exercised against ergo only
(no registered Libera account) — recorded honestly. Killing ergo mid-session
prints one disconnected line per attempt with backoff between, as designed.

**Review:** ran below; dispositions recorded there.

**Carry-forward consumed:** all five notes — backoff written inside run() reading
state() (Failed → Fatal, never retried; no reset(), fresh Machine per attempt);
trace filter specified in trace-to-steps.sh's header (the `>> ` user-command
caveat included); live-run.sh stays scoped to this Mac with the pinned platform
(recorded, not fixed — CI coverage is the trigger); corpus format decided
(inline Step tables; PLAN amended); phase()-folding respected — the retry loop
never consumes reports.

**Carry-forward raised:** see Review.

## Correction — the TLS decision entry misstates its own alternative
**Date:** 2026-08-10  **Supersedes:** "Decision — the TLS trust story" (2026-08-10)
**Category:** stale-doc-copy

**Claimed:** "rustls (ring provider) over tokio-rustls" — as written, the entry
rejects the very crate its Affects line ships.
**Actually:** the choice was ring **over aws-lc** as rustls's crypto provider,
*via* tokio-rustls, which is and remains the shipped async TLS layer.
**Lesson:** a decision entry's Chose/Over pair is load-bearing; a misplaced word
inverts it, and append-only means it stands until corrected loudly.

**Not mechanizable because:** no check can read intent behind prose in a decision
entry; the guard is the post-prompt review, which is what caught it.

Prompt 6 Review addendum (dispositions): fixed now, per the review — the backoff
jitter modulus (subsec_nanos()%1000 is degenerately zero on microsecond-granular
macOS clocks; now derived from subsec_micros()), the unjustified `logging` feature
on rustls/tokio-rustls (dropped; `tls12` stays — old ircds are real, and the
compat choice is now stated here), and the Correction entry above. Recorded as
deviation, previously missing: the acceptance's "wrong-password test" shipped as
the cap-NAK fail-closed variant instead — same Failed terminality through a
different arm; the NAK arm covers the retry-policy contract this prompt owed and
a 904-path live exchange can join the corpus from a future capture. The
converter's silent dropping of pre-first-`<<` client lines is now noted here
(openings are asserted from Machine::start directly). Carry-forward raised, all
adopted: prompt 7 (re-registration side effects replay; the sleep-poll note
amended to the four-loop reality), prompt 9 (commands dropped while disconnected
have no signal), prompt 10 (env/flag installed base must be replaced, not
extended), stage 5 item 2 (resync must be core-driven off the second
Registered). Rejected from the harvest: none.

Prompt 6 addendum 2 — a CI-only discipline failure surfaced a portability bug in
check-docs.sh itself: pipe-fed `grep -q` exits at first match, and under
`set -euo pipefail` GNU sed upstream dies of SIGPIPE, failing the pipeline
*because* the match succeeded (BSD sed on the dogfood Mac never surfaced it —
test-checks.sh's own expect() comment described this exact class). All eight
pipe-fed `grep -q` sites now read to EOF (`grep ... >/dev/null`); file-arg greps
keep `-q`. The fixture suite (38) covers the rules' semantics unchanged; the
SIGPIPE trigger itself is GNU-environment-only and is proven by this PR's own CI
run going green.

## Decision — ingestion identity and the parse-once seam
**Date:** 2026-08-10  **Affects:** havoc-core (connection/ingest.rs, storage), ciborium into havoc-core

**Chose:** the actor parses each line exactly once and shares the parse between
`Machine::handle_message` (now pub; `handle_line` stays as the corpus's wrapper)
and the new classifier — `our_join`/`JoinedChannel` deleted, buffer creation rides
ingestion. The ingest type is core-private (wire `Message` keeps no msgid berth —
dedup is storage's business). Identity lives entirely in the storage layer: seq
from per-writer-thread counters, dedup by the partial unique index with the
conflict target spelling out `WHERE msgid IS NOT NULL`, and tagless lines get a
synthetic `fnv:<hex>` msgid (inline FNV-1a 64 — disk format must be stable across
releases, which rules out DefaultHasher; sha2 fails the dependency bar) over
(nick, text, 30s bucket) stored in the same column so one index enforces both
identities. tags land as CBOR (ciborium moves to havoc-core runtime deps; already
on the allowlist). server-time is a hand-rolled parser for the one pinned IRCv3
grammar. Own messages are recorded via echo-message only.
**Over:** widening `handle_line` to carry messages (churns the entire transcript
corpus); amending the wire type with msgid; a time/sha2/rand-class dependency per
small need; a second parse per line (what prompt 5 shipped and this deletes).
**Revisit if:** stage 5's bouncer work shows the 30s-bucket collapse biting real
replay traffic, or a dogfood network without echo-message makes own-send logging
urgent.

## Decision — synchronous=NORMAL, measured
**Date:** 2026-08-10  **Affects:** crates/havoc-core/src/storage/mod.rs

**Chose:** `PRAGMA synchronous=NORMAL` under WAL. Flood measurement (500 lines,
live ergo, this Mac): 8 commits at NORMAL, 8 commits at FULL, ~6s wall each —
indistinguishable at this scale; the win is batching itself (8 commits vs 500
potential fsyncs). NORMAL keeps committed data across an app crash and concedes
only the power-loss tail — the right trade for chat history.
**Over:** FULL (no measured benefit here, more fsync on slower disks later).
**Revisit if:** dogfood on slower storage shows checkpoint stalls.

## Prompt 7 — Message ingestion, identity, and batched writes

**Commit:** PR #10 (squash)  **Date:** 2026-08-10

**Shipped:** the write path — parse-once actor seam feeding the classifier
(connection/ingest.rs, hand-rolled server-time parser), core-private Ingest type,
the no-await ingest lane (actor → core report → storage job queue, plain send),
batched transactions (256 rows / ~100ms window / flush-before-reads /
flush-at-shutdown), storage-layer identity (per-buffer seq counters, partial-index
dedup, FNV synthetic msgid), CBOR tags, NetworkRow newtype killing the id-space
swap class, BufferCreated idempotency across reconnects, the `wait message` verb,
and the flood segment in live-run.sh.

**Deviations:** the upsert's conflict target must spell the partial index's
predicate (`ON CONFLICT (buffer_id, msgid) WHERE msgid IS NOT NULL`) — SQLite
refuses the bare column pair; discovered by the module tests. QUIT/NICK are not
ingested (membership state doesn't exist; PLAN stage-2 note raised). The
content-hash fallback ships unit-tested only — ergo tags everything, so its live
proof honestly waits for stage 5's bouncer.

**Deferred:** None beyond the recorded skips above.

**Learned:** two harness lessons that are really product lessons. (1) `quit`/EOF
races in-flight requests: carol's flood lost ~300 sends to the runtime drop until
she drained via her own echo-message count — noted on prompt 9, where the verbs
should grow a real drain. (2) A dead session's fifo write SIGPIPEs bash itself,
which skips the EXIT trap and leaks ergo — the script now ignores PIPE so writes
fail as catchable errors. Also: ergo needs `fakelag: enabled: false` or a flood
trickles.

**Measured:** 500-line flood → 8 commits (vs 500 potential fsyncs), ~6s wall, at
both synchronous=FULL and NORMAL (decision entry above). 16/16 live assertions.
sqlite post-mortem: journal=wal, rows=500, distinct=500, maxseq=502.

**Live run:** all 16 assertions green — B's send arriving as MessageAdded, the
500-line flood counted by A and read back from disk after process death, exactly
one BufferCreated across the ergo restart, batching measured. Run three times
(NORMAL, FULL, NORMAL-final).

**Review:** ran below; dispositions recorded there.

**Carry-forward consumed:** all ten notes — msgid berth (core-private Ingest);
ensure_buffer kind conflict (deterministic target_kind + loud mismatch, never
re-kinded); sync Storage bridge (the no-await ingest lane; connect's single
spawn_blocking stays); tags CBOR (ciborium in, decision entry); parse-once seam
(decided: actor parses, machine handle_message pub); our_join subsumed (deleted);
handle_report serialization (fire-and-forget lane, zero awaits); id spaces
(NetworkRow newtype); sleep polls (event-shaped polls now sync on wait-verb
echoes; harness-level polls stay, commented); reconnect replay idempotency
(index-driven dedup + first-touch BufferCreated, asserted across a live restart).

**Carry-forward raised:** prompt 9 (quit races in-flight requests — drain or
document), PLAN stage 2 item 5 (QUIT/NICK need membership state). Both adopted.

Prompt 7 Review addendum (dispositions): fixed now — the rollback cache-poisoning
(a real correctness bug: phantom buffer ids and advanced seqs survived a failed
batch; caches now clear on rollback and reseed from disk), the silent ingest drop
on an unknown network (now loud), the server-time parser's missing lower bounds
and month-aware day validation (negatives and 02-31 now fall back), the harness's
weakened contiguity assert (now COUNT(*) == MAX(seq) over all kinds), the commits
bound (tightened to <50), and FLOOD_SECS measuring the wrong interval (moved
before carol's pipeline). Accepted as recorded imprecision: the "B's send arrived"
assert and `wait message` count both match any MessageAdded kind — the verbs are
rewritten at prompt 9, where the kind-aware note now lives. Carry-forward raised,
all four adopted: prompt 8 (rollback invalidation meets FTS sync; the second-
reader staleness window), prompt 9 (unannounced pre-existing buffers — decide the
attach replay; kind-aware counting + keep the script's contiguity check).
Rejected from the harvest: none.
**Oversize:** 1097 changed lines in crates/ against the 800 cap. Identity and
batching are one insert path by design ("Examined for a split and left whole" —
the flood harness that proves batching is the same one that proves dedup), and a
third of the lines are the storage-thread rewrite the batching required plus its
module tests. The split candidate (classifier vs writer) would ship an ingest
type with no writer to receive it.

## Decision — FTS: a plain self-contained table, trigger-synced, riding the job queue
**Date:** 2026-08-10  **Affects:** migration 0002, storage/exec.rs, search.rs, core dispatch

**Chose:** a plain FTS5 table (`text` indexed; `buffer_id`/`seq` unindexed) —
external-content cannot bind to the WITHOUT ROWID message table — synced by an
AFTER INSERT trigger so the index write is structurally part of the inserting
statement (dedup never fires it; rollback carries it), backfilled in the same
migration. Search executes on the storage thread's single connection behind the
flush barrier (read-your-writes holds); results return correlated on the directed
lane, Response first, event on success only. Stock bm25 order, 100-hit cap.
**Over:** contentless-delete FTS5 + docid map (a second table, an allocator, and a
three-way join to save text bytes — disk is the cheap resource, moving parts are
not); Rust-side index writes (forgettable by future insert paths); a second
read-only WAL connection (silently forfeits read-your-writes for an unmeasured
latency win).
**Revisit if:** dogfood shows search dwell behind write bursts, or retention's
delete migration arrives (it owes the DELETE trigger).

## Decision — the actor report lane is unbounded
**Date:** 2026-08-10  **Affects:** connection/actor.rs, core.rs

**Chose:** the actor→core report lane becomes an unbounded channel, with the
actor's select biased to reads. The flood harness exposed a genuine deadlock
cycle: core awaited the full (64) command lane while the actor awaited the full
(256) report lane — throughput stopped at ~430/500 and ergo sendq-killed the
client. The report lane carries history, and prompt 7 already established the
principle for the storage queue: history is never dropped (or deadlocked) for
backpressure; memory bounds are a dogfood question.
**Over:** try_send-and-drop (drops history), bigger bounded caps (moves the
deadlock threshold instead of removing the cycle), or unbounding the command lane
(commands are droppable; history is not — the asymmetry is the design).
**Revisit if:** dogfood memory profiles show the lane growing without bound under
real network pathology.

## Prompt 8 — Full-text search

**Commit:** PR #11 (squash)  **Date:** 2026-08-10

**Shipped:** migration 0002 (plain FTS5 table + AFTER INSERT trigger + backfill),
the core-side query grammar (search.rs: quote-aware scanner, from:/in:/after:/
before:), Job::Search riding the storage queue behind the flush barrier, deferred
correlated responses (handle_request → Option<Response>; Response-then-
SearchResults on the requester's directed lane only), the CLI search + wait search
verbs with per-hit lines, the live-run search segment, and — from the harness — the
unbounded report lane fixing a real deadlock (decision entries above).

**Deviations:** the read-your-writes proof waits for the fresh line's *echo*
(network) before searching with no further delay (storage) — the JIT text's
"no wait between" conflated the two hops; recorded, not fudged. The
sleep-sequenced NickServ pre-registration and harness-level polls stand as before.

**Deferred:** None.

**Learned:** the flood harness caught a genuine product deadlock (core↔actor
channel cycle — see the decision entry) that five earlier green runs had timed
around; and FTS5 treats bare hyphenated terms as column-filter syntax
(`xyzzy-quicksilver` → "no such column") — the error comes back cleanly as
designed, and stage 2's /search UX should quote bare terms containing operator
characters (note raised). ON CONFLICT DO NOTHING provably does not fire the
AFTER INSERT trigger (pinned by test). ergo sendq-kills a non-reading client at
96k, which is how the deadlock presented.

**Measured:** search over the 500-row corpus: <1s wall (recorded by the harness).
Four consecutive 24/24 live runs post-fix. 16 lib tests, 5 search-scanner units,
v1→v2 upgrade/backfill pinned.

**Live run:** 24/24 assertions, four consecutive runs — filters, phrase-vs-prefix,
read-your-writes through the flush barrier, malformed-MATCH error survival, and
the on-disk index proven from outside the process (500 rows MATCH 'flood').

**Review:** ran below; dispositions recorded there.

**Carry-forward consumed:** all five notes — WITHOUT ROWID vs external-content
(plain table, decision entry); rollback invalidation meets FTS (trigger rides the
transaction; mid-batch test); second-reader staleness (rejected — job-queue rider
keeps read-your-writes; barrier documented); frozen wire shape (core-side grammar,
zero wire changes); ClientId discard (handle_request now takes the client and
returns Option<Response>).

**Carry-forward raised:** see Review.

Prompt 8 Review addendum (dispositions): fixed now — a failed Search enqueue
returns an immediate Error instead of silently breaking exactly-one-Response; the
dead `state` parameter dropped from handle_search_outcome; the rollback test's
sleep replaced with a schema_version round-trip (the flush barrier as a
deterministic sync); the NULL-text claim now asserted against the index's actual
row count; the malformed-MATCH live assert anchored to a numbered error line.
Recorded, previously missing: the tokenize clause (`unicode61 remove_diacritics
2`) is NORTH-STAR §4.9's own sketch, adopted with it — it belongs in the FTS
decision entry and now does. Carry-forward raised, all adopted: prompt 9 (don't
copy the swallowed enqueue; the bounded-outcome-lane topology under bigger
payloads; wait counts events not responses), prompt 10 (`in:` unions across
network rows — scope it with config), PLAN stage 2 item 5 (quote bare FTS5
operator terms in /search). Rejected from the harvest: none.
**Oversize:** 868 changed lines in crates/ against the 800 cap — the index, the
grammar, the dispatch rework, and the deadlock fix are one seam ("Examined for a
split and left whole": the filter grammar and index shape must be designed
together), and ~250 lines are the ordered tests. The deadlock fix could not wait
for its own PR: the flood harness this prompt extends is what exposed it.

## Decision — prompts 9 and 10 split into 9a/9b/10a/10b
**Date:** 2026-08-10  **Affects:** STAGE-1-PROMPTS.md, scripts/check-docs.sh (STAGES), scripts/test-checks.sh, README.md, PLAN.md blocking annotation

**Chose:** split the two remaining stage-1 prompts along their real seams —
9a windowed backlog + the buffer-announcement decision (one read-path seam),
9b read markers + the accumulated verb/drain hardening (quit drain, kind-aware
counting, disconnected-command signalling), 10a the TOML config file with
deliberately zero credential surface, 10b keyring credentials (deleting the env
bridge) + the stage acceptance run. Labels, not renumbering — the status checker
counts headings by position, which is exactly what the 13a/13b convention exists
for. Stage total: 12. The seven notes that had piled onto prompt 9 distribute
four/three; prompt 10's three notes split config-half/credentials-half.
**Over:** leaving them whole (three of the last five PRs blew the 800-line
tripwire — the trend, not any single breach, is the signal the cap exists to
produce), or renumbering (churns every cross-reference for zero benefit).
**Because:** the original prompt-10 fence argued the split ships the
plaintext-password trap; the resolution is a config format that never holds or
references a password in any version — 10a keeps SASL on the env bridge, 10b
deletes it (replace, don't deprecate). The trap dies by construction, not by
prompt size.
**Revisit if:** 9a's announcement decision turns out to need marker state —
nothing currently suggests it.

**Carry-forward consumed:** none — the notes on prompts 9 and 10 were
redistributed verbatim onto 9a/9b/10a/10b, not consumed; this section satisfies
the mechanical pairing for the moved block headings.

## Correction — merged branches were never deleted, eleven times
**Date:** 2026-08-10  **Supersedes:** the working method's "squash-merge and delete the branch" as practiced in PRs #1–#11
**Category:** convention-not-checked

**Claimed:** every PR was squash-merged "and the branch deleted once CI is
green" (CLAUDE.md working method; asserted implicitly in every prompt entry).
**Actually:** all eleven branches survived, local and remote. `gh pr merge
--delete-branch` was run from inside each prompt's worktree; deleting the local
branch requires checking out main, which was held by the primary worktree, so gh
aborted its entire branch-cleanup step — including the remote half — and the
error was being filtered out of the merge output as noise. Removing a worktree
never deletes its branch, so nothing downstream caught it. All eleven were
verified merged (tree-identity at merge time, PR state) and deleted 2026-08-10.
**Lesson:** a cleanup that runs from inside the thing being cleaned up will
fail in ways the happy path never shows; and filtering an error message to
reduce noise is how a recurring failure becomes invisible.

**Not mechanizable because:** verifying remote-branch absence needs network and
gh state, which pre-commit checks must not require. The procedural fix is in
force instead: merges now delete the branch explicitly (`git push origin
--delete` + local `-D`) after exiting the worktree, from the repo root, and the
merge output is no longer filtered.

## Decision — handoff hardening before a context reset
**Date:** 2026-08-10  **Affects:** this entry; memory notes outside the repo

Audited what was true only in the working session rather than on disk. Three
gaps found and closed here — the rest of the session's knowledge was already in
the log, the queue notes, or the scripts themselves, which is the system
working.

### State at handoff

`main` at "Split prompts 9 and 10 into 9a/9b/10a/10b (#12)"; status **8/12,
next: prompt 9a** (JIT — its detail is unwritten, four carry-forward notes
attached). No open PRs, no branches besides main (local or remote), no
worktrees. Stage 1 remainder: 9a, 9b, 10a, 10b, then the stage acceptance run
(in 10b) and the forced retrospective — CLAUDE.md prune, docs audit, rule
review from discipline-stats.txt, cold-start drill — before stage 2 opens.
All eleven prior prompts/PRs live-verified; the live-run harness asserts 24
checks and runs only on this Mac (pinned ergo, macos-arm64).

### Things learned that are worth not relearning

- **An interrupted live-run leaves ergo/supernaut processes behind, and the
  next run then dies instantly with empty output** (all output before the
  assert block goes to $WORK files, so early death prints nothing). First
  move when the harness fails weirdly: `pkill -f 'ergo run'; pkill -f
  'supernaut session'`, then rerun with `KEEP_WORK=1` and read the kept
  $WORK artifacts.
- **Patch-script edits fail silently.** Python string-replace no-ops when the
  target drifted (rustfmt reflowed it, an earlier edit changed it) — this bit
  four times. Always grep the result after applying a patch script; never
  trust "applied" output.
- **My own recurring failure mode, from the register:** filtering command
  output to reduce noise is how the branch-deletion failure stayed invisible
  eleven times (Correction, 2026-08-10). Read unfiltered output at least once
  per new command shape.
- The reviewer/JIT sub-agent flow is fully specified in SUBAGENT-BRIEFS.md and
  the diff-capture convention (`git diff --cached --output=target/promptN.diff`)
  needs no session memory — noted here only to say so.

## Decision — the buffer set is announced at attach, never requested
**Date:** 2026-08-10  **Affects:** core.rs (attach arm, `announce`), storage `Job::ListBuffers`/`BufferRow`

**Chose:** on attach, core enqueues `Job::ListBuffers`, resolves each row's
network *name* back to the caller's `NetworkId` through `state.settings`, seeds
`state.buffers`, and replays one `Event::BufferCreated` per resolved buffer down
that one session's directed lane. Buffers on a network absent from config are
skipped. `RequestBody` grows nothing, so §4.7's "there is no give-me-the-buffer
request, ever" is not approached: enumerating buffers is §4.5's attach contract
in as many words, the set is bounded by human action rather than traffic, and it
carries no message text. `BufferCreated`'s meaning is restated — "this buffer
exists and you did not know it" — because renaming the variant is itself a wire
break.
**Over:** a `RequestBody::ListBuffers` (the fence, and it is the exact request
§4.7 names); a new `Event::BufferList` batching the replay (unknown *fields* are
serde-tolerant on this wire, unknown *variants* are not — a real v1 break bought
for a payload shape that already exists); broadcasting the replay (it would
announce one client's history to every other attached client).
**Revisit if:** stage 4's capability handshake makes variant additions
negotiable — that is when batching the replay into one message becomes free — or
if a client is ever seen swamped by a large replay.

## Decision — read answers are dropped, not awaited: `Bus::try_direct`
**Date:** 2026-08-10  **Affects:** bus.rs, core.rs `handle_read_outcome`

**Chose:** a second, non-blocking directed primitive. `Bus::try_direct` returns
`false` and removes the lane on `Closed` (an ordinary detach, silent) or on
`Full` (after one loud line naming the `ClientId`). Backlog responses go out
this way. The asymmetry with the write path, stated: writes are buffered without
bound because a lost line is unrecoverable; reads are dropped because a read is
by definition re-askable, and the engine must never hold history behind a reader
that has ignored 64 answers and asked for a 65th. `Bus::direct` is unchanged,
and search's ordered Response-then-Event pair still rides it.
**Over:** awaiting `direct` for reads (one wedged reader stalls the select loop
and, behind it, the storage thread's `blocking_send` — the prompt-8
carry-forward); unbounding the directed lane (a read is not history); a bigger
bounded cap (moves the threshold instead of removing the coupling); converting
search's pair too — "what does half a delivered pair mean" is a real question
and it belongs to 9b's verb/drain seam, not here.
**Revisit if:** 9b converts the search pair, or dogfood shows a legitimate
client tripping the 64-slot drop.

## Prompt 9a — Windowed backlog

**Commit:** branch `prompt-09a-backlog` (PR open)  **Date:** 2026-08-10

**Shipped:** `FetchBacklog` for real, all four anchors, always ascending by seq.
`run_backlog` + `BACKLOG_MAX_LIMIT = 200` + a shared row-hydration fn in
storage/query.rs; two fire-and-forget read jobs (`Job::Backlog`,
`Job::ListBuffers`) behind the existing flush-before-non-ingest barrier,
answering on one new bounded (64) `reads_tx` as `ReadOutcome::{Backlog,
Buffers}`; `BufferRow` carrying the network *name* so storage never mints a wire
id; the `FetchBacklog` arm mirroring Search including the immediate
`Error("storage unavailable: …")` on a failed enqueue; attach-time buffer
announcement with network-name resolution, `state.buffers` seeding, and delivery
by a short-lived task holding a `Bus::lane` clone; `Bus::try_direct`; the CLI
`backlog <buffer> <latest|before:|after:|around:|around-hit> [limit]` verb,
`wait backlog` counting responses, and `backlog request=/line …` printing in the
new crates/supernaut/src/session_backlog.rs; two new live-run segments. No wire
change: `PROTOCOL_VERSION` stays 1, no variant added, no `RequestBody` addition.
Both decisions above were made in the prompt text and are recorded there too.

**Deviations:** two, both small. (1) The implementer's warning that
`crates/havoc-core/tests/core_search.rs` would need to skip replay events did
not bite — that test spawns core with an empty settings map, so every seeded
buffer's network is unresolvable and nothing is announced. The test passes
untouched; the skip would have been dead code. (2) `parse_kind` (exec.rs) became
`pub(super)` so `run_list_buffers` could reuse it rather than fork a second
string→`BufferKind` mapping; the detail named only message-row hydration as
shared, and `parse_kind` then moved again in the split below. Also,
`handle_outcome`'s inline wire-Message construction now calls the same private
`wire()` helper the backlog and search paths use — a strict simplification, not
a behaviour change.

**Deviation: four file splits the detail did not foresee.** It predicted pressure
on session.rs alone (handled as ordered, by session_backlog.rs), but the
longest-file ratchet caught four files over 400 at first `make check`: core.rs
464, storage/tests.rs 462, storage/mod.rs 419, storage/exec.rs 419. Split along
seams rather than by cutting anything, and each new file says why in its header:
`crates/havoc-core/src/core/reads.rs` (the read path's whole delivery story —
search's ordered `direct` pair and a window's `try_direct`, next to each other on
purpose); `crates/havoc-core/src/storage/identity.rs` (the parts of storage that
**are** disk format — the two enums' column encodings plus the synthetic-msgid
hash — so the "changing this rewrites history already written" rule has one home
instead of three, which also gave `parse_kind` a better home than exec.rs); and
`crates/havoc-core/src/storage/tests/backlog.rs`. Longest file is now 375
(session.rs and storage/mod.rs tie). The ratchet's standing invitation to tighten
the ceiling to the new value is **declined here, deliberately**: 375 is exactly
session.rs, which is prompt 9b's own working file, so tightening now would fail
9b's first commit for a reason unrelated to 9b.

**Deferred:** nothing from this prompt's scope. Converting search's
Response-then-Event pair to `try_direct`, and `wait search`'s event-counting
blindness, remain 9b's (both were named in the prompt text as such and are
carried forward below).

**Learned:** the empty-settings escape hatch above is worth remembering — the
attach replay is silent in every test that does not configure a network, which
is why eleven prompts of dispatch tests needed no edits. `Anchor::AroundSearchHit`
over a *deleted* seq needed a second SQLite connection to test at all: the write
path is append-only by design, so the only way to punch the gap retention will
one day punch was to open the file again and `DELETE` (the storage thread was
idle; WAL made it uneventful). The centred-window arithmetic is the part worth
having pinned by test: `before = (n-1)/2`, `after = (n-1)-before`, descending
scan inclusive of the anchor — three assertions (middle, first row, last row)
were enough to catch getting it off by one.

**Measured:** the whole backlog segment — a 5-row window, a 9999-asked/200-served
window, and a 7-row centred window over a 502-row buffer — is 0s at the
harness's 1s granularity, as search is; `storage backlog buffer=N rows=K` under
`--trace-irc` is where a real number would come from if one is ever needed. 35
live assertions (up from 24). session.rs 375/400 against the longest-file
ratchet, which is what the new session_backlog.rs bought; three further files
needed the same treatment (see the deviation above). 50 workspace tests, 9 of
them new (5 storage anchors/cap/errors, 2 core attach-replay, 2 bus
try_direct).

**Live run:** `scripts/live-run.sh`, 35/35 green on the first attempt after
implementation, no retries. The two new segments, observed: A printed
`backlog request=… count=5` with `seq=1` first for `after:0 5`; `latest 9999`
came back `count=200` — the engine refusing the number it was handed, seen from
outside; `around-hit 7` centred on `flood line 250` with 247 and 253 present,
which is jump-to-context working headless off the search hit the session had
actually seen. Then session D — a process that issues no `connect` at all — over
A's closed data dir: `waited buffer #supernaut` from the attach replay alone,
`line … text=the deployment failed` read back out of history a *different*
process wrote, and no `event connection-state` line anywhere in its output.

**Review:** pending

**Carry-forward consumed:** all four notes on prompt 9a, deleted from
STAGE-1-PROMPTS.md in this change. (1) *Don't copy search's swallowed enqueue
failure* — the `FetchBacklog` arm returns an immediate
`Error("storage unavailable: {e}")`, and it was written that way from the first
line, not retrofitted. (2) *The bounded-lane-behind-blocking-send topology, and
the read-side backpressure story* — decided as `Bus::try_direct` with the
never-block-history asymmetry inverted for reads (decision entry above); the
storage thread can no longer be stalled behind a wedged reader on the backlog
path. (3) *`wait search` counts success events only* — the `backlog` verb counts
**responses** instead (`backlog_pending: HashSet<RequestId>`, incremented by
both `Backlog` and `Error`), so a failed window ends the wait with a printed
error rather than a timeout with nothing to read; `wait search` itself is
deliberately left alone as 9b's. (4) *A buffer that predates the core instance
is never announced* — settled by making the buffer set something the core
announces at attach rather than something a client can ask for, proven live by
session D.

**Carry-forward raised:** see Review.

**Oversize:** 1420 changed lines in crates/ against the 800 cap — 1099 before the
four ratchet-forced file splits above, which are pure moves and account for the
other 321. The detail predicted staying inside the cap and named the remedy —
trail the storage-level edge-case anchors into 9b — so this was measured against
that remedy rather than waved through, and the remedy does not close the gap: at
1099 the **non-test** code was 725 lines on its own (query.rs 193/37 largely the
shared hydration refactor, the core diff the announcement seam, bus.rs 76 for
`try_direct` and its two tests), and dropping every storage edge-case anchor
named as tradeable removes ~130, landing at ~969. Deleting verified tests to buy
~130 lines against a cap the non-test code already exceeds is a worse trade than
saying so here. Examined for a split and left whole for the reason the 9a/9b
split already recorded: the window and the announcement are one read-path seam —
the announcement exists so that a window can be *asked for* over a data dir this
process did not write, and the live proof of one is the live proof of the other.
382 of the lines are tests, all ordered by the prompt. The live run was never a
candidate for trimming.

## Prompt 9a — review addendum (answers the `**Review:** pending` line above)

**Date:** 2026-08-10  **Affects:** the prompt 9a entry above; PR #14, second commit

Appended rather than edited into that entry, deliberately: the entry is already
committed and pushed, so revising it in place trips `log-append-only` on the
staged diff — and "correct in a new entry" is what the rule asks for. Prompt 8's
review addendum set the same shape.

**Shipped:** five fixes from the review. (1) The one flatly-unmet order
requirement: `Event::BufferCreated` now carries the doc comment stating its
announced meaning — "this buffer exists and you did not know it", fires on
creation *and* on attach-time replay, receivers must treat it as idempotent.
Reusing the variant was always going to restate its meaning, and the reason for
not renaming it is that a rename is a wire break; leaving the restatement
undocumented would have made the decision unfindable from the type. Doc-only, so
still no wire change. (2) `run_search` restated the column list by hand while
`MESSAGE_COLUMNS`' comment claimed every message-row read shared it — it now
interpolates the constant (qualified with the join's alias via
`message_columns_as`), so a reorder cannot silently desynchronize the positional
`hydrate`; and the comment claiming `run_backlog` is "the single site that binds
the SQL LIMIT" now says what is true — `scan` binds it, `run_backlog` caps the
number handed to it. (3) `last_hits` inserted in *rank* order, so it held the
last-by-relevance hit while two comments claimed newest; it now keeps the
greatest seq per buffer, which makes the comments true and `around-hit` mean what
it says. Uncleared by design: across searches, the newest hit ever seen for a
buffer wins. (4) The gap test's second connection sets `busy_timeout` (5s) before
its DELETE, so future batch-timer work cannot make it flaky. (5) The 247/253 live
assertions are kept — they are what proves centring — with the arithmetic written
above them (the hit is at seq 252 because two join rows precede the flood; limit 7
splits 3/3), so anyone who changes #flood's traffic diagnoses the failure instead
of doubting the feature. Plus eleven carry-forward notes landed (below). The
entry's `**Oversize:**` figures become 1456 changed lines in crates/, 391 of them
tests, with these fixes.

**Learned:** the `last_hits` bug is the useful one — the code was defensible and
the *comment* was the defect, twice, which is the failure mode a reviewer catches
and a test never would (any single-hit search passes either way; the live run's
`around-hit` worked because `in:#flood "flood line 250"` returns exactly one hit).
Also mechanical, and worth not relearning: **splitting a prompt's work across two
local commits makes `make check` fail on the second one.** The pre-commit hook
compares staged-vs-HEAD, so an in-place edit to a build-log entry that is already
committed reads as a rewrite, and the entry's Shipped/Learned/Live-run sections
are no longer in the *added* lines. CI compares against the base ref and is green
either way. Two honest routes exist — squash the branch to one commit, or append
an addendum as here — and force-pushing a PR branch is not always available.

**Live run:** `scripts/live-run.sh` re-run after these fixes, since two of them
are on live paths (`last_hits` and `run_search`'s SQL): **35/35, exit 0** — the
third green run of this prompt.

**Review:** the review's highest finding is **deferred to 9b by design**: the
announcement task and `try_direct` contend for the same 64-slot lane while
`handle_search_outcome` still awaits `bus.direct`, so a client that awaits a
Response without draining events can be deadlocked by a replay it never asked
for, and a task parked on a wedged lane makes the Full-drop outcome
nondeterministic (zombie vs loud-kill). That is exactly the conversion this
prompt's text named as 9b's, for the reason it gave — "what does half a delivered
pair mean" has to be answered before search's ordered pair moves off `direct` —
and answering it here would have been the second seam the 9a/9b split exists to
avoid. It goes to 9b with the >64-buffer attach named as its test. The ungated
`FetchBacklog` (a client can read, and enumerate by probing ids, buffers the
announcement deliberately withheld — the skip is advisory, not a boundary) is
harmless under single-user filesystem auth and becomes a real question when the
socket makes clients plural, so it is a stage-4 note. Two liberties are
**accepted as they stand**: `Anchor::Latest` binding `seq <= i64::MAX` as a
sentinel rather than branching the SQL (one parameter shape for all four anchors
is worth more than the purity), and the CLI's optional anchor with a default
`limit` of 50 (a debug-harness convenience the wire never sees). Nothing else was
left unaddressed.

**Carry-forward raised:** eleven proposals from the harvest, all adopted, none
rejected — four to prompt 9b (the replay/`try_direct` lane collision and search's
await; `wait search` converting to the response-counting pattern rather than
growing a parallel one; live-run's session-D window being anchored before
#supernaut gains traffic; the stale "SetReadMarker arrives in prompt 9" string),
two to prompt 10a (network `name` uniqueness becoming a validated config
invariant rather than an assumption inside `announce`; session D's `--host`
coupling switching to config in the same commit), three to PLAN stage 4
(`FetchBacklog`'s lost exactly-one-Response guarantee needing a wire story; the
advisory skip; `BACKLOG_MAX_LIMIT` being deliberately undiscoverable and owed by
the handshake as a negotiated *value*), one to PLAN stage 2 (a `Backlog` response
can name a buffer before its `BufferCreated` arrives — the mirror of the existing
ordering note), and one to PLAN stage 6 (`last_read_seq` is already a *read*-path
value handed to every attaching client, so per-client markers change
`run_list_buffers` too).

## Decision — one directed primitive, bounded at 4096, never awaited

**Date:** 2026-08-10  **Affects:** bus.rs, core.rs `run`, core/reads.rs
(`handle_search_outcome`, `handle_read_outcome`, `announce`), `CoreHandle::attach`

**Supersedes** "read answers are dropped, not awaited: `Bus::try_direct`"
(2026-08-10, one prompt old). That entry's own *Revisit if* named this prompt, so
this is sanctioned rather than a violation, and its asymmetry argument survives
intact: writes are never dropped, reads are re-askable. Only the mechanism
changes, because dropping *a read* at 64 slots was never the real bound — a
second writer (the replay task) could park on the same lane and make the drop
nondeterministic.

**Chose:** `DIRECTED_LANE_CAPACITY = 4096` and exactly one primitive,
`Bus::direct(&mut self, id, message) -> bool`, implemented on `try_send`:
never awaits, never blocks, never silently drops a message while the lane lives.
`Bus::try_direct` and `Bus::lane` are deleted. `Closed` removes the lane silently
(an ordinary detach); `Full` removes it after one loud line naming the `ClientId`.
Three things follow structurally rather than by care. (1) **No path in the core
loop awaits a client**, so no client can stall the select loop and the storage
thread's `blocking_send` on `search_tx`/`reads_tx` can never be held behind a
wedged reader — those stay bounded at 64, because that backpressure is
core↔storage and both ends now always drain. (2) "What does half a delivered pair
mean" becomes unrepresentable: `handle_search_outcome` is non-async and pushes
Response-then-`SearchResults` through two synchronous calls with no await point
between them, so the pair either both lands or the lane is already gone and the
session is over. (3) The attach replay stops being a task and delivers inline from
the core loop, which deletes the second writer and with it the zombie-vs-loud-kill
nondeterminism — the map entry is again the only aliveness token, and only the
core loop touches it.

**Over:** an unbounded lane with a `sender.len()` depth watermark at 4096 — the
JIT writer's original proposal, and it **cannot compile**: in tokio 1.53 `len()`
exists on `UnboundedReceiver` only, not `UnboundedSender`, and the bus holds the
sender. Tracking depth receiver-side would buy machinery for zero behavioural
difference against `try_send` at the same threshold, and would change
`Session.directed`'s type (touching `wiring.rs` and every dispatch test) for it.
Also over: a per-session writer task with an overflow deque (the right answer when
*bytes* are the bound, which is stage 4's problem, where clients become plural and
untrusted); dropping a single message at the threshold instead of the session (a
client that has ignored 4096 answers is broken, and dropping message 4097 quietly
leaves it broken and undiagnosed); and keeping two primitives (the two-writer
nondeterminism above is what two primitives cost).

**Honest about the bound:** 4096 counts *messages*, not bytes, and one message can
be a 200-row window. It sits far above any legitimate attach replay (one message
per buffer; a human does not have 4096 buffers) and far below hurting a laptop for
`BufferInfo`-sized traffic. Byte-based accounting is a PLAN stage-4 note.

**Revisit if:** stage 4's socket makes bytes rather than messages the bound, or
dogfood shows a legitimate client tripping 4096.

## Decision — read markers: broadcast, backward-legal, its own job behind the barrier

**Date:** 2026-08-10  **Affects:** storage `Job::SetReadMarker` /
`ReadOutcome::MarkerSet` / `rows.rs::set_read_marker`, core's `SetReadMarker` arm,
`Event::ReadMarkerChanged`'s doc

**Chose:** four things that could each have gone the other way.
(1) **Broadcast, not directed.** §4.5 puts read markers in the Core column, the
value is the same one `announce` already hands *every* attaching client in
`BufferInfo.last_read_seq`, and a marker moved by one client is a marker moved for
the machine as long as `buffer.last_read_seq` is one nullable column. It is not a
leak, and `Bus::broadcast`'s `debug_assert` list stays `SearchResults`-only —
`ReadMarkerChanged` carries no `RequestId`, which is the structural tell.
(2) **A marker may move backward; last write wins, no clamp.** The client is the
authority on where a person has read to, and scrolling back to an unread point is
a real product action. A monotonic clamp refuses a legitimate request with no way
to report the refusal, and "highest wins" *is* the last-write-wins reconciliation
rule PLAN's Still-open owns — pre-deciding it inside an `UPDATE` is how that
question gets answered by accident.
(3) **The seq is never checked for existence and never clamped to `MAX(seq)`**; a
marker is a position, not a row reference, and retention (stage 6) will make gaps
real — the reasoning `Anchor::AroundSearchHit` over a vanished seq already ships
with. `seq < 1` *is* an error, the call `limit == 0` got in 9a. Unknown buffer is
decided by the `UPDATE`'s own `changes()`, not a preceding `EXISTS`: one
statement, precise, and cheaper than the read path's check.
(4) **Its own job, not a batched ingest.** Batching exists to amortize fsync over
history at IRC's line rate; a marker is human-paced and one row, and delaying it
100ms would make the `Ack` a lie. Falling into `exec.rs`'s `Some(other)` arm gives
the flush barrier for free, so the persisted marker is never ahead of persisted
history — which is what makes it safe to read back after a crash. It answers on
the existing `reads_tx`, not a fourth `mpsc` in core's select loop for one row.

**Over:** a per-client marker or any reconciliation (the wire and the schema can
represent exactly one, so "shipping markers" means shipping the machine-wide one —
stage 6, and the Still-open stays open); a monotonic clamp (see above); validating
the seq against `MAX(seq)` (a false promise the moment retention lands); clearing
a marker (no `Option<Seq>` berth on the wire, and "mark unread" is a stage-2
product question); a directed `Ack`-only answer with no event (the marker is state,
and state that only its setter learns about is the §4.5 bug class); a trace
`eprintln` for the write (one row is not a measurement, and the Ack and the event
are already observable).

**Revisit if:** the schema gains a per-client marker table — at which point the
*event's audience* changes with it, not only the write, and the variant's
documented meaning changes too (a stage-4-handshake-gated wire change).

## Decision — "sent" means at-most-once, and the drain is what quit owes

**Date:** 2026-08-10  **Affects:** `ActorCommand`'s doc and the backoff drop site
in connection/actor.rs, `RequestBody::SendText`/`Join` docs, the CLI's
`finish()` in session_wait.rs

**Chose:** documented at-most-once, made loud — plus a real drain on the client
side, because the two failures were being confused for each other. The drop site
in the backoff sleep gains one `eprintln` naming the network and the command;
`ActorCommand`'s doc and `SendText`/`Join`'s wire docs state at-most-once outright.
Separately, `quit` and stdin EOF both call one `finish()`: wait until nothing is
outstanding or a deadline (`quit [secs]`, default 10) expires, then a 50ms
quiet-period sweep so an event queued *behind* its own response is printed rather
than discarded by the runtime drop. A timeout is an `Err` naming the count, so the
process exits non-zero and the harness notices — an engine that did not answer is
a finding, not a shrug.

**Over:** an actor-side delivery *outcome*, which has nowhere to go — the `Ack` has
already been sent and the correlation with it, so saying "not sent" needs a wire
berth: a per-request delivery outcome, i.e. a variant addition, i.e. a real v1
break (the same refusal 9a made twice). Also over: a resend queue (that is a
decision about duplicates after reconnect, and it belongs to stage 5's resync
seam); and treating the drain as a fix for the drop (it is not — the drain makes
the client wait for answers the engine *did* send).

**The deeper reason at-most-once is the right answer anyway:** the echo is the real
confirmation. `echo-message` is in the requested caps (`connection/caps.rs`), so
our own PRIVMSG comes back as a `MessageAdded` from the authority that matters; on
a server without it, "sent" is unconfirmable by *anyone*, which is exactly what
at-most-once means. Carol's live-run echo wait already relies on this and stays.

**Revisit if:** stage 4's handshake makes a per-request delivery outcome
negotiable, or stage 5's resync gives duplicates a story that makes a resend queue
safe.

## Prompt 9b — Read markers and verb hardening

**Commit:** branch `prompt-09b-read-markers` (PR open)  **Date:** 2026-08-10

**Shipped:** read markers set, broadcast, persisted, and handed to a process that
never dialled anything — and the delivery-policy rewrite that was the larger half.
Storage: `Job::SetReadMarker` behind the existing flush-before-non-ingest barrier,
answering on the existing `reads_tx` as a third `ReadOutcome::MarkerSet`, with the
SQL in a new `storage/rows.rs`; `seq < 1` and unknown-buffer are errors, the latter
from the `UPDATE`'s own `changes()`; a marker may move backward. Core: the
`SetReadMarker` arm mirrors Search/FetchBacklog including the immediate
`Error("storage unavailable: …")` on a failed enqueue, and the stale
`error("SetReadMarker arrives in prompt 9")` string is gone; the outcome is
`ResponseBody::Ack` on the requester's directed lane plus a broadcast
`Event::ReadMarkerChanged`. Bus: `DIRECTED_LANE_CAPACITY = 4096` and one
synchronous `Bus::direct` on `try_send`, with `try_direct` and `lane` deleted;
`handle_search_outcome` non-async so its ordered pair has no await point inside it;
`announce` delivers inline from the core loop instead of a spawned task. CLI:
`SessionState.outstanding: HashMap<RequestId, Awaited>` with one insert site
(`request`, classifying the body itself) and one remove site (`handle_incoming`,
counting before the body match), replacing `search_count` *and*
`backlog_pending`/`backlog_count`; `mark-read <buffer> <seq>`; kind-aware
`msg_counts { chat, total }` with `wait message` (privmsg/notice) and the new
`wait rows` (every kind); `wait marker`; the drain `finish()` on both `quit [secs]`
and stdin EOF; `BufferCreated`'s printer grows `last_read=<seq|->`. Doc comments on
`ReadMarkerChanged` (broadcast core-owned state, one marker per machine, per-client
is a stage-6 schema change) and on `SendText`/`Join`/`ActorCommand`
(at-most-once). live-run.sh: session D re-anchored to `after:0 50` first, then the
marker segment, then the drain's two counts and the marker on disk. No wire change:
`PROTOCOL_VERSION` stays 1, no variant added, no `RequestBody` addition. The three
decisions above were made in the prompt text and are recorded there too.

**Deviations:** three, all mechanical, none of scope. (1) The detail predicted
`storage/exec.rs` would cross the ratchet and named `storage/rows.rs` as the split
to take; it did (366 → ~401 with the new arm and fn), so the split was taken as
ordered — `ensure_network`, `ensure_buffer`, `set_read_marker` in one file, exec.rs
at 353. (2) Two *unforeseen* splits along the same seam: `storage/mod.rs` hit 421
with the new `Job` variant, `ReadOutcome::MarkerSet`, and the client method, so the
channel's data vocabulary (`Ingest`, `SearchOutcome`, `BufferRow`, `ReadOutcome`,
`IngestOutcome`, `StoredMessage`) moved to `storage/records.rs` — pure data, no SQL
and no thread, which is a real seam and not a cut; and `storage/tests.rs` hit 403
with the marker tests, so those moved to `storage/tests/markers.rs` exactly as
`backlog.rs` had. (3) The `wait` arg parsing moved into session_wait.rs with the
`wait` machinery (the detail named "the wait/drain machinery" without deciding
where the parsing lived); `handle_incoming` moved with it, because it is the
response-counting site the detail put in that file.

**Deviation: the completion claims are held back one commit, deliberately.** The
detail ordered the status line bumped to `10/12 complete. Next: prompt 10a.` — and
`make check`'s blocked-prompt rule refuses exactly that, because PLAN's
config-vs-runtime-state question carries `*(blocking: prompt 10a)*`. The gate is the
mechanism working as designed, one commit earlier than expected: it fires on the
commit that *names* 10a as next, not on the commit that starts it. Answering it (or
downgrading it, which the item's own "prompt 10a ships the config file, so it settles
here" contradicts) is a stage-10a design decision with a rejected alternative, and
making it inside 9b to turn a check green is the worst possible reason to decide
anything. So this commit ships the prompt's work, its detail, its consumed notes, and
this entry, and leaves three lines for a follow-up commit on this branch, landing
together with the decision entry that unblocks them: STAGE-1-PROMPTS.md's status
line, its 9b outcome block, and README's badge plus table row.

**Deferred:** nothing from this prompt's scope. The two tradeables the detail named
against the line budget (storage-level marker edge cases; `wait rows`) were *not*
traded — see **Oversize:** below for why trimming them would not have closed the
gap.

**Learned:** three things worth not relearning. (1) The oldest bug here was not the
64 slots, it was the *second writer*: with a spawned replay task holding a lane
clone, the map entry stopped being the only aliveness token, so whether a wedged
client got killed loudly or lingered as a zombie depended on scheduling. Making
delivery synchronous deleted the task and the nondeterminism together — the
capacity number was the smaller half of the fix. (2) A 65-buffer attach is a
genuinely cheap way to pin a lane policy: seeding 65 buffers via `ensure_buffer`
(no ingest needed) and attaching a `Session` with no pump reproduces the exact
client the old code deadlocked on. Verified load-bearing by temporarily setting
`DIRECTED_LANE_CAPACITY = 64` — the test fails at the 65th announcement — then
setting it back. (3) The Ack/event ordering claim is not theoretical: the live run
printed `event read-marker buffer=2 seq=3` *before* `ok 16` for the request that
caused it. Two lanes, no ordering, observed on the first run — which is why `wait
marker` counts responses and why stage 2 gets the note.

**Measured:** 40 live assertions (up from 35), 9.2s wall for the whole run.
Longest file 363 (`tests/state_machine.rs`, untouched by this prompt); the four
files this prompt pushed over now read exec.rs 353, storage/mod.rs 327,
storage/tests.rs 289, session.rs 298 — session.rs *down* 77 lines despite gaining a
verb, the drain, and two wait targets, which is what session_wait.rs (215) bought.
The ratchet ceiling stays 400, as the detail ordered. 56 workspace tests, 6 of them
new (3 storage markers, 2 core markers, 1 the 65-buffer attach) plus the bus
capacity test replacing the Full-drop one. Storage trace shows 11 commits for the
whole flood+session and three `storage backlog` lines; the marker write adds no
trace line, deliberately.

**Live run:** `scripts/live-run.sh`, **40/40 green, exit 0, on the first attempt**
after implementation, no retries. Observed, in order: `waited backlog` for the
centred window, then `event read-marker buffer=2 seq=3` and `ok 16` — the broadcast
event arriving *before* its own Ack, in the process that asked. Then session D, which
issues no `connect` at all, over A's closed data dir: `event buffer-created
buffer=2 network=1 name=#supernaut last_read=3` straight out of the attach
announcement (with `name=* last_read=-` and `name=#flood last_read=-` around it, so
the marker is visibly per-buffer), and `line … text=the deployment failed` from the
`after:0 50` window. `sqlite3` agreed from outside both processes:
`last_read_seq=3`. The drain's counts: exactly 3 `ok` lines in b.out and exactly
502 in c.out — the requests that used to be discarded by the runtime drop (c.out
stopped near `ok 192` at prompt 7). Not exercised live: the loud
dropped-while-disconnected line, because no command is issued inside a backoff
window in this script — the path is documented and made loud, and it is honest to
say it did not fire.

**Review:** pending

**Carry-forward consumed:** all seven notes on prompt 9b, deleted from
STAGE-1-PROMPTS.md in this change. (1) *From prompt 7 — `wait message` counts every
kind, keep the contiguity assert intact*: `msg_counts` is now `{ chat, total }`,
`wait message` counts privmsg/notice and the new `wait rows` counts every kind, so
a row-level claim stays expressible in the CLI; the `COUNT(*) == MAX(seq)` assert
was not touched, and `wait rows` exists so the script never needs to. (2) *From
prompt 7 — quit races in-flight requests*: `finish()`, on both `quit` and stdin
EOF, with a hard `Err` on timeout; proved by asserting 3 responses in b.out and 502
in c.out rather than by claiming a feature. (3) *From prompt 6 — commands issued
while disconnected vanish with no signal*: decided as documented at-most-once, made
loud at the drop site and stated on `ActorCommand`, `SendText`, and `Join`, with the
echo named as the real confirmation (decision entry above); an actor-side outcome
was rejected for having nowhere to go, and the composer's half of it is a stage-2
note. (4) *From prompt 9a — the replay task and `try_direct` fight over one lane,
and search still awaits it*: both awaits decided together — one synchronous
`Bus::direct` at 4096, the replay inline, `handle_search_outcome` non-async — with
the 65-buffer attach as the ordered test, which fails against the pre-9b engine.
(5) *From prompt 9a — two counting patterns in one struct*: `outstanding` +
`Awaited` replaced both, `search_count` is gone, and the counter still runs before
the body match so an `Error` ends a wait. (6) *From prompt 9a — session D's window
is fragile to new #supernaut traffic*: re-anchored to `after:0 50` **first**, before
any verb semantics moved, exactly as ordered. (7) *From prompt 9a — the stale
"SetReadMarker arrives in prompt 9" string*: died with the arm.

**Carry-forward raised:** see Review.

**Oversize:** 1454 changed lines in crates/ against the 800 cap. Measured against
the remedies the detail named rather than waved through, and they do not close the
gap: of that total, 437 lines are tests (core_markers.rs 187, the 65-buffer attach
70, the storage marker tests 118, the bus capacity test ~62) and ~214 are pure
*moves* counted twice by numstat (storage/records.rs, storage/rows.rs,
storage/tests/markers.rs, session_wait.rs — every one of them forced by the
longest-file ratchet and taken along a named seam). Net of the moves the change is
~1240, and the two tradeables the detail listed are the storage marker edge cases
(~118) and `wait rows` (~25): dropping both lands at ~1100, still well over, while
deleting verified tests to buy 143 lines is a worse trade than saying so here. The
reason the number is what it is, plainly: this prompt's markers are small (the arm,
the job, one `UPDATE`, one verb) and its *other* half was a delivery-policy rewrite
touching every path prompts 5–9a shipped — which is what the 9a/9b split budgeted
for and spent here deliberately. Examined for a further split and left whole: the
lane rewrite and the marker cannot be separated, because the marker's Ack-plus-event
pair is the thing that made "what does half a delivered pair mean" have to be
answered, and the 65-buffer test proves the policy the marker rides on. Never a
candidate for trimming: the live run, the 65-buffer attach, the capacity test.

## Decision — config vs. runtime state: the database owns runtime state, config is seed-only

**Date:** 2026-08-10  **Affects:** prompt 10a's whole surface (the TOML schema),
and PLAN's **Still open** list, from which this item is deleted by this entry

**Chose:** the config file is **read-only seed data** — networks, nick, autojoin —
applied at startup; everything the program learns, or the user does at runtime
(joined channels, the buffer set, read markers) lives in the database. **The program
never writes the config file, ever.** "I joined this channel manually" is database
state, which the buffer row ingestion already creates; config's `autojoin` list is
the *seed*, not the record.
**Over:** config as the mutable source of truth — the program rewriting the user's
TOML clobbers their comments and formatting, races their own edits, and makes
"config paths are public API" a lie; and two-way sync, which has both problems plus
a reconciliation story nobody asked for.
**Because:** NORTH-STAR §9's stated leaning (database, config as seed only), and the
same principle already in force elsewhere — the program never writes to its own
source tree, and the config file is likewise the *user's* document.
**Revisit if:** dogfood (stage 3) shows users expecting manual joins to become
autojoin entries — and the answer then would be an explicit "save to config" action,
never a silent rewrite.

**This unblocks prompt 10a**, whose own text said the question settles there; the
gate fired one commit earlier than that, on 9b's status bump, which is what forced
the decision now (see the 9b entry's deviation on the held-back completion claims).
**Verified while here, as the item's own text required:** no earlier prompt persists
join *intent* anywhere this answer would have to migrate. The only join-shaped rows
on disk are `message` rows of kind `join` (observed history, written by the ingest
classifier) and `buffer` rows (a buffer exists because traffic was seen for it);
autojoin lives only in `connection::Config.autojoin`, in memory, passed per actor
spawn. Both are consistent with the answer: history is a record of what happened,
not a declaration of what should happen at next startup.

## Prompt 9b — review addendum (answers the `**Review:** pending` line above)

**Date:** 2026-08-10  **Affects:** the prompt 9b entry above; PR #15, second commit

Appended rather than edited into that entry, deliberately, for the reason prompt 9a's
addendum recorded: the entry is committed and pushed, so revising it in place trips
`log-append-only` on the staged diff.

**Shipped:** seven fixes from the review.

(1) **The highest finding, and it was a real one: the 65-buffer attach test did not
pin the deadlock — it passed against prompt 9a's engine.** The shape drained all 65
announcements *before* asking for a window, so the old spawned replay task made
progress and `try_direct` never saw a full lane. The reviewer's proposed shape (ask
immediately, nothing drained, sleep, then drain and assert the correlated Response
arrived at all) was applied — **and still passed on main**, because with nothing
ordering the two answers the outcome is a coin flip between the replay task filling
the lane and the window's answer arriving first. That coin flip *is* the
nondeterminism this prompt's lane decision describes, so a test that depends on it
proves nothing. The shape that is deterministic adds one wait between attach and the
fetch, so the replay has demonstrably landed (pre-9b: filled the 64-slot lane and
parked its task on the 65th) before the window is asked for. **Verified both ways, 5
runs each: 5/5 FAILED against `origin/main`** with the exact loud line
`client 1 is not draining its directed lane; dropping it and this read`, **5/5 pass
on HEAD.** The test's doc comment now records both wrong shapes and why each was
wrong, because the next person to "simplify" it will reach for one of them.

(2) **A secrets leak in a line this prompt added.** The dropped-while-disconnected
`eprintln` printed `{command:?}`, and `ActorCommand::Privmsg` carries the text — so a
NickServ `IDENTIFY` issued during backoff would have put a password on stderr,
against CLAUDE.md's rule, which governs logs exactly as much as config. Fixed with a
`describe(&ActorCommand)` helper printing variant and target only (`Privmsg to
#chan`, `Join #chan`); the line stays **ungated and loud**, only the body goes, and
`ActorCommand`'s doc now says so. Worth naming the shape of the mistake: the leak
arrived *with* the fix for silence — making a drop loud and making it safe are two
jobs, and `{:?}` on a domain type is where the second one gets skipped.

(3) `quit abc` silently meant `quit 10`. Now an error line naming the argument, and
the session stays alive — a swallowed typo would quietly restore the exact race the
drain exists to close.

(4) Two over-broad doc claims tightened. bus.rs said no `blocking_send` can be parked
behind "a wedged reader"; the true claim is *behind a client* — the core loop can
still park on the storage thread, because `connect` awaits a
`spawn_blocking(ensure_network)` round trip and the bounded reply lanes are drained
only by that same loop. The doc now scopes itself and points at the stage-4 note.
`announce`'s idempotence contract ("a duplicate is legal, a missing one is not") now
says what it is conditional on: it holds while the lane is live; under `Full` the
client is dead by policy and the replay is abandoned with it.

(5) live-run.sh's `around-hit` line gained the comment its correctness depends on:
`last_hits` is filled by the SearchResults *event* while `wait search` now returns on
the *response*, which is safe **only** because both ride the same directed lane in
order. Filed as a stage-2 carry-forward too, so the invariant is not documented in
only one place.

(6) **Finding (b), the untested failure path: fixed rather than deferred.** `crates/
supernaut` had zero tests, and `finish()` returning `Ok` on timeout would have let
the whole 40-assertion acceptance suite pass while `quit` silently discarded in-flight
requests — invisible from outside, because the assertions count printed responses, not
the exit code. It unit-tests cleanly with a fabricated `SessionState` (a dangling
`RequestId`, an `incoming` channel held open so the lane is pending rather than
closed) and deadline 0, which keeps it instant without a paused-clock runtime and
takes the identical code path a real 10s timeout takes. No restructuring was needed,
so the rejected-because is moot. First test in the binary crate.

(7) STAGE-1-PROMPTS.md's 9b section gained the `**Status:** complete.` outcome
paragraph every other completed prompt has.

**Accepted as they stand, with the reasoning recorded rather than re-litigated:**
the oversize disposition (1469 vs 800 — the review confirmed neither descope-ladder
item would have closed the gap; they total 143 lines, so the justification in the
entry above stands); `storage/rows.rs` taken unconditionally although exec.rs would
have landed at ~383 rather than over 400 (the split's *shape* was the one the detail
ordered, and its cost was paid against a budget already blown — reversing it now
would churn a file for a line count that is no longer the binding constraint);
`storage/records.rs` as a second unforeseen ratchet-forced split (a pure move along a
real seam); and `wait search`'s reorder consequence, which the review verified safe
via the same-lane invariant now written into the script.

**Rejected, with the reason:** the proposed note that `Awaited::from`'s exhaustive
match makes a future wire change a two-file edit. No remaining stage-1 prompt may add
a wire variant — each one's fence forbids it and `PROTOCOL_VERSION` is frozen until
stage 4's handshake — so the note changes nothing about how 10a or 10b execute, and
the exhaustive match is already its own compile-time enforcement. A note whose only
effect is to be read and agreed with is noise in a file the next session must read
front to back.

**Learned:** the useful lesson is (1), and it generalizes past this test. **A
regression test written from the fix rather than against the bug will pass on the
buggy code**, and nothing in a green suite says which kind it is. The only way to
know was to check out the parent commit, copy the test onto it, and run it — cheap
(one `git worktree add --detach origin/main`), and it turned a test that proved
nothing into the one that pins the lane policy. It also caught the second-order
version of the same error: a shape that fails on main *sometimes* is not a
regression test either, and "sometimes" is exactly what a concurrency fix's test
looks like when it is still racing. Corollary worth keeping: when the bug being fixed
*is* a nondeterminism, the test has to remove the nondeterminism to observe it —
hence the wait, which is not a sleep-instead-of-a-sync but the ordering the assertion
is about.

**Live run:** `scripts/live-run.sh` re-run after these fixes, because actor.rs and
session.rs are both on live paths: **40/40, exit 0** — the third green run of this
prompt. `quit` with no argument still drains (b.out 3 `ok` lines, c.out 502), and the
marker round-trip is unchanged (`event read-marker buffer=2 seq=3` in a.out,
`last_read=3` in d.out and in `sqlite3`).

**Review:** the review's findings are all dispositioned above — two code fixes
(the non-load-bearing test, the secrets leak), five smaller fixes, four liberties
accepted with reasons, one note rejected with a reason. Its highest finding was
correct and its proposed remedy was insufficient, which is recorded above rather
than smoothed over.

**Carry-forward raised:** eight notes landed, one rejected. Two to prompt 10a (`quit`
now blocking and exiting non-zero, which a config-seeded autoconnect can trip; `wait
message` no longer counting the Join rows autojoin produces, so 10a must use `wait
rows`). Four to PLAN stage 4 item 1 (the replay is now the burst most likely to trip
the lane cap, so a per-session writer must serve it first; `Full` and `Closed` are
indistinguishable at the client, so the frame protocol owes the distinction; the core
loop can still park on the storage thread and `reads_tx` now has three producers; and
the existing byte-accounting note). One to PLAN stage 2 item 1 (`around-hit`'s
same-lane dependency). One rejected (the `Awaited::from` note, above). The
config-vs-runtime-state answer is its own decision entry above, not a note.

## Decision — the config schema is a map keyed by network name, and ids come from the loader

**Date:** 2026-08-10  **Affects:** `crates/havoc-core/src/config.rs`, and the
"caller-assigned ids" phrase in prompt 10a's stub

**Chose:** `[networks.<name>]` tables in a `BTreeMap<String, NetworkEntry>`, with
`NetworkId(1..N)` assigned by the loader in name order and **no `id` key in the
schema**. The table key *is* the stable network name — the same string
`ensure_network` keys the `network` table on — so config identity and storage
identity are one thing rather than two that must agree.
**Over:** (a) `[[network]]` array-of-tables with an explicit `name`, which puts name
uniqueness back in a validation pass somebody has to remember to write; (b) either
form with a hand-typed `id`, which is exactly the config ceremony NORTH-STAR §2.1
calls a failed product — and a uniqueness rule plus a migration story on top; (c)
ids hashed from the name, which trades renumbering for collisions and makes the
`network=1` in every live-run assertion unreadable.
**Because:** two `[networks.libera]` tables are a **TOML-level error**, so name
uniqueness is enforced by the *file format* and is unrepresentable in the parsed
type. That is what discharged prompt 9a's note without adding a check: `announce`
builds its `HashMap<&str, NetworkId>` exactly as before, and there is nothing to
assert because there is nothing to represent. Ids stay "caller-assigned" in the
sense core.rs means — assigned outside the engine, a distinct type from the storage
row id — but the assigner is the loader, not a human.
**The precondition was checked, not assumed:** no wire `NetworkId` is ever
persisted. `ensure_network` (`crates/havoc-core/src/storage/rows.rs`) keys on name,
`buffer.network_id` references the storage row, and `CoreState.network_rows` is
rebuilt at every `connect`. So renumbering across runs — add a network alphabetically
first and every id shifts — is unobservable. Names are case-sensitive, matching the
`network.name TEXT UNIQUE` column's default collation: `Libera` and `libera` are two
networks and two rows, deliberately and consistently.
**Revisit if:** a wire `NetworkId` is ever persisted or cached across restarts (a
config-id cache, a NetworkId in a saved layout, a marker table keyed by it). Then
renumbering becomes data corruption, the id becomes a config field, and the
uniqueness rule and migration arrive with it. Recorded as a PLAN stage 5 note so the
condition is checked where multi-network config is built, not remembered here.

## Decision — `toml` + `serde` in havoc-core, deliberately without the serializer

**Date:** 2026-08-10  **Affects:** `crates/havoc-core/Cargo.toml`

**Chose:** `toml = { version = "1.1.4", default-features = false, features = ["std",
"parse", "serde"] }` and `serde = { version = "1", features = ["derive"] }` in
havoc-core. Both were already on `DEP_ALLOWLIST` in `scripts/check-docs.sh`, seeded
from NORTH-STAR §5, so no allowlist edit and therefore no `scripts/test-checks.sh`
fixture is owed — but the decision entry is.
**Over:** (a) hand-rolling a TOML subset, rejected outright: quoting, escapes, dotted
keys, multiline strings, datetimes, and error spans are a commodity, and ours would
be the buggy one — in the file a user hand-edits, where a wrong parse is a wrong
connection; (b) `toml` with default features, which is the interesting rejection.
**Because:** dropping `display` means the **serializer is not compiled in**, so "the
program never writes the config file" (the 2026-08-10 config-vs-runtime-state
decision) becomes a capability the crate does not have rather than a rule it obeys.
Verified rather than assumed, since a feature name that silently does nothing would
make the claim false and invisible: a throwaway example calling `toml::to_string`
fails to compile with *"found an item that was configured out … gated behind the
`display` feature"*. `parse` and `serde` are both needed — `parse` for the text, and
`serde` for `Deserialize` plus `toml::Table`, which the credential-key scan walks.
**The transitive set**, named because the allowlist check reads *declared* deps only,
so hygiene here is an argument and not a check: `toml_parser`, `winnow`,
`toml_datetime`, `serde_spanned`, `serde_core`. Five crates, all from the same
maintainer as `toml` itself, no build scripts, no proc macros beyond `serde_derive`
(already in the tree via havoc-ipc). `winnow` is the only genuinely new lineage.
**Revisit if:** a "save this to config" action is ever wanted (stage 2's first-run is
the only plausible caller). That needs `display` back, which needs this entry
revisited *and* the config-vs-runtime-state decision revisited — two gates, on
purpose, because a silent config rewrite is the failure mode both exist to prevent.

## Decision — the debug CLI's whole connection surface moves into config; the six flags are deleted

**Date:** 2026-08-10  **Affects:** `crates/supernaut/src/session.rs`,
`scripts/live-run.sh`

**Chose:** `--host`, `--port`, `--nick`, `--join`, `--tls-ca` and
`--allow-plaintext` are **deleted**. `supernaut session` reads
`$SUPERNAUT_CONFIG_DIR/config.toml` (then `XDG_CONFIG_HOME`, then `$HOME/.config`)
and fails if it is absent. What remains is `--network <name>` (optional when the file
names exactly one), plus `--data-dir`, `--trace-irc` and `--sasl` — the first two
because they are location and diagnostics rather than seed data, the third because
prompt 10b deletes it together with the keyring that replaces it.
**Over:** (a) flags as a fallback when config is absent, which is the real trap:
live-run.sh would keep exercising the flags and config — the surface every later
stage builds on — would be the decorative path, green in CI and untested in fact;
(b) auto-writing a default config file on first run, forbidden by the governing
decision, and the exact behaviour that makes "config paths are public API" a lie;
(c) keeping `--allow-plaintext` beside the per-network `plaintext` key.
**Because** (c) is the sharpest of the three: **security is per-network, so a
process-global flag cannot say which network it blesses.** With two networks and one
flag, the safe reading and the useful reading differ, and neither is expressible. The
opt-in belongs where the host is. The loopback-only restriction moved with it, from
`is_loopback` in session.rs into config validation — a string check, not name
resolution, and that is unchanged.
**Mandatory config is scoped to `session` only**, which is how NORTH-STAR §3.1's
"works well before configuration" survives: `supernaut --data-dir <p>` still opens
the store and reports with no config file anywhere (verified by hand, exit 0). That
is the whole of the property stage 1 can honestly claim; the product's answer is
stage 2 item 7, recorded there with both fences (no flags fallback, no silent write).
**Revisit if:** stage 2's first-run needs a pre-config connect path. The shape then
is a user-confirmed write, never an implicit one.

## Decision — network identity after config: orphan rows abandoned but reversible, `in:` left unscoped and pinned

**Date:** 2026-08-10  **Affects:** `crates/havoc-core/src/core/reads.rs`,
`crates/havoc-core/src/search.rs`, `crates/havoc-core/src/storage/query.rs`

**Chose, for the pre-config `debug-<host>` network rows:** abandonment, made visible
and kept reversible. No migration. `announce`'s silent `continue` over an
unconfigured network name became one greppable stderr line per skipped network —
`orphan network <name>: N buffers not announced (not in config)` — counted during the
loop and printed after it, so a data dir full of orphans stays readable. Reversibility
is structural, not a feature: storage keys networks on the same stable name config now
supplies, so adding `[networks."debug-localhost"]` to the file brings those buffers
back.
**Over:** migrating `debug-<host>` rows onto config networks by host string. That
match is a **guess**, and guessing wrong merges two networks' histories
irreversibly — the one unrecoverable outcome available here. Nobody has irreplaceable
history in a temp-dir debug row, so the expensive half of the trade buys nothing.
**Chose, for `in:`:** leave it unscoped, and stop it being an accident. The accretion
*cause* died with `debug-<host>` (stable names mean one `network` row per network, not
one per host string), so the cross-network union of one channel name is now real but
rare. The deferral took its honest form instead of a comment: the behaviour is
documented on `SearchSpec.buffer` and `run_search`, and **pinned** by
`in_filter_unions_one_buffer_name_across_networks` — two network rows, one buffer
name, `in:#supernaut` returns both.
**Over:** inventing the scoping grammar here. `in:net/#chan` versus a separate
`network:` filter is a UI question owned by stage 2's `/search`, which is the only
consumer that can *render* which network a hit came from; the debug CLI prints
`buffer=<id>`, so a scope filter would be unobservable in the one harness that could
have tested it. A grammar chosen where it cannot be observed is a grammar chosen
badly.
**Honest about the hole this leaves:** orphan history stays reachable through
`Search`, which has no network filter at all, so a hit can carry a `BufferId` the
client cannot name. That is the same advisory-not-a-boundary gap PLAN stage 4 already
owns for `FetchBacklog`, and it is recorded there as a second note rather than fixed
here — a check that gates search results on the client's config is a real access
decision, and it belongs with the socket that makes clients plural.
**Revisit if:** stage 2's `/search` finds the union actively confusing on two
networks (it should: that is when the pinned test starts failing on purpose), or if
stage 4 decides the announcement skip is a boundary — then search inherits the same
rule in the same commit.

## Prompt 10a — Network config file

**Commit:** branch `prompt-10a-config` (PR open)  **Date:** 2026-08-10

**Shipped:** the user's TOML file is now the authority on network identity, and the
debug CLI has no connection flag left. New `crates/havoc-core/src/config.rs` (243
lines) owns the schema — top-level `nick`, `[networks.<name>]` with `host`, optional
`port`, `tls_ca`, `autojoin`, `plaintext` — its validation, and the lowering
`Config::into_networks() -> HashMap<NetworkId, NetworkSettings>`. It does **no file
I/O**: `parse(text, base_dir)` so every rule is testable without touching disk, with
`String` errors because nothing branches on the variant, exactly as `search::parse`.
Validation, all of it before anything dials and every message naming the network and
the key: `nick` non-empty and whitespace-free, `host` non-empty, network name
non-empty, `plaintext` with a non-loopback host, `plaintext` with `tls_ca`. Not
validated, deliberately: `autojoin` channel names (CHANTYPES is ISUPPORT's, so we
would be guessing) and `tls_ca`'s existence (the TLS connector reports that once,
where the failure is). `#[serde(deny_unknown_fields)]` on both structs plus a
**by-name refusal** for `password`/`pass`/`sasl_password`/`nickserv_password`,
recursive so `[networks.x] password = …` is caught too. `tls_ca` resolves against the
config file's own directory. The binary side: `default_config_path` beside
`default_data_dir` in main.rs (`SUPERNAUT_CONFIG_DIR`, `XDG_CONFIG_HOME`,
`$HOME/.config`, else an error naming all three), and `session.rs` loses
`--host`/`--port`/`--nick`/`--join`/`--tls-ca`/`--allow-plaintext` and gains
`--network <name>`; `const NETWORK: NetworkId = NetworkId(1)` is gone and
`SessionState` carries the resolved `network`. Core is spawned with **every**
configured network, not just the selected one. SASL is injected into the selected
network's `connection.sasl` after lowering, in one marked place, because config lowers
`sasl: None` always. `announce` reports orphan networks out loud. `SearchSpec.buffer`
and `run_search` document the unscoped `in:`, pinned by a new storage test. Eleven
integration tests in `crates/havoc-core/tests/config.rs`. live-run.sh generates a
config per session and passes no connection flag anywhere; session E is new. No wire
change (`PROTOCOL_VERSION` stays 1), no storage schema change, no migration. The four
decisions above were made while doing this and are recorded there.

**Oversize:** 833 changed lines in `crates/` against a cap of 800 — 4% over, and a
deviation from the detail's own claim that this was "comfortably inside" the tripwire.
The excess is not a second prompt hiding inside this one: `config.rs` (243) plus
`tests/config.rs` (256) is 60% of it, and the rest is session.rs's rewrite. **The
descope ladder was not used, deliberately**, and the reasoning is worth keeping: its
three rungs (session E and the orphan line, the credential-key refusal list, the `in:`
pinning test) total roughly 90 lines, so cutting all three would have landed at ~740 —
under the cap, and having deleted the three things that make this prompt's *claims*
observable. Session E is the only proof that abandonment is visible; the refusal list
is the only reason "credentials never live here" is a message rather than a hope; the
pinned test is the entire content of the `in:` deferral. Trading them for 33 lines
would be optimising the check instead of the work. Recorded rather than negotiated.

**Learned:** three things, in descending order of how much they would have cost later.

(1) **A feature flag that means "the capability is absent" has to be verified, because
a wrong feature name fails silently and in the safe-looking direction.** The whole
"the program cannot write the config file" claim rests on `toml`'s `display` feature
being off — and if the feature had been renamed between versions, `default-features =
false` would have quietly compiled anyway, the serializer would have been absent for a
different reason or present for no reason, and the entry above would have asserted
something nobody checked. Cost to check: one throwaway `examples/` file calling
`toml::to_string`, one `cargo build`, and rustc says *"found an item that was
configured out … gated behind the `display` feature"* by name. That is a general
shape: **when a dependency's feature set is load-bearing for a safety claim, compile
the thing you claim is impossible and read the error.**

(2) **The `debug-<host>` naming was not one bug, it was a coupling that three
different places had quietly organised themselves around** — and moving to config
names paid all three off at once, which is why the detail's "the row arithmetic is
preserved exactly" turned out to be the load-bearing sentence rather than a
reassurance. live-run session D passed `--host localhost` *solely* so its derived
network name would match A's; the `in:` accretion problem was one `network` row per
host string; and `announce`'s skip was invisible because a mismatched name was
routine. Config names deleted the cause of all three. The lesson for reading the
remaining carry-forward notes: a note that says "X accretes per host string" may not
need a fix at all once the identity underneath it changes — check whether the *cause*
died before designing a remedy.

(3) **Two of the detail's four "surfaced rather than resolved silently" tensions were
discharged by building nothing.** Note 9b's quit hazard needed no drain change,
because autojoin issues no `Request` and nothing autoconnects — the note travels to
stage 2's embedded wiring, where the hazard becomes real. Prompt 9a's name-uniqueness
note needed no validation pass, because the map form makes duplicates a parse error.
Both were *recorded* as discharged-by-absence rather than silently dropped, which is
the only way a later session can tell "considered and unnecessary" from "forgotten" —
and in the 9b case the note itself is now attached to the item that will need it.

**Deviations from the detail, all four:**

- **The multi-network deferral landed under PLAN stage 5 item 1, not stage 6.** The
  detail said "PLAN stage 6" three times; multi-network is stage 5 item 1 in PLAN.md,
  and stage 6 is polish/release. Filed where the work actually is.
- **`parse` deserializes the text twice** — once as `toml::Table` for the
  credential-key scan, once as `Config`. Deserializing `Config` from the already-parsed
  `Table` would have been one pass, but `toml`'s span-carrying errors ("TOML parse
  error at line 4, column 11" with a caret) only exist on the text path, and those
  spans are the whole value of an error in a hand-edited file. Two passes over a
  twenty-line file, with the reason in a comment.
- **session.rs grew rather than shrank** (317 → 353 lines), against the detail's
  expectation. Four flags' worth of clap fields went away; the config read,
  `resolve_network` with its candidate-naming error, the security-warning match and
  the marked SASL injection site are larger than what they replaced. Still 47 lines
  under the ratchet.
- **`realname` is a `const` in config.rs, not derived from `nick`.** The detail said
  `username` and `realname` "stay derived from it in the lowering, as session.rs
  derives them today" — but session.rs derives only `username`; `realname` has always
  been the literal `"supernaut debug session"`. Carried verbatim rather than changed,
  since changing the USER line's realname is an observable protocol change with no
  benefit in this prompt; the constant says why.

**Measurements:** 833 changed lines in `crates/`, 1306 including live-run.sh and docs.
Ratchets: `todo-count 0`, `longest-file 363` (session.rs) against a 400 ceiling —
neither worsened, and no ratchet-forced split was needed for the first time in three
prompts. `cargo test --workspace`: 69 tests, all green, 11 of them new in
`tests/config.rs` and 1 in `storage/tests.rs`. Five new transitive crates
(`toml_parser`, `winnow`, `toml_datetime`, `serde_spanned`, `serde_core`).

**Live run:** `scripts/live-run.sh` — **42/42, exit 0**, up from 40 (session E's two).
Every session now runs from a generated `config.toml` under its own
`SUPERNAUT_CONFIG_DIR`; the string `--host` survives in the script only inside the
comment explaining that it is gone. The claim the detail most wanted checked held
exactly: **A's join moved from a verb to config autojoin and the row arithmetic did
not move.** Confirmed from outside the process — `sqlite3` over A's data dir shows
`#supernaut` seq 1 = alice's autojoin (kind 2), 2 = bob's join, 3 = bob's "the
deployment failed", 4 = alice's autojoin re-fire after the ergo restart, 5 =
xyzzysearchtoken — so `wait rows #supernaut 4 10` and the marker assertions on seq 3
are untouched. One assertion *did* need honest repair, and it was a real hazard rather
than a cosmetic one: A's post-restart sync grepped `-q 'waited rows #supernaut'`, which
now matches the *first* autojoin's echo and would have returned instantly, so it became
a `grep -c … -ge 2` count. The `network` table holds exactly one row, named `liverun`
(not `debug-localhost`), which is the whole point. Session E, same data dir, config
naming `[networks.elsewhere]`: `orphan network liverun: 3 buffers not announced (not in
config)` on stderr, and no `buffer-created … name=#supernaut` — abandonment, observed.

**By-hand acceptance**, all six cases, against a local ergo in an isolated
`SUPERNAUT_CONFIG_DIR` (script kept at `.cache/p10a/byhand.sh`, gitignored):
(1) a hand-written config with `nick`, one `[networks.liverun]` table and
`autojoin = ["#supernaut"]`, run as `session --network liverun --data-dir <tmp>` with
no connection flag of any kind — registered, and `sqlite3` shows `#supernaut` seq 1 is
a Join row by `hazel` with no `join` ever typed. Then the four refusals, each before
anything dialled, each exit 1: (2) a second `[networks.liverun]` table →
`config: TOML parse error at line 4, column 11 … duplicate key`, with the caret, from
the format itself; (3) `password = "hunter2"` → *"`password` is never a config key —
SASL/NickServ credentials live in the OS keyring, never in plaintext in this file"*;
(4) `plaintext = true` with `host = "bnc.example.net"` → *"network bouncer:
`plaintext` permits loopback only; bnc.example.net is not 127.0.0.1/::1/localhost"* —
note this fired *before* `--network liverun` failed to resolve, which is the right
order: the file is wrong regardless of which network was asked for; (5) no file at all
→ *"cannot read config <resolved path>: No such file or directory … write the file, or
point SUPERNAUT_CONFIG_DIR at the directory holding it"*. And (6) `supernaut
--data-dir <tmp>` with no config file anywhere: name/version and history lines, **exit
0** — §3.1's works-before-configuration property, still true.

**Review:** pending.

**Carry-forward consumed:** all seven notes attached to prompt 10a, deleted as one act
with this entry.

- *From prompt 8 — `in:` resolves buffer names across every network row.* Decided, not
  fixed: unscoped, documented on `SearchSpec.buffer` and `run_search`, and pinned by
  `in_filter_unions_one_buffer_name_across_networks`. The note's own premise (reused
  data dirs accrete one `debug-<host>` row per host string) is what died; the union
  that remains is real, and the grammar is stage 2's. See the fourth decision above.
  The note's file reference was stale — `run_search` moved from `storage/exec.rs` to
  `storage/query.rs` in prompt 9a.
- *From prompt 5 — the debug session hardcodes network identity that config must
  replace.* `const NETWORK: NetworkId = NetworkId(1)` is deleted; `SessionState.network`
  is resolved from the file, and the storage network name is the config table key, so
  `debug-<host>` is gone from the codebase entirely. `debug-*` rows are abandoned, not
  migrated — visibly and reversibly (fourth decision).
- *From prompt 6 (config half) — `--tls-ca` and `NetworkSettings.security` are
  installed base.* `tls_ca` is a per-network config key, resolved against the config
  file's directory; the loopback-only plaintext rule is rehomed into config validation
  and `is_loopback` is deleted from session.rs. The note asked where the rule should
  live; the answer is also that `--allow-plaintext` had to die for it (third decision).
- *From prompt 9a — `announce` resolves buffers by network name and silently collapses
  duplicates.* Discharged **by the file format**: two `[networks.x]` tables are a TOML
  parse error, so the collision is unrepresentable and there is no validation pass to
  forget. `announce` is unchanged in that respect and gained a doc sentence saying so;
  the duplicate-table test is what makes it a claim. What *did* change is the silent
  skip beside it, now a loud orphan line.
- *From prompt 9b — `quit` blocks on unanswered requests and exits non-zero.*
  Discharged by absence: this prompt builds no autoconnect, `connect` stays an explicit
  verb, and autojoin issues no `Request`, so `outstanding` holds exactly what each
  live-run segment named and the early-quit failure mode is unreachable. The comment
  above B's `quit` says that, and the warning travels to PLAN stage 2 item 1, where a
  startup-issued request nobody typed becomes real.
- *From prompt 9b — `wait message` no longer counts Join rows; `wait rows` does.*
  Applied: A's autojoin sync is `wait rows #supernaut 1 10`. `wait message` would have
  hung until a human spoke. This note is why the segment works at all.
- *From prompt 9a — live-run session D hard-codes `--host localhost` solely because
  the network name is `debug-<host>`.* Deleted in this same commit, as the note
  required. D now resolves `#supernaut` because its config names `liverun`, the network
  A's config named, and the comment says that instead of explaining a host-string
  accident.

**Carry-forward raised:** two notes to prompt 10b in this file (the single SASL
injection site its keyring replaces; `sasl_account` must leave the credential-key
refusal list and enter the schema in one commit, and must *not* be added to that list
"for symmetry" — an account name is not a secret). Six to PLAN, at the stage where
each belongs: stage 2 item 1 (autoconnect re-opens 9b's early-quit hazard), stage 2
item 5 (`in:` scoping is `/search`'s grammar question, naming the pinned test), stage 2
item 7 (first-run may inherit neither "config is mandatory" nor a silent config write;
and the loopback-only plaintext rule is stricter than §2.3 asks and relaxing it is that
item's call — two notes), stage 4 item 1 (`Search` is the second hole in the
announcement's advisory skip, and needs no id probing), stage 5 item 1 (per-network
nick, SNI `server_name`, and an explicit config `id` *if* a wire NetworkId is ever
persisted). One staleness observed and deliberately **not** fixed here, flagged for the
review instead: PLAN stage 2 item 1's 9a note still says announcements "go out on a
spawned task", which prompt 9b made inline — staled by 9b, so it is 9b's correction to
make, not this prompt's silent edit.

## Prompt 10a — review addendum (answers the `**Review:** pending` line above)

**Date:** 2026-08-10  **Affects:** the prompt 10a entry above; PR #16, second commit

Appended rather than edited into that entry, for the reason 9a's and 9b's addenda
recorded: the entry is committed and pushed, so revising it in place trips
`log-append-only` on the staged diff.

**Review verdict:** the order was executed faithfully and the fence is clean — no
credential surface of any kind, `--sasl`/`SUPERNAUT_SASL_PASSWORD` intact,
`PROTOCOL_VERSION` still 1 with no variant touched, no migration, no `--config` flag,
no new dependency beyond `toml`/`serde`, no autoconnect. The findings were all in the
validation surface rather than the architecture, which is where a first config schema's
bugs live.

**Shipped:** eight fixes.

(1) **The highest finding, and it was a real bug this prompt introduced:
`autojoin` entries were unvalidated for line integrity.** `autojoin = ["#my chan"]`
becomes `JOIN #my chan`, which IRC parses as channel `#my` **with key `chan`** — a
join of the wrong thing, succeeding, with no error from the client, the server, or the
user's own eyes; and an entry containing CR/LF injects a second IRC command outright.
The mistake has a name worth keeping: **the fence said "do not validate channel
names", and I read that as "do not validate autojoin".** Those are different. A channel
*name* is the server's business (CHANTYPES comes from ISUPPORT, so we would be
guessing, which is exactly what the fence forbade); the *line* is ours, because
`connection/mod.rs` joins the entries with `,` into one `JOIN` we write. The order's
own `nick` rule states the principle verbatim — "whitespace breaks the NICK line
itself, which is our bug, not the server's policy" — and it applies unchanged one field
over. Fixed: each entry must be non-empty and free of whitespace and control
characters, with the message naming the network and the entry; two tests, plus a
positive case asserting `["#a", "&b", "weird"]` still parses, so the fix cannot drift
into policing names.

(2) **Whitespace-only network names and hosts passed**, because both checks were
`is_empty()` rather than `trim().is_empty()` — while `nick`, one screen above, was
checked properly. A `[networks." "]` table would have minted a `network` row named
`" "`, which nothing could refer to again and which config could only rediscover by
being edited to contain a quoted space. Both tightened, both tested.

(3) **A zero-network config blamed the wrong thing.** `networks` is
`#[serde(default)]`, so a `nick`-only file is schema-valid and the refusal came from
`resolve_network` in the binary: *"--network is required unless the config file names
exactly one network"* — a message about a flag, to someone whose file has no network in
it. `parse` now refuses it directly (*"no networks; add a [networks.<name>] table with
a `host` key"*), and `resolve_network`'s dead empty-candidates branch is gone with a
comment saying `parse` guarantees non-empty. Tested.

(4) **`refuse_credential_keys` did not descend into arrays.** Unreachable through
today's schema — there is no array-of-tables key — but the entry above claims the
schema "cannot acquire one by accident **in any version**", and a guard that is true
only of the current version does not support that claim. Three lines and a
`refuse_in_value` helper; tested with `[[extras]] password = "…"`.

(5) Three doc pointers in config.rs said "PLAN stage 6" for the multi-network
deferral, which landed under stage 5 item 1 — the entry above recorded the deviation
and then left the comments spelling the wrong stage, which is the version of a stale
doc that is hardest to notice.

(6) `i64::try_from(index).expect("…")` in `into_networks` replaced by
`.zip(1i64..)` — the panic is deleted rather than argued to be unreachable, and the
`+ 1` off-by-one hazard goes with it.

(7) `#[derive(Clone)]` removed from `Config` and `NetworkEntry`; nothing clones
either, and speculative trait impls are the cheap kind of premature abstraction.

(8) **live-run.sh's post-restart sync had no post-loop failure check** — a weakness
the rewrite inherited rather than introduced, but the rewrite is what made it reachable
(the `-ge 2` count replaced a `grep -q`). Without it a reconnect that never completes
falls through into the search, backlog and marker assertions, which then fail for the
wrong reason — an empty corpus — and hide the reconnect as the cause. Now a loud
failure naming the segment, with the a.out and ergo log tails every other sync point
prints.

**Accepted as they stand, with the reasoning recorded rather than re-litigated:**

- **`Config` is publicly constructible around its own validation.** The fields are
  `pub`, `validate` is private, and `into_networks` is `pub` — so code that builds a
  `Config` by hand and lowers it skips the loopback-only plaintext rule, the new
  autojoin rule, and `tls_ca` resolution. Real, and deliberately not restructured
  here: the only plausible second constructor is stage 2's first-run wizard, and the
  choice between "the wizard round-trips through `parse`" and "validation moves into
  `into_networks`" wants that wizard in front of it. Filed as a PLAN stage 2 item 7
  note naming both options and exactly what is skipped.
- **`default_config_path`'s `XDG_CONFIG_HOME` and `$HOME/.config` legs have no test.**
  Only the `SUPERNAUT_CONFIG_DIR` leg is exercised, by every by-hand and live-run
  session. Accepted and recorded rather than papered over: testing env-var precedence
  means either mutating process env in a test (racy across a parallel test binary) or
  extracting a pure helper taking three `Option<OsString>`s, which is a refactor
  arriving for a test rather than for the code.
- **The order's own trailing list contradicted its Acceptance paragraph** — the
  process finding, and the one worth mechanizing. See Learned.

**Learned:** two, both about the order rather than the code.

(1) **A budget bullet's descope list must be disjoint from the Acceptance paragraph.**
10a's budget listed session E and the orphan line as the first thing to trail if the
size cap bound, while its Acceptance paragraph required session E to run and say
`orphan network liverun` out loud. Both were followed, so nothing broke — but had the
cap actually bound, the two instructions would have been in direct contradiction and
whichever one got read second would have won silently. The rule: **if a step is in
Acceptance, it cannot be on the trail list; if it is trailable, take it out of
Acceptance.** Not mechanized as a check — it is a property of prose in a
just-in-time-written prompt, which no grep can see — so it is recorded here for the
next detail-writing sub-agent to be given.

(2) **"Do not validate X" fences need to name the *reason*, not just the field.**
Finding (1) landed because the fence said "channel names in `autojoin` are
deliberately not validated" and the reason (CHANTYPES is ISUPPORT's, so we would be
guessing) was in the same sentence — but I generalised from the field to the whole
value. Had it read "do not guess at channel *name* syntax; line integrity is still
ours", the bug is unwritable. The fence was right and the reading was lazy; the fix to
that class is fences that state the boundary rather than the exclusion.

**Measurements after the fixes:** 931 changed lines in `crates/` (was 833; still
oversize, still justified above — the eight fixes added ~100 lines, most of them
tests). config.rs 282 lines, tests/config.rs 317, session.rs 351. Ratchets unchanged:
`todo-count 0`, `longest-file 363` against a 400 ceiling. `cargo test --workspace`: 72
passing, 0 failing — 3 more than before, all in `tests/config.rs`, which now has 14.

**Live run:** `scripts/live-run.sh` re-run twice after the fixes, because config.rs's
validation and the script itself both changed: **42/42, exit 0** both times. Nothing in
the run's shape moved — the seq arithmetic still lands the marker on seq 3 and the
autojoin re-fire on row 4, the `network` table still holds exactly one row named
`liverun`, and session E still reports `orphan network liverun: 3 buffers not announced
(not in config)` with no `#supernaut` announcement. The new post-loop check did not
fire, which is the correct outcome and not evidence it works; it was verified by
reading, not by breaking the reconnect on purpose — recorded honestly rather than
claimed.

**Review:** the review's findings are all dispositioned above — eight fixes applied
(one a real bug in shipped behaviour, four validation gaps, three hygiene), three items
accepted with reasons, **none rejected**. Its highest finding was correct and its
proposed remedy was sufficient, which is the opposite of 9b's experience and worth
noting for the same reason.

**Carry-forward raised:** seven notes, none rejected. **Four to prompt 10b** in
STAGE-1-PROMPTS.md, all four about the keyring's blast radius rather than its
mechanism: key entries by the config network *name* and never by `NetworkId` (which
`into_networks` renumbers whenever a network is added alphabetically earlier — a
keyring entry would be the first thing to persist a wire id, and would silently
re-point one network's credentials at another's); resolve the secret for the *selected*
network only, since the map deliberately holds every configured one; live-run's
credential surface is two lines and `write_config` is fixed-shape, so deleting the env
bridge means extending the helper; and the stage-1 acceptance run against Libera now
needs a hand-written config in an isolated `SUPERNAUT_CONFIG_DIR`, which is an explicit
step to budget rather than a one-liner. **One to PLAN stage 4 item 3**: the
orphan-network report is engine stderr only, so under the daemon the attaching client
sees nothing and the "made visible" half of the abandonment decision stops holding —
it needs a frame representation (gated on the handshake, since v1 forbade the variant)
or a documented log location. **Two to PLAN stage 2**: item 1, the plaintext warning
fires once at session start for the selected network only, so autoconnect dialing
several networks would connect a plaintext one with no warning at all; item 7, config
validation is a property of `parse` rather than of the `Config` type.

**Also fixed while in PLAN.md, and recorded rather than done silently:** stage 2 item
1's prompt-9a note said announcements "go out on a spawned task", which prompt 9b made
inline. The entry above flagged it and deferred it as 9b's staleness to correct; the
review confirmed it now reads wrong, so it is corrected here at first touch — the
ordering hazard the note is *about* survives 9b's change (the storage jobs behind the
announcement and the response still complete in their own order), so the note keeps its
warning and loses its wrong mechanism, with a parenthetical saying it was corrected.
Fixing a doc at the moment you touch it beats filing a ticket against yourself.

## Decision — `keyring` 4.1.6 lives in the binary, at the platform default store

**Date:** 2026-08-10  **Affects:** `crates/supernaut/Cargo.toml`,
`crates/supernaut/src/credentials.rs`

**Chose:** `keyring = "4.1.6"` with default features (`default = ["v1"]`) declared in
**`crates/supernaut`**, wrapped by a `credentials` module of ~140 lines. `keyring` was
already on `DEP_ALLOWLIST` in `scripts/check-docs.sh`, seeded from NORTH-STAR §5, so no
allowlist edit and therefore no `scripts/test-checks.sh` fixture is owed — the decision
entry is.
**Over:** (a) **havoc-core owning the store.** Rejected: core's test suite would then
depend on the developer's login keychain and on OS state, which is the opposite of the
"no file I/O, everything testable from a string" property `config.rs` was built for one
prompt ago. A credential store is an OS integration belonging to whichever process
assembles the core — today `supernaut`, at stage 4 `havocd`, a binary either way — and
the type that crosses the seam, `SaslCredentials`, is already core's. (b) **`keyring-core`
plus `apple-native-keyring-store` wired up directly.** Rejected because what we want is
precisely the platform default, which is exactly what the `keyring` façade *is*; naming
the store ourselves buys control we have no use for and a second thing to keep in step
with the platform.
**Because:** the seam is one function each way (`load`, `set`) over a stable façade, and
the alternative that matters — a *non*-default store — is the deferred file fallback,
which is what `keyring-core` exists for and where this gets revisited.
**Measured, because the number is worse than the shape suggests:** `Cargo.lock` goes from
**107 to 202 packages** — 95 new entries for a credential store — because keyring 4's
`v1` feature enables *every* platform backend in the manifest and the gating is by target
at build time, not by feature at resolve time. What actually compiles on
`aarch64-apple-darwin` is eight crates: `keyring`, `keyring-core`,
`apple-native-keyring-store`, `security-framework`, `security-framework-sys`,
`core-foundation`, `core-foundation-sys`, plus `log` (and `bitflags`/`libc`, already in
the tree). The other 87 — `zbus`, `secret-service`, `aes`, `num-bigint`, `async-io`,
`regex`, `tracing`, `uuid`, `toml_edit` and the rest — are the Linux and Windows store
backends and their transitive tails, resolved and locked but never built here. That is
still real: a lock entry is a supply-chain surface, `cargo audit` reads it, and a CI
runner on Linux *would* compile the zbus half. Recorded rather than glossed, because the
tidy `cargo tree` for this target is the misleading view.
**Revisit if:** we need a non-default store — the file fallback, a scratch keychain for
CI, or a Linux target where the Secret Service is not the right answer. Then this becomes
`keyring-core` plus exactly the stores we name, which also cuts the 95 lock entries down
to the ones we use. A CI runner that builds on Linux is the other trigger: the compiled
set there is the zbus tail, and it should be measured before it is trusted.

## Decision — the encrypted-file credential fallback is deferred, and its absence is made loud

**Date:** 2026-08-10  **Affects:** `crates/supernaut/src/credentials.rs`, `PLAN.md`
stage 1 item 10 and stage 4 item 3

**Chose:** ship the keyring half of §5.8's "keyring with encrypted-file fallback" only,
and make the missing half speak: when the platform store is unavailable for any reason
other than "no entry", `credentials::describe` says the OS keyring is unavailable, quotes
the platform error, states that **there is no encrypted-file fallback yet**, and names
the PLAN deferral by item. Filed on PLAN stage 4 item 3, due by stage 6 item 3.
**Over:** (a) **building it now with `argon2` + `chacha20poly1305`.** Two dependencies off
the allowlist, bought for a consumer that does not exist: the dogfood machine is a Mac
with a working login keychain. (b) **keyring 4's own `db-keystore`.** Checked rather than
assumed, and it is worse: it is Turso-backed — a *second* SQLite implementation beside
rusqlite — and it still takes its data-encryption key as a hex string handed in by the
application, so the key problem is exactly as unsolved and the dependency is far larger.
(c) **a file "encrypted" with a key stored beside it.** Rejected as the worst of the
three: obfuscation labelled encryption is worse than no fallback, because it reads as
safe.
**Because:** a fallback file needs a key, and every honest source of one is out of stage
1's reach. A passphrase needs a no-echo prompt, and the terminal belongs to stage 2 item
7; a machine-derived key is the obfuscation option; a key in another store is the store we
are trying to substitute for. The boundary that keeps this honest is **absence must be
loud** — the deferral is a sentence a user reads, not a gap they discover when SASL
silently does not happen.
**Revisit if:** the first target without a working platform store — which is exactly PLAN
stage 4 item 3's daemon started at login against a locked Secret Service, or a headless
Linux box. `keyring-core` with an explicit store is the route; the key's provenance is the
design question, not the file format.

## Decision — keyring entries are keyed by the config network name, and nothing else

**Date:** 2026-08-10  **Affects:** `crates/supernaut/src/credentials.rs`

**Chose:** service `"supernaut"`, account = the **config network's name** (`liverun`,
`libera`), one entry per network.
**Over:** (a) **`NetworkId`.** Rejected on the carry-forward's reasoning:
`Config::into_networks` numbers networks `1..N` from `BTreeMap` order, so adding
`[networks.aardvark]` renumbers everything after it. A keyring entry keyed by id would be
the first thing in this program to persist a wire id, and it would silently re-point one
network's credentials at another's after an edit that had nothing to do with credentials.
(b) **`<network>:<account>`.** Rejected: it makes `credential set` need an account
argument config already holds, and editing `sasl_account` would silently orphan the
secret you stored five minutes ago. With the name alone, a mismatched account fails
loudly at the server — fail-closed, and honest. (c) **hashing the config path into the
service string.** Rejected, though it fixes a real cost.
**Because:** the name is the stable key, and it is what storage keys on too, so there is
one identity in the system rather than two. **What (c) would have fixed, said out loud in
the module doc rather than left to be discovered:** the keychain namespace is
process-global, so one `supernaut`/`liverun` item is shared by *every*
`SUPERNAUT_CONFIG_DIR` on the machine — two config dirs naming a network `liverun` share
one secret. The price of fixing it is an entry nobody can find in Keychain Access and
nothing can share with stage 4's daemon, which is a worse trade for a single-user client.
**Revisit if:** one machine ever runs two independent supernaut *installations* that must
not see each other's credentials — a shared build box, or per-project config dirs holding
different accounts on the same network. Then the service string grows a discriminator, and
it should be human-readable (`supernaut:<profile>`), not a hash.

## Decision — `credential set` reads stdin with echo on, rather than prompting

**Date:** 2026-08-10  **Affects:** `crates/supernaut/src/credentials.rs`,
`crates/supernaut/src/main.rs`, `PLAN.md` stage 2 item 7

**Chose:** `supernaut credential set <network>` reads the secret from **stdin only**,
trims exactly one trailing newline, refuses empty, and — when stdin is a TTY — prints one
stderr line saying the input is not hidden and showing the pipe form.
**Over:** (a) **argv (`--password`).** Never: `ps` is world-readable, which is the reason
prompt 6 refused `--sasl-pass`. (b) **an interactive no-echo prompt**, either via a
password-reading dependency or hand-rolled raw mode. Rejected for now. (c) an env var,
which is what this prompt *deleted*.
**Because:** stdin is the one seam that serves a human and a pipe identically, and it is
what lets `scripts/live-run.sh` exercise the product's real write path instead of a
harness-only shortcut. Suppressing echo is either a new dependency or raw-mode terminal
code, and the terminal belongs to stage 2 item 7's first-run, which owns credential entry
as a product experience. Shipping a half-hidden prompt now would put terminal code in the
one binary that is supposed to have none. The TTY warning is the honest interim: the
failure mode of a visible password is a person who does not know it was visible.
**Revisit if:** stage 2 item 7 lands the first-run wizard — it adds a hidden prompt
*beside* the pipe, and does not replace it; a pipe is how the next machine gets a
credential without a human.

## Decision — `sasl_account` beside `plaintext = true` is a config error, not a warning

**Date:** 2026-08-10  **Affects:** `crates/havoc-core/src/config.rs`, `PLAN.md` stage 2
item 7

**Chose:** `Config::validate` refuses the pair by name, before anything dials, with a
message that says SASL PLAIN puts the secret on the wire and plaintext leaves it
readable.
**Over:** a loud warning that still connects. Rejected.
**Because:** §2.3's shape is that you say what you trust and never switch trust off, and
this pairing does switch it off — it is the one combination where a *credential* is the
thing exposed, so the loopback-only rule beside it bounds the blast radius to the box but
does not make it fine. 10a's review flagged exactly this pairing under the old `--sasl`
flag, when the flag made it a runtime accident; in config it is a written statement, and a
written statement that leaks a password should not be honoured. A warning that connects is
a warning nobody reads twice.
**Revisit if:** stage 2 item 7 relaxes the loopback-only plaintext rule for a LAN
bouncer — then a user authenticating to that bouncer is the real case, and the two rules
must be decided together rather than one of them quietly outliving the other. Filed as a
carry-forward note there naming both.

## Decision — the stage-1 acceptance run moves to OFTC after Libera network-banned this IP

**Date:** 2026-08-10  **Affects:** the prompt 10b entry below, `PLAN.md` stage 1
Done-when and **Still open**, stage 3

**Chose:** run stage 1's acceptance sequence against **OFTC** (`irc.oftc.net`, TLS 6697,
stock webpki roots), and record the Libera clause of the Done-when as an open,
user-owned gap.
**Over:** (a) **retrying Libera.** Refused: mid-prompt, Libera answered
`465 … You are banned from this server- Your bot is not permitted to connect to Libera
Chat … (2026/8/11 02.58)` and closed the link. A ban is an instruction, not an error to
work around, and the reconnect backoff knocking on it repeatedly is the specific harm to
avoid — so the acceptance script gained a hard ban-guard that SIGKILLs the session on
`465`/`464`/`ERROR` instead of letting the actor retry. (b) **appealing the ban** by
mailing `bans@libera.chat`. Not ours to send: it is the user's identity, the user's IP and
the user's relationship with a volunteer network. (c) **shopping for a third, busier
network** until the search leg produced a stranger's chat line. Rejected after one
substitution: each new connection from a freshly-banned IP with a realname that literally
says "debug session" is another chance to get the user banned somewhere else, and the
value of a second opinion is far below that cost. (d) **skipping the live acceptance run**
and closing the stage on `scripts/live-run.sh` alone. Rejected outright — the whole point
of the Done-when is a real network.
**Because:** what the Done-when actually tests is TLS against a public network on the
stock root store, autojoin from a config file, real traffic landing in SQLite, `kill -9`,
restart over the same data dir, and search with filters over what the dead process wrote.
OFTC proves all of that. It cannot prove SASL — it advertises no `sasl` capability at all
— and neither could Libera without a registered account, so the SASL clause was
unprovable at a public network in this session either way, which is why prompt 6's honest
precedent (verify what ran, name what did not) is the one followed.
**Revisit if:** the Still-open ban/account question is answered — then the acceptance
sequence is re-run at Libera with SASL from the keyring, which is the one clause still
owed, and it is a rerun of an existing script rather than new work.

## Prompt 10b — Credentials and the stage acceptance run

**Commit:** branch `prompt-10b-credentials` (PR open)  **Date:** 2026-08-10

**Shipped:** the secret left the environment for the OS keychain. Config gains exactly
one credential-adjacent key — `NetworkEntry::sasl_account`, an account *name* — and it is
deliberately **not** on `CREDENTIAL_KEYS`, whose four names stay and whose message is
upgraded from aspiration to instruction (it now names `sasl_account` and
`supernaut credential set <network>`). Validation refuses empty-after-trim, whitespace and
control characters in the account, stating the boundary rather than the exclusion: what a
network permits as an account name is the network's business, but `authcid\0authcid\0
password` is our payload to frame. `sasl_account` beside `plaintext = true` is a refusal,
by name, before anything dials. New `crates/supernaut/src/credentials.rs` (138 lines) owns
the store: `keyring = "4.1.6"`, service `"supernaut"`, account = the config network's
*name*, `load` for the session and `set` for the new
`supernaut credential set <network>` subcommand — stdin only, never argv, one trailing
newline trimmed, empty refused, one confirmation line naming the entry and never the
secret, and one stderr line when stdin is a TTY saying the input is not hidden. Three
distinct failures, all before dialling and all non-zero: `NoEntry` names the service, the
account and the exact command to run; `Ambiguous` says how many items to de-duplicate in
Keychain Access; anything else says the keyring is unavailable, quotes the platform error,
and names the deferred file fallback by PLAN item. `--sasl` and `SUPERNAUT_SASL_PASSWORD`
are **deleted**, not deprecated, and `load_config` is lifted into main.rs so `session` and
`credential set` say "cannot read config" identically. The lookup replaced the env read at
the one existing injection site, for the **selected network only**, with the account name
captured before `into_networks` consumes the config, and a comment saying why it is not a
loop. `--trace-irc` no longer prints the SASL payload: a `redact_outbound` helper in
`connection/actor.rs` turns `AUTHENTICATE <base64>` into `AUTHENTICATE <redacted>` at the
three outbound trace sites — at the trace only, never at `send_line`, so the wire and the
state-machine corpus are untouched. `scripts/live-run.sh` seeds and removes its own
keychain item through the product's own write path under a bounded watchdog, and asserts
that neither the fake password nor its base64 appears in `a.trace`, `a.out` or
`.cache/last-a.trace`. No wire change (`PROTOCOL_VERSION` stays 1), no storage schema
change, no migration. Six decisions above were made while doing this and are recorded
there.

**Deviations:** three, and the first is the one that matters.

- **The stage acceptance run ran against OFTC, not Libera.Chat, because Libera
  network-banned this IP mid-prompt.** Recorded as its own decision entry above, with the
  four rejected alternatives (retry, appeal, shop for a third network, skip). The
  coordinator had already settled that the Libera leg would be TLS-without-SASL, since no
  registered account exists; the ban removed the host as well. `PLAN.md` stage 1's
  Done-when now carries a paragraph naming exactly which clause went unproven, and
  **Still open** carries the two user-owned questions (an account; the ban appeal).
- **A sixth decision entry, and three PLAN notes the order did not name.** The order named
  five decisions and three PLAN edits; the ban forced the sixth decision, and the
  acceptance run itself surfaced two notes that are more valuable than anything the code
  changed: stage 2 item 5 gets "on a network without `echo-message`, nothing you say is
  ever stored" (observed, see Learned) and stage 3 gets "the USER realname is the const
  `supernaut debug session`, and a real network read it as a bot and banned us".
- **`scripts/live-run.sh`'s new never-in-logs loop got a file-existence check the order did
  not ask for.** `grep -qF` on a missing path fails, and the loop read that as "the secret
  is not in there" — a check that fails open, silently, which is the exact failure mode
  CLAUDE.md's "a broken check fails open, and open is silent" names. Two lines, and the
  reason is in a comment.

**Deferred:** the encrypted-file fallback, with its own decision entry above, filed on
PLAN stage 4 item 3 and due by stage 6 item 3; its absence is a sentence the user reads
rather than a gap they discover. Nothing else from this prompt's scope was left undone —
the trailable item in the budget (`credential set` refusing a network with no
`sasl_account`) shipped, because the change came in at 313 lines against a cap of 800.

**Learned:** four, in descending order of what they would have cost later.

(1) **A real network's capability set is the thing that decides whether your own words
exist.** OFTC answered `CAP * LS :multi-prefix` — one capability — so the client sent no
`CAP REQ` at all, and the line it PRIVMSGed into its own channel is absent from `message`,
from `backlog` and from `search`, while every inbound line the network sent landed and was
searchable. Ingestion is inbound-only by design and `echo-message` is what makes our own
text inbound; prompt 9b's carry-forward note said as much about *composer state*, and the
consequence for *history* is a strictly bigger deal that nobody had written down. It cost
this run its planned search corpus (the token line was never stored, so the first three
searches came back `hits=0`) and it was recovered by searching what the network itself had
sent — ChanServ's `#debian` welcome and the AUTH notices, real text written by the process
that got killed. The general shape: **when a design says "the server tells us X", test
against a server that does not.** Filed on PLAN stage 2 item 5 with stage 3 as the
deadline.

(2) **A ban is a measurement, and the thing it measured was our realname.** Libera's
`465` says *"Your bot is not permitted to connect"*. Two candidate causes and both
happened: a survey run that autojoined ten channels from one client, and a USER line whose
realname is the const `"supernaut debug session"`. Prompt 10a carried that const verbatim
on the reasoning that changing the USER line is "an observable protocol change with no
benefit" — the benefit has now been supplied by a network refusing us. The transferable
lesson is narrower than "be careful": **a string the program sends to strangers is not an
implementation detail, and a placeholder in one is a promise to be misread.** Filed on
PLAN stage 3, because a dogfood month cannot start on a network that bans you on day one.

(3) **The keychain-ACL fear was unfounded, and settling it took one script.** The order
flagged as unverified whether `security delete-generic-password` on an item created by
*another* application (our ad-hoc-signed debug binary, whose cdhash changes every rebuild)
prompts for authorization. It does not: probed in order, `delete` on nothing → exit 44,
`credential set` → exit 0, `find` → exit 0, **`delete` on the product-created item → exit
0, no dialog, no watchdog trip**, `find` again → exit 44. So the harness uses the product's
own write path throughout and the documented `security add-generic-password -A` fallback
was never needed. The watchdog stays anyway — it costs 20 lines and it is the difference
between a loud failure and a CI run that hangs forever on an invisible dialog.

(4) **95 lock entries for a credential store, 8 of them compiled.** Measured in the
dependency decision above rather than discovered at audit time. `cargo tree` for this
target shows a tidy eight-crate tail and is the misleading view; `Cargo.lock` nearly
doubles, 107 → 202, because keyring 4's `v1` feature enables every platform backend in the
manifest and gates them by *target* at build time rather than by *feature* at resolve time.
The habit worth keeping: **for a new dependency, count the lock, not the tree.**

**Measured:** 313 changed lines in `crates/` (271 added, 42 deleted) against the 800-line
tripwire, and 1922 across the whole diff including `Cargo.lock`'s 95 new packages,
`scripts/live-run.sh` and the docs. `cargo test --workspace`: **74 tests, all green**, up
from 72 — three new in `tests/config.rs` (the PLAIN-payload validation, the
`plaintext` pairing, and `sasl_account` pinned as *not* a credential key) and one in
`actor.rs` (the redactor over four line shapes). Ratchets: `todo-count 0`,
**`longest-file 399` against a ceiling of 400 — one line of headroom**, and the file at the
wall is **`crates/havoc-core/src/connection/actor.rs`** (373 → 399; the 26 lines are the
redactor and its test), not the test file the order expected to crack first
(`crates/havoc-core/tests/config.rs`, 317 → 393, second at the wall; `session.rs` 355).
Left at 400 rather than tightened to 399, because the value **worsened** and a ratchet only
turns one way on an improvement — but the consequence is stated here so it is not a
surprise: **the next prompt that adds a line to `actor.rs` fails `make check`, and the
remedy is to split the file, not to raise the ceiling** (raising it is a decision entry).
`redact_outbound` plus its test moving to a `connection/trace.rs` of their own is the
obvious cut, and it was deliberately not made here — the order placed the helper and its
test in `actor.rs`, and restructuring a module after the live runs and before the review is
churn buying nothing this prompt needs. `credentials.rs` is 138 lines, `config.rs` 331. Clean `cargo build -p supernaut` after the dependency landed: 9.33s.

**Live run:** four separate exercises, and the fourth did not go as planned.

**1. `scripts/live-run.sh` — 46/46, exit 0, run twice** (up from 42; the four new
assertions are the redaction line and the three never-in-logs files). Session A now
authenticates with a credential that exists in nothing it can see: not its config (which
holds `sasl_account = "alice"` and no secret), not its argv, not its environment. The
regression net prompt 6 left in place still fires — `>> AUTHENTICATE PLAIN` and
`903 … authentication successful` are both in `a.trace` — which is what proves the keyring
produced a *real* credential rather than an empty string, and the payload line beside them
now reads `>> AUTHENTICATE <redacted>`. Neither `fake-livetest-passw0rd` nor its
`alice\0alice\0…` base64 appears in `a.trace`, `a.out` or `.cache/last-a.trace`. Sessions
B–E never touch the keychain (no `sasl_account` in their configs), and after the run
`security find-generic-password -s supernaut -a liverun` exits 44 — **the harness left no
credential behind**, verified after both runs. `security … -g` appears nowhere in the
repository.

**2. The keychain-ACL probe** (`.cache/p10b/probe-keychain.sh`, gitignored): the five-step
sequence in Learned (3), exit codes 44/0/0/0/44, no authorization dialog at any step, and
the item confirmed to live in `~/Library/Keychains/login.keychain-db` — which is what the
live-run header comment tells the reader it does to their machine.

**3. The by-hand ergo acceptance, all four cases, isolated `SUPERNAUT_CONFIG_DIR`**
(`.cache/p10b/byhand-ergo.sh`, gitignored), against a local ergo with a self-signed cert
and `alice` pre-registered with NickServ. (1) A config naming `sasl_account = "alice"` with
no keyring entry: **exit 1, nothing dialled**, and the message is
*"no SASL password for network liverun: the OS keyring holds no item for service
supernaut, account liverun. Store one: printf %s 'your-password' | supernaut credential set
liverun"*. (2) `printf %s … | supernaut credential set liverun` → exit 0 and one line,
*"stored a password in the OS keyring: service supernaut, account liverun (SASL account
alice)"* — the entry named, the secret not. (3) The identical session command again: exit
0, registered, autojoined, and the trace's SASL exchange verbatim —
`>> AUTHENTICATE PLAIN`, `<< … AUTHENTICATE +`, `>> AUTHENTICATE <redacted>`,
`<< … 900 … You are now logged in as alice`, `<< … 903 :Authentication successful` — with
zero occurrences of the fake password or its base64 in either stdout or the trace. (4)
`plaintext = true` beside the account key: **exit 1 from `config::parse`**, *"config:
network liverun: `sasl_account` with `plaintext` would send the password in cleartext …
Drop one of the two keys."* Then the keychain item removed, `find` exit 44.

**4. The stage acceptance run — at OFTC, not Libera.Chat.** Libera first: with the
hand-written config in a scratch `SUPERNAUT_CONFIG_DIR` and no `sasl_account`, the run got
`<< :erbium.libera.chat 465 supernaut-smoke :You are banned from this server- Your bot is
not permitted to connect to Libera Chat. Please email bans@libera.chat if you think this
network ban was set in error. (2026/8/11 02.58)` followed by
`<< ERROR :Closing Link: … (*** Banned )`. Not retried; see the decision entry and
Learned (2). Substituted OFTC, one connection, two autojoin channels
(`#supernaut-smoke` quiet, `#debian` busy), no speaking in the busy one, a short lurk, a
clean quit, and a ban-guard that would SIGKILL on another refusal.

- **Config, hand-written, verbatim:** `nick = "supernaut-smoke"`, one `[networks.oftc]`
  table with `host = "irc.oftc.net"`, `autojoin = ["#supernaut-smoke", "#debian"]`. **No
  `port`, no `tls_ca`, no `plaintext`, no `sasl_account`** — 6697 and the stock
  webpki-roots store are the defaults being proven.
- **Registered 12s after the process started**, TLS on the stock root store with no
  configured anchor. Both autojoined channels had produced a stored row **0s later** —
  neither `join` was ever typed. Five buffers appeared: `AUTH`, `CTCPServ`,
  `#supernaut-smoke`, `#debian`, `ChanServ`.
- **Traffic, as observed rather than as hoped:** across a 180s lurk, `#debian` produced
  **zero** chat lines — 04:30 UTC in a support channel — and, per Learned (1), our own
  PRIVMSG was never stored because OFTC advertises only `multi-prefix`. What *did* land was
  real inbound text the network sent: four `AUTH` notices, ChanServ's `[#debian] Welcome to
  #Debian…` notice, CTCPServ's `VERSION` request, our two joins and a `+n +t` mode line.
  **9 rows, 9 `message-added` events, 5 storage commits.**
- **`kill -9`, and the batch-timer question answered with a number:** `9` rows before the
  kill (read by `sqlite3` from outside the process), `9` after. **Difference: 0 rows.** Not
  "nothing was lost" as a promise — the measurement, on a run whose last write was ~2
  minutes before the SIGKILL, which is exactly the case where a ~100ms batch timer has
  nothing in flight. A kill mid-flood is still capable of losing one batch; this run had no
  batch to lose, and that is what is claimed.
- **Restart over the same `--data-dir`, identical command:** the attach announcement
  resolved **all five** buffers before the process dialled anything, and
  `backlog #debian after:0 50` / `backlog #supernaut-smoke after:0 50` read back lines
  written by the dead process. The three planned searches returned `hits=0`, correctly and
  for the reason above: the corpus held no chat text.
- **The search leg, closed against the real text that did land** (`.cache/p10b/
  oftc-phase3.sh`, gitignored) — a session over the same data dir with `connect` never
  typed, so **`event connection-state` count 0: it dialled nothing**. Four searches over
  what a real network wrote and a killed process stored: `search hostname` → **2 hits**
  (`*** Looking up your hostname...`, `*** Couldn't look up your hostname`);
  `search from:ChanServ Debian` → **1 hit** (the `#debian` welcome);
  `search in:ChanServ "support channel"` → **1 hit**; `search in:AUTH "look up"` → **1
  hit**, and notably not the `Looking up` line, which is FTS5 tokenisation being visible
  rather than a bug. **Four searches, 0.036s wall time**, measured around the whole block.
- **What could not be verified live, named rather than left implicit:** **SASL against a
  public network.** OFTC advertises no `sasl` capability at all, and Libera would have
  needed a registered account which does not exist — so the clause was unprovable at a
  public network in this session by two independent causes. SASL from the keyring is
  proven against local ergo, twice by hand and on every `scripts/live-run.sh` run, which
  is the same honest shape prompt 6 recorded. Both halves of the gap are in PLAN's
  **Still open** with the user named as their owner, and PLAN stage 1's Done-when now says
  so in place.

**Review:** pending.

**Carry-forward consumed:** all seven notes attached to prompt 10b, deleted as one act
with this entry.

- *From prompt 6 (credentials half) — the env bridge must be deleted, not kept alongside.*
  Done: `--sasl` and `SUPERNAUT_SASL_PASSWORD` are both gone from `session.rs`, the module
  doc says why (a secret in the environment is the same plaintext §5.8 forbids in the
  file, visible in `ps eww`, inherited by every child), and `scripts/live-run.sh` has no
  other path to a credential — so the harness proves the real surface rather than a
  fallback. Fixture credential is `fake-livetest-passw0rd`.
- *From prompt 10a — the SASL injection site is one place, and it is the place the keyring
  replaces.* Applied exactly: the shape did not move, only where the two halves come from.
  `into_networks` still lowers `sasl: None` (its doc now says where the secret comes from
  instead of merely that it is absent), and `run()` joins the account name from config to
  `credentials::load`'s half after lowering.
- *From prompt 10a — `sasl_account` must leave the refusal list and enter the schema in one
  commit, and must not be added to that list "for symmetry".* Both halves done in this
  commit, and the "for symmetry" trap is now **pinned** by
  `sasl_account_is_not_treated_as_a_credential_key`, which also asserts the four secret
  names still refuse beside it and that the refusal message names `credential set`. The
  list's doc comment says why the account name is not on it: the message says credentials,
  and the lie would be in the error a user reads.
- *From prompt 10a — key entries by the config network name, never `NetworkId`.* Done, with
  its own decision entry recording the two rejected alternatives and the cost of the one
  chosen (one keychain namespace across every `SUPERNAUT_CONFIG_DIR`, said out loud in the
  module doc).
- *From prompt 10a — resolve the secret for the selected network only.* Done, with the
  comment the note asked for so nobody "fixes" it into a loop over the map. Verified from
  outside: live-run sessions B–E have no `sasl_account` and touch the keychain zero times.
- *From prompt 10a — the live-run credential surface is two lines and `write_config` is
  fixed-shape.* The helper gained a fifth optional `sasl_account` argument emitted from an
  `if` block, never a trailing `[ -n … ] &&` test (which under `set -e` would make an
  absent value the function's failing last command — the comment already there says why).
  The credential now reaches the store through `"$BIN" credential set liverun` under a
  watchdog, with `security delete-generic-password` before the seed and again in
  `cleanup()` *before* the `KEEP_WORK` early return.
- *From prompt 10a — the acceptance run needs a hand-written config in an isolated
  `SUPERNAUT_CONFIG_DIR`, budgeted as its own step.* Budgeted, and the budget was the
  right call: the step consumed a Libera ban, a network substitution, a capability
  discovery and a third script. Had it been treated as a one-liner at the end of the
  prompt it would have been discovered with no time left to be honest in.

**Carry-forward raised:** none to a later prompt in `STAGE-1-PROMPTS.md` — 10b is the last
prompt of stage 1, so every note goes to `PLAN.md` at the stage that will need it. **Five,
all recorded above at the moment they were raised:** stage 2 item 5 (nothing you say is
stored without `echo-message` — the history half of a note that previously existed only as
a composer-state warning; deadline stage 3); stage 2 item 7 (two notes: there is no no-echo
password prompt and this item owns the terminal that would give it one; `sasl_account` +
`plaintext` is stricter than §2.3 asks and relaxing it is the same call as the loopback
rule beside it); stage 3 (the USER realname is a const that says "debug session" and a real
network read it as a bot); stage 4 item 3 (the encrypted-file fallback, with stage 6 item 3
as the deadline and the requirement that `credentials.rs`'s error text be updated in the
commit that closes it). Plus two entries on PLAN's **Still open** list, both user-owned and
both marked *(not blocking)*: a registered account to prove SASL with, and the Libera ban
appeal. None came from a harvest sub-agent — the post-prompt review has not run yet, and
its findings will be dispositioned in an addendum below, as 9a, 9b and 10a did.

## Prompt 10b — review addendum (answers the `**Review:** pending` line above)

**Date:** 2026-08-10  **Affects:** the prompt 10b entry above; PR #17, second commit

Appended rather than edited into that entry, for the reason 9a's, 9b's and 10a's addenda
recorded: the entry is committed and pushed, so revising it in place trips
`log-append-only` on the staged diff.

**Review verdict:** the order was executed faithfully and the fence is fully clean — no
second credential store of any kind, no `credential get`/`status`, no interactive prompt,
no password-shaped key in the schema, `sasl_account` correctly absent from
`CREDENTIAL_KEYS`, no map walk, nothing keyed by `NetworkId`, no mechanism key, the
connect-time SASL failure policy untouched, redaction at the trace and not at `send_line`,
no autoconnect, `PROTOCOL_VERSION` still 1, no migration, and no dependency beyond
`keyring`. The findings were about **whether the new guarantees actually hold**, not about
the shape of the design — and the highest one is the best kind: an assertion that passed
for the wrong reason.

**Shipped:** six fixes.

(1) **The highest finding, and the assertion it fixes was provably inert:
`scripts/live-run.sh` computed the never-in-logs base64 with the wrong SASL framing.**
It used `printf 'alice\0alice\0%s'` — `authcid\0authcid\0password` — but SASL PLAIN is
`authzid\0authcid\0password` and `connection/caps.rs` sends an **empty authzid**, so the
wire carries `\0alice\0…`; caps.rs's own unit test pins `\0alice\0sesame` and the entry
above quoted the wrong shape too. The check was therefore searching every log for a string
the program can never emit. **Verified by breaking the redactor on purpose**, which is the
only way to know an absence-assertion works: with `redact_outbound` returning `line`
unchanged, the corrected check FAILs on `a.trace` and on `.cache/last-a.trace` (and the
`<redacted>` assertion fails beside it), the run exits 1 at 43 ok. In that same leaking
trace, `grep -cF` finds the **corrected** payload **twice** and the **old** payload
**zero** times — so the fix is not a tidy-up, it is the difference between a live check and
a decorative one. Redactor restored, and the line now carries a comment saying which
framing is real and that this version was verified by breaking it.

(2) **The watchdog's own FAIL line was silenced for both `security` calls.**
`keychain_forget` wrote `with_watchdog … >/dev/null 2>&1 || true`, and that redirection
applies to the *function*, so it swallowed the guard's `FAIL: … did not finish in 10s`
along with `security`'s chatter — silencing precisely the line whose absence the watchdog
exists to prevent, in the failure mode (an invisible authorization dialog) that is
otherwise indistinguishable from a deadlock. `with_watchdog` now takes a `quiet|loud`
argument that redirects the **inner command only**; `keychain_forget` passes `quiet`, the
`credential set` seeding passes `loud`.

(3) **`longest-file` was 399/400 with `actor.rs` at the wall, and "ratchets not worsened"
did not hold** — 363 → 399 is a worsening even though it stayed under the ceiling, and the
entry above recorded the number while declining the cut. The review is right that a stage
boundary is the wrong place to hand the next prompt an unbudgeted split, so the cut named
there was taken: `redact_outbound` and its four-case test moved to
`crates/havoc-core/src/connection/trace.rs` (a `mod trace;` in `connection/mod.rs`, the
helper `pub(super)`), whose module doc says both why it is its own module — the rule is
about what a *log* may contain, not about driving a connection — and that the ratchet is
the other reason. **Trajectory, recorded as asked: 363 → 399 → 393.** `actor.rs` is 352
and no longer the longest file; the longest is now `crates/havoc-core/tests/config.rs` at
393, seven lines of headroom instead of one. Still worse than 363, so the ceiling stays at
400 rather than being tightened, and the next prompt to grow that test file splits it.

(4) **`$WORK/a.pass` — a file holding the fake password — was never removed**, and under
`KEEP_WORK` it survived, contradicting the header comment's promise that the credential
lives in the keychain and nowhere else. `rm -f` immediately after the seeding call on both
the success and the failure path. **Checked by running it, not by reading it:** a
`KEEP_WORK=1` run is 46/46, `a.pass` is absent from the kept tree, and
`grep -rlF fake-livetest-passw0rd` over that whole kept tree returns **zero files** — so
the strong form of the claim holds, not just the narrow one.

(5) **The store-unavailable error lost the platform error on exactly the platform that
needs it.** `keyring::Entry::new` collapses every store-initialization failure into a bare
`NoDefaultStore`, whose own message says only that there is no store — the real cause (the
D-Bus or Secret Service error on a headless Linux box, or "platform not supported") is
cached in `keyring::Entry::store_status()`. Verified against the 4.1.6 source rather than
guessed: `Entry::new` returns `Err(Error::NoDefaultStore)` when `SET_CREDENTIAL_STORE_RESULT`
is an error, and `store_status()` hands back that `&'static Result<()>`. `entry()` now
consults it on that one variant and describes the underlying error, so the sentence the
entry above promised a headless Linux user ("quote the platform error") is now actually the
sentence they get.

(6) **Two user-facing strings stated the PLAIN payload as `authcid\0authcid\0password`** —
the config.rs validation comment and, worse, the refusal message a user reads — while the
program sends `\0account\0password`. Same root cause as (1), which is the interesting part:
one wrong mental model of the payload wrote itself into a comment, an error string, a shell
assertion and a BUILD-LOG sentence. Both strings corrected, and the comment now names
caps.rs's pinning test so the next reader checks rather than remembers.

**Accepted as they stand, with reasons recorded rather than re-litigated:**

- **The `Ambiguous` arm is unreachable on macOS.** Only the secret-service store constructs
  it; the Apple store cannot return two matches for one service/account pair. Kept, per the
  order, because the message is the correct thing to say if it ever fires — and recorded
  honestly as **untested by construction on the dogfood platform**: no test exercises it
  and none can here, so its text is reviewed prose, not verified behaviour.
- **`credential set` trims only `\n`, so a CRLF pipe stores a trailing `\r`.** Literal
  compliance with the order ("trims exactly one trailing newline and nothing else"), and
  deliberately not widened: `\r` is a legal password character and guessing costs more than
  it saves. The symptom is recorded so the next person recognises it: the password is
  simply wrong at the server, with no local diagnostic, and the cause is invisible in a
  confirmation line that never prints the secret.
- **The watchdog orphans up to three `sleep 10` processes and has a narrow zombie race.**
  Harmless in both directions — the guard finding no such pid is a no-op, and a stale guard
  can only produce a spurious FAIL line, never mask a real failure. The one-line fix was
  obvious while in the file, so it was taken (`pkill -P "$guard"` before killing the
  subshell, with a comment saying a script that must never sleep should not leave sleeps
  behind); the structure was left alone as instructed.
- **The reviewer could not find the "ban-guard" in the diff, and was right not to.** It
  lives in the ad-hoc acceptance script under `.cache/p10b/` (gitignored, like 10a's
  by-hand script), not in the repository — so it protected *that run* and protects nothing
  in the product. Said plainly rather than left to imply otherwise: **the committed
  protection against a ban is the carry-forward note on PLAN stage 2 item 1, not a guard**,
  and until that note is acted on the shipped client still retries a `465` forever.

**Learned:** one, and it is the same lesson twice over. **An assertion about an absence is
worthless until you have seen it fail.** Findings (1) and (4) are both "the check passed and
the property did not hold" — a base64 string the program could never emit, and a promise
about `KEEP_WORK` that nothing had run. Both were settled the same way, by making the bad
state on purpose (break the redactor; run with `KEEP_WORK=1` and grep the kept tree), and
both took under two minutes. The entry above already recorded a weaker version of this from
10a's live run — "the new post-loop check did not fire, which is the correct outcome and not
evidence it works; it was verified by reading" — so this is the second prompt in a row where
a not-firing check went unexercised, and the first where doing the work found a real hole.
**The rule that follows: a new assertion whose job is to prove something is *absent* lands
with a recorded observation of it failing.** Not mechanized — no grep can tell a
never-fired assertion from a correct one — so it is written here for the next
detail-writing sub-agent, alongside 10a's disjoint-descope-list rule.

**Measured after the fixes:** **476** changed lines in `crates/` against the 800 cap (was
313 — the module split moves 60 lines and the diff counts them on both sides, plus fixes
(5) and (6)). `cargo test --workspace`: **74 tests, all green** — unchanged, because the
redactor's four-case test moved rather than multiplied. Ratchets: `todo-count 0`,
**`longest-file 393`** against a ceiling of 400 (was 399, and the check's own hint to
tighten is declined for the reason in fix (3)). New `trace.rs` is **60** lines,
`actor.rs` is **352** (from 399), `credentials.rs` **149**,
`crates/havoc-core/tests/config.rs` **393** — now the longest file in the tree.

**Live run:** `scripts/live-run.sh` re-run **three** times after the fixes, because both the
harness and the trace path changed. (1) With `redact_outbound` deliberately returning its
input: **exit 1, 43 ok, 3 FAIL** — the `<redacted>` assertion and the never-in-logs check on
both `a.trace` and `.cache/last-a.trace`, which is the evidence for finding (1) above. (2)
Redactor restored: **46/46, exit 0**, keychain clean afterwards (`find` exit 44), no
`a.pass` anywhere. (3) `KEEP_WORK=1`: **46/46, exit 0**, and the kept tree contains no file
holding the fake password and no `a.pass`, with the keychain item still removed — because
`keychain_forget` runs before the `KEEP_WORK` early return. The OFTC acceptance run was
**not** repeated: nothing in these six fixes touches connection behaviour (the redactor
moved modules; the rest is the harness, an error string and two comments), and a second
connection to a public network for a no-op change is exactly the citizenship the ban taught.
Said explicitly rather than left as a gap.

**Review:** all six findings fixed, four accepted with reasons, **none rejected**. The
review's highest finding was correct, its diagnosis exact (it named the empty authzid and
pointed at caps.rs), and its proposed remedy sufficient — and it caught a claim the entry
above made about its own verification that was not true, which is the finding class most
worth having.

**Carry-forward consumed:** none. All seven of 10b's notes were consumed by the first
commit and recorded in the entry above; this commit deletes no carry-forward block.

**Carry-forward raised:** **six, all adopted from the review's harvest, none rejected**, and
one placement overruled by the coordinator. To `PLAN.md`, since 10b is stage 1's last prompt
and no later prompt file exists to hold them: **stage 2 item 1** — a network that bans us is
retried forever while a network that rejects our password is not, `465` unhandled in
`connection/mod.rs`, observed live at Libera (**placed at stage 2, not the reviewer's
suggested stage 3, by the coordinator's call: stage 3 ships nothing new by rule, and a
dogfood month must not begin with a client that hammers networks that have said no**);
**stage 2 item 7** — `credential set` requires `sasl_account` to exist first, which settles
the first-run wizard's step order (config written and validated before the password is
taken, not after); **stage 4 item 3** — the secret lives in the *login* keychain, so on
macOS the engine must start as a launchd **Agent**, never a Daemon, or every
`sasl_account` network gets a store-unavailable error with no config-side fix; **stage 5
item 1** — the keyring read is a hard pre-dial failure of the whole process, so
multi-network must write the very map loop stage 1 forbade, with per-network degradation and
the failure surfacing as an event rather than a process exit; **stage 6 item 2** —
`AUTHENTICATE` has no chunking and `redact_outbound` compares against `Plain.as_str()`
alone, so a second mechanism is over-redacted and breaks live-run's assertions; **stage 6
item 3** — the Linux keyring tree (~95 lock packages) is invisible to the allowlist check,
which reads direct dependencies only.

**A measurement the entry above asked for and could not have:** its dependency decision
said a Linux CI runner "*would* compile the zbus half" and that this "should be measured
before it is trusted". CI is `ubuntu-latest`, and it built and tested this PR **green in
1m35s** — so the honest wording is now **compiled once, by a runner, unread by a human**,
which is what the stage 6 item 3 note says. It is evidence that the Linux tree is not
broken; it is not evidence that anyone has reviewed ~30 new crates, and the release item is
where that distinction has to be paid.

## Retrospective — Stage 1
**Date:** 2026-08-10

Stage 1 closed at 12/12: twelve prompts, 12 prompt entries plus 4 review addenda, 38
decision entries, 2 corrections, and 5 questions still open. The engine connects to a real
network over TLS with SASL, logs what it sees into SQLite, and answers search, backlog and
read-marker requests through a typed boundary a debug CLI drives. 74 tests. No UI.

**CLAUDE.md:** re-read end to end. **Two corrections, no prunes, and the honest reason
for that.** Corrected: (1) the Secrets paragraph promised "the OS keyring (encrypted-file
fallback)" as shipped fact when prompt 10b deferred the file half — now "(encrypted-file
fallback deferred — PLAN stage 4 item 3)", the same staleness the drill found in
NORTH-STAR §5.8 and fixed by the same reasoning; (2) the live-run rule named
"local `ergo` or Libera.Chat" as the two targets, and Libera has network-banned this
machine — now "local `ergo` or a public network", which is also the more durable sentence.
**Nothing was pruned**, and that is a finding rather than a pass: at **100/100 lines the
cap is fully consumed**, so the next rule that earns a place has to evict one, and no
section is currently dead enough to volunteer. One genuine deletion *candidate*, proposed
here rather than taken silently because deleting a rule wants a decision entry: "Vendored
fixtures must record their upstream commit SHA" has no subject in the tree — the only
external artifact is `ergo`, which is downloaded and sha256-verified by
`scripts/live-run.sh`, not vendored. It has therefore never fired and cannot fire until
something is actually vendored. Left in place for now (a rule that is merely dormant is
cheaper than re-deriving it later), but it is the line to spend when the cap next binds.
Two pointers deliberately **not** changed: line 4 still names `STAGE-1-PROMPTS.md` as the
queue, which is true today and becomes false the moment `STAGE-2-PROMPTS.md` exists — so
it is on the stage-2 opening checklist below, per the rule that a doc is fixed by the
commit that stales it; and the `discipline-stats.txt` sentence still describes the
telemetry as something the rule review argues from, which the Rule review section below
shows is not yet true — the caveat lives at the mechanism (a comment in
`scripts/check-docs.sh`) rather than costing a line here.

**Docs audit:** every artifact opened and checked, not just the ones the drill named.

- **NORTH-STAR.md** — two real staleness bugs, both fixed by a **dated appendix**
  (2026-08-10), never by editing the section, which is this document's own rule. §9 listed
  *buffer identity* and *config vs. runtime state* as open when both were settled in stage
  1, and *the name* as open when the 2026-08-09 amendment settled it; the appendix marks
  the three settled, points the two genuinely-open ones at PLAN, and states that **PLAN's
  Still open list is the sole live register** — §9 is the snapshot this document opened
  with. It also records a gap neither document had: §9 **omits** the licence question, and
  PLAN does not carry it either, so it is owed before stage 6's release item. Separately,
  §5.8's "keyring with encrypted-file fallback" now says the keyring half shipped and the
  file half is deferred, with the item and deadline.
- **README.md** — four fixes beyond the badge and row. "**Nothing runs yet**" was false: it
  now says stage 1 is complete, names what the headless engine does, and says plainly that
  there is no UI yet and nothing is packaged. The stage 1 heading said "(in progress)". And
  the **Building** section gained the toolchain preconditions a fresh session on this
  machine trips over first: `cargo` must be on `PATH` and Homebrew's rustup leaves it off,
  with the exact `export`; plus `sqlite3`/`openssl`/`nc` for the live run, the pinned
  `ergo` download, and the one login-keychain item that run creates and removes.
- **PLAN.md** — current, because it was edited inside each prompt rather than swept at the
  end; this stage added 43 carry-forward notes to later stages and 5 Still-open questions
  that way. Stage 1 item 10's fallback wording and the stage 1 Done-when's honest
  "what went unproven" paragraph both landed in prompt 10b's own commits.
- **STAGE-1-PROMPTS.md** — status line at 12/12 with the new closing form, 10b's outcome
  paragraph appended under its block, and **no `### Carry-forward` block left on any
  prompt** (verified: the only two in the file are the preamble's convention heading and
  its illustrative example, which sit above prompt 1 and which check 7's position counter
  ignores by construction).
- **scripts/check-docs.sh** — two comment-only edits, no behaviour change and therefore no
  `test-checks.sh` fixture owed: the commented `2:STAGE-2-PROMPTS.md:18` line now says the
  18 is an inherited template **placeholder** (PLAN's stage 2 has 7 items) to be set when
  stage 2 is planned, and the ledger's definition now documents that it is per-checkout.
- **METHOD.md** — read, nothing stale found; it describes the method's reasoning, which
  this stage did not change.

**Failure register:** two `## Correction` entries in the whole stage, and the register is
exactly those entries and their `**Category:**` tags — stated because the drill (finding 6)
could only infer it. Counts against `CATEGORY_LIMIT=3`: **`stale-doc-copy` 1**, the TLS
decision entry that misstated its own alternative; **`convention-not-checked` 1**, merged
branches never deleted, eleven times — a single correction covering eleven occurrences,
which is the shape that matters here, because it closed its loop with a mechanical guard
rather than a reminder. Neither category is trending: no second occurrence of either, and
nothing at two. What the register does **not** capture, and should be read alongside: this
stage's most expensive mistakes were caught by the post-prompt review and recorded as
review findings, not as corrections — 10a's unvalidated `autojoin` line integrity and 10b's
inert base64 assertion were both real shipped-or-nearly-shipped bugs. A correction entry is
for a *past log entry* being wrong, so the register is deliberately narrower than "things
we got wrong", and reading it as the latter would flatter the stage.

**Rule review:** **the ledger does not exist, and that is the finding.**
`discipline-stats.txt` is absent from the primary checkout, absent from this worktree, and
absent from the machine — `find` over the whole repository returns nothing. The mechanism is
sound in isolation (`err()` appends `date rule branch`, local runs only, deduplicated per
day/rule/branch) but the **path is relative and prompts run in worktrees that are deleted
when their PR merges**, so every prompt's telemetry died with its worktree, and the primary
checkout has none because no prompt is ever run there. The consequence is precise and worth
stating rather than glossing: **this review cannot distinguish "no rule ever fired in twelve
prompts" from "every record was thrown away", and it therefore cannot argue from counts at
all** — which is exactly the failure mode the ledger was introduced to prevent, arriving in
a form nobody checked for. The instruction for this retrospective was to read the primary
checkout's copy; there is no copy to read, so the instruction is answered by reporting its
absence instead of inferring a zero. What can be said honestly without the ledger: no
`make check` failure was observed locally in prompt 10b's own worktree across roughly a
dozen runs, and the checks that visibly did work this stage did so as *blockers* rather than
as telemetry — the size cap forced 10a's oversize justification, the ratchet forced two file
splits, check 10 fused this retrospective into 10b's PR, and check 7 is what makes a
consumed carry-forward a mechanical fact. **No rule is proposed for deletion on evidence,
because there is no evidence**; the one dormant rule found by reading is the vendored-fixture
SHA rule above. **The mechanization this review does earn** is the fix to the ledger itself:
anchor the path at the parent of `git rev-parse --git-common-dir` so every worktree appends
to one file. It is a behaviour change, so it lands with fixtures in
`scripts/test-checks.sh`, and it is **owed by the commit that opens stage 2** — recorded
here and in a comment at the mechanism, deliberately not filed under a PLAN feature item,
because PLAN has no numbered item for tooling and burying tooling under "Embedded-mode
wiring" is how a note stops being read. Deferred out of this PR on the coordinator's
prefer-filing guidance, and because a check that lands without its fixtures is the thing
CLAUDE.md forbids most specifically.

**Cold-start drill:** a fresh sub-agent, given only the repository at commit `cdc431c`, was
asked what comes next and why. **It got the answer right**: it identified the project, that
stage 1 was the active stage, that prompt 10b's implementation was complete, and that the
next unit of work was this closing commit — the retrospective plus the 12/12 bump. It also
hit the anticipated status-lag state (`**Status:** 11/12` over finished 10b code) and
**recognised it from the queue text rather than being misled by it**, which is the fused-PR
decision working as designed — though only because it read 1,889 lines in to find the
explanation. Thirteen gaps, with what each produced:

1. **The status line said 11/12 over finished 10b code, with no pointer near the line.**
   Transient — this commit ends it — but the pattern recurs at every stage boundary.
   Recorded; the fused-PR decision in the 10b block already documents the window.
2. **Nothing on disk says the implementation is finished and only the closing commit
   remains; PR state is unknowable from documents.** Inherent: git and GitHub state are not
   repository content. Recorded, not fixed.
3. **No document said what follows "Next:" in a 12/12 status line.** **Fixed** by deciding
   it: `**Status:** 12/12 complete. Stage 1 closed — retrospective in BUILD-LOG.md; stage 2
   planning is next.` Verified against check 5's regex before committing — it captures the
   count and accepts any text after "complete" — and against check 8, whose "Next: prompt M"
   parse now yields empty and skips cleanly instead of failing.
4. **Who opens stage 2, and the commented placeholder naming 18 prompts against PLAN's 7
   items.** **Comment-only fix** in `check-docs.sh` saying the 18 is an inherited
   placeholder to be set when stage 2 is planned; the opening sequence is recorded in Next
   stage below.
5. **`discipline-stats.txt` absent, and nothing said which checkout's ledger governs.**
   **Fixed and escalated** — the drill found the symptom, and looking for the primary
   checkout's copy found there is none anywhere. Documented at the mechanism, and the real
   repair is owed by stage 2's opening commit. See Rule review, which this finding rewrote.
6. **How the failure register's counts are produced was inferred, not stated.** **Fixed by
   stating it**: the register is the `## Correction` entries and their `**Category:**`
   tags. See Failure register.
7. **NORTH-STAR §9 listed settled questions as open and omitted the licence, while PLAN
   cites §9 as live.** **Fixed** by dated appendix.
8. **§5.8 promised keyring *plus* fallback with no note the fallback is deferred.**
   **Fixed** in the same appendix.
9. **No rule said whether a stage may close with a Done-when clause unproven.** Recorded as
   a principle, because it is the honesty rule's corollary rather than a new rule: **it may,
   when the gap is named where the claim would be** — in the Done-when itself, in the
   prompt's Live run section, and on Still open with an owner. The instance is SASL at a
   public network. The alternative — holding a stage open until a volunteer network's ban
   appeal is answered by a human who is not in the loop — would trade a named gap for an
   unnamed stall, which is worse in exactly the way this log exists to prevent.
10. **Whether a docs-only closing commit owes Shipped/Learned/Live-run.** Recorded: check 3
    requires those sections only for `crates/` changes, and the retrospective template
    governs this commit. The drill's inference was correct; saying so is the fix.
11. **Drill circularity — its own fixes change the branch it ran against.** Recorded as
    convention: **the drill runs once, against the finished branch (`cdc431c`), and its
    fixes are its output.** It is not re-run to make it come out clean, because its purpose
    is measurement, not a green light — re-running until there are no gaps would convert
    the one honest instrument at the boundary into a formality.
12. **The stage-acceptance evidence lives in gitignored `.cache/`; a fresh clone has only
    prose.** Recorded as a deliberate trade, not an accident: the **committed recipe is the
    Acceptance paragraph of the 10b block**, which is reproducible on any machine, while
    logs are per-machine and would rot into fiction if committed. The cost is real — a
    reader cannot audit the numbers, only re-run them.
13. **Toolchain preconditions unstated; the Makefile calls bare `cargo`.** **Fixed** in
    README's Building section, including the Homebrew-rustup `PATH` trap that every fresh
    session on this machine hits first, the live run's other binaries, and the keychain item
    it creates.

Nine of the thirteen produced a documentation fix in this commit; four are recorded as
inherent or as conventions. The drill's value was concentrated in the two it found that
nobody had suspected — the missing ledger (5) and NORTH-STAR §9 (7) — both of which are
documents *quietly disagreeing with reality* rather than documents that are merely thin,
which is the class a fresh reader is uniquely able to see.

**Ratchets:** `todo-count` **0** against a ceiling of 0 — at ceiling all stage, and no
`TODO`/`FIXME`/`HACK` was ever committed. `longest-file` **393** against a ceiling of 400.
**Neither ceiling was tightened, and the reason is that neither improved:** `longest-file`
went 363 → 399 → 393 within prompt 10b alone (the redactor pushed `actor.rs` to one line of
headroom; the review's fix moved it to `connection/trace.rs`, bringing `actor.rs` to 352),
so 393 is a recovery from a self-inflicted worsening, not a gain, and tightening to it would
turn the ratchet on the strength of a repair. `crates/havoc-core/tests/config.rs` is now the
file at the wall at 393 — **seven lines of headroom, and the next prompt to grow it splits
it**, because raising a ceiling is a decision entry and not a convenience. The ratchet did
real work this stage: it forced two file splits that would otherwise have been argued about.

**Next stage:** nothing was reordered, rescoped or deleted in PLAN.md at this boundary —
stage 2's seven items still read correctly against what stage 1 actually shipped — but the
stage does not open with a blank slate, and two proposals are named here rather than made
silently, per the rule that structural changes are proposed and not enacted.

**Opening stage 2 is its own first unit of work**, and it is one commit doing four things
together, because any three of them without the fourth leaves `make check` describing a
repository that does not exist: write `STAGE-2-PROMPTS.md` (with prompts 1–4 detailed and
the rest carrying scope and fence only, per the JIT rule that stage 1 vindicated);
uncomment the `STAGES` line and **set the real prompt total** in place of the inherited 18;
move `README_STAGE` to 2 and add the stage 2 badge and progress table; and update
CLAUDE.md's line 4 pointer from `STAGE-1-PROMPTS.md` to the new queue. The ledger fix from
Rule review, with its `test-checks.sh` fixtures, belongs in that same commit — it is the
one piece of tooling debt this stage leaves, and the next retrospective is unable to do its
job without it.

**Stage 1 left 43 carry-forward notes on later stages, and 19 of them are on stage 2** — 7
on item 1 (embedded wiring), 6 on item 7 (first-run), 5 on item 5 (input and command line),
1 on item 3 (scrollback). That distribution is the argument for both proposals.

- **Proposal: handle `465` before the TUI work, not inside it.** Item 1's newest note is
  that a network which bans us is retried forever, observed live at Libera during 10b's
  acceptance run. It is engine-side, small, and independent of every UI decision around it,
  and stage 3's dogfood month must not begin with a client that hammers networks that have
  said no. Proposed as stage 2's first *code* prompt — ahead of embedded wiring — or as a
  numbered item of its own.
- **Proposal: pull item 7's *decisions* forward without moving its polish.** First-run is
  listed last, but six stage-1 notes hand it questions that earlier items depend on: whether
  config stays mandatory and how a wizard may write it, the no-echo prompt, whether the
  loopback-only plaintext rule and the `sasl_account` + `plaintext` refusal relax, whether
  validation moves off `parse`, and the step order `credential set` already forces (config
  written and validated *before* the password is taken). Item 5's composer work and item 1's
  wiring both touch the first two. Proposed: answer those as decision entries early in the
  stage, and leave the wizard itself where it is.

One thing stage 1 should be honest about handing over: **the two clauses of its own
Done-when that closed with named gaps** — SASL against a public network (no registered
account; Libera has network-banned this IP; OFTC advertises no `sasl` at all) and, found by
the same run, that on a network without `echo-message` nothing the user says is stored at
all. The first is on Still open with the user as owner. The second is on stage 2 item 5 with
stage 3 as its deadline, and it is the more dangerous of the two, because it is not a gap in
what was verified — it is a gap in what was *built*, discovered only because the acceptance
run went somewhere the plan had not.
