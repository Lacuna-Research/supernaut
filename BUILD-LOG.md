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
