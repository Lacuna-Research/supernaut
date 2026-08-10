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
