# The Method — a project-scaffolding template for LLM-assisted software work

This is a language-agnostic extraction of a working system proven on a real
project. Copy the files in this directory into a new repo, run the
adaptation checklist at the bottom, and delete this file (or keep it; it is the only
file here that explains *why*).

The system exists to solve one problem: **an agent with no memory, working across
dozens of sessions, cannot be relied on to keep a promise it made in a previous
session.** Everything below is either a way to write things down so a cold session
finds them, or a way to make a machine check that they were written.

---

## The four artefacts

| File | Kind | Lifetime | Rule |
|---|---|---|---|
| `CLAUDE.md` | Operative rules only | Permanent, hard-capped | Prune as readily as you add |
| `PLAN.md` | Living roadmap + the one list of open questions | Permanent, freely rewritten | Reorder, rescope, delete |
| `STAGE-N-PROMPTS.md` | The work queue for the current stage | One stage | Detail written just-in-time |
| `BUILD-LOG.md` | Reasoning, deviations, surprises, measurements | Permanent, append-only | Never edit a past entry |
| `SUBAGENT-BRIEFS.md` | Static dispatch briefs for the four sub-agent roles | Permanent, static reference | No state lives here |
| `ratchets.txt` | Ceilings for slow-rot metrics | Permanent | Only turns one way |

The split matters more than the contents. `CLAUDE.md` holds *rules*; `BUILD-LOG.md`
holds *reasoning*. That separation is what keeps the instruction file short enough to
be read carefully, and it is the first thing that erodes if you let it.

**Deliberately absent: a status file, a TODO file, and a summary of the plan inside the
README.** Two copies of one list drift, and the copy nobody edits is the one that gets
read. Every derived number — a README badge, a progress table, a website counter — is
either checked mechanically against its source or does not exist.

---

## The seven self-healing mechanisms

These are the parts worth understanding before adapting anything. Each one closes a
failure mode that had already happened at least once.

### 1. The line cap on the instruction file

`CLAUDE.md` is capped (100 lines) by `scripts/check-docs.sh`, and the cap is not to be
raised. Instruction files rot by accretion: rules get appended, never removed, until
the file is long enough that nothing in it is read carefully. A hard cap converts
"should we prune this?" from a judgement call into a build failure. In practice the cap
fires exactly when you add something genuinely new, which is exactly when the old rules
are worth re-reading.

Corollary rule, in the file itself: *when a convention proves important, make it
mechanical rather than writing it more emphatically.* Emphasis does not survive a
context reset. A failing check does.

### 2. Append-only log with corrections as new entries

`BUILD-LOG.md` fails the build if a diff removes or modifies an existing line. A wrong
entry is corrected by a later `## Correction —` entry that says what it supersedes. This
is what makes the log trustworthy enough to be worth reading: nothing in it has been
quietly tidied, so a claim in it is evidence of what was believed at the time.

It also produces a genuinely useful artefact — a record of the project's *recurring
failure modes*, visible only because the wrong answers were never deleted.

### 3. Carry-forward notes, with an expiry check

The core mechanism, and the one most worth stealing.

When work on prompt 4 discovers something prompt 11 needs to know, the note is written
**onto prompt 11, in the queue file** — not into a notes file, not into the log, not
into the agent's head. The queue file is already re-read at the start of every prompt,
so there is no second place anyone has to remember to check.

```
**Carry-forward** *(consumed when this prompt runs)*

- From prompt 4: the transport reports `.ready` before the TLS handshake completes,
  so do not start the registration timer here — wait for the first byte.
  `Transport.stateDidChange` is the seam.
```

Three rules make it work:

- **A note names a file and a symbol.** A note that says "think about X" is worth
  little; one that names the seam is worth a session.
- **A note is deleted when the prompt that received it runs**, and the fact that it was
  applied is recorded in `BUILD-LOG.md`.
- **`check-docs.sh` fails if a note is still attached to a prompt the status line says
  is complete.** Without this the convention decays silently: notes accumulate,
  nobody consumes them, and the file grows a layer of stale advice that later sessions
  cannot distinguish from live advice.

Notes aimed past the current stage go on the `PLAN.md` item instead, under the same
heading, with the same consume-and-delete rule.

### 4. Just-in-time prompt detail

Early prompts are written out in full. Later prompts carry **their scope, their
grouping and their scope fence, but not their detail** — and are fleshed out
immediately before they start.

A detailed brief for work that six intervening prompts will have reshaped is
confidently wrong, and a wrong brief is worse than a thin one because it gets followed.
The `PLAN.md` item is the durable statement of intent; the prompt is the short-lived
working document that turns one into a session.

This is also what makes carry-forward notes usable: they arrive *before* the detail is
written, so they shape it rather than contradicting it.

### 5. One list of open questions, in the roadmap, never in the log

Open questions live in a single **Still open** list in `PLAN.md`, each marked blocking
or not blocking, each deleted when answered — and the answer recorded as a decision
entry in the log. A question buried in an append-only log is a question nobody finds.

This rule was written after an audit found open questions scattered across four
`### Open` sections in a 950-line log.

### 6. Machine-checked agreement between every copy of a number

The queue file carries a machine-readable status line:

```
**Status:** 14/18 complete. Next: prompt 14.
```

`check-docs.sh` parses it, then verifies every derived copy agrees — the README badge,
the README progress table, any website counter or meter. A stale badge is worse than no
badge, because it is confidently wrong.

Note the subtlety worth copying: the status line is a **count**, and prompt *labels* may
be split (13a, 13b) without renumbering everything after them. So the staleness check
counts headings by **position**, not by parsing the number out of the label.

### 7. Hooks that fire at the end of a turn

A pre-commit hook runs the doc checks and refuses commits to `main`. A pre-push hook
refuses to push a branch still carrying its auto-generated worktree name. A `Stop` hook
in `.claude/settings.json` detects a worktree that has outlived its merged PR and blocks
the turn with an explanation.

The design constraint on a Stop hook is worth restating: **it must be silent in every
case it is not sure about.** A hook that cries wolf gets ignored, and this one can block
a turn. The worktree check uses two conclusive signals (the upstream ref is gone; or
HEAD's *tree* is identical to the base's despite differing commits, meaning a squash
landed) and exits silently otherwise.

---

## The prompt lifecycle

Written into `CLAUDE.md` as a numbered list, because a cold session will do exactly what
is listed and nothing that is merely implied.

**Starting a prompt:** re-read it in the queue, including any carry-forward block. If it
is one of the just-in-time ones, write its detail now.

**Finishing a prompt, before reporting done:**

1. Append a `BUILD-LOG.md` entry — deviations, deferrals, surprises, measurements. Not a
   restatement of the diff; git already has the diff.
2. Raise carry-forward notes on later prompts, naming file and symbol.
3. Consume the notes addressed to this prompt: act, delete, record that you did.
4. Push anything deferred into `PLAN.md` at the stage where it belongs.
5. **Run it live**, not just the test suite. (The source project shipped three defects
   that passed every unit test and failed within a minute of a live run. All three were
   about what the window showed, not what the state was.)
6. Bump the status line, run the checks, merge, and clean up the worktree.

**Between prompts**, record decisions *at the moment they are made*:

- A choice with a rejected alternative → a decision entry, with the reasoning and what
  would justify revisiting it.
- A change to scope → edit `PLAN.md` in the same turn. Never answer "good idea, we'll do
  that" without writing it down.
- A question left open → the **Still open** list.

---

## The handoff audit

Before any planned context reset, and at every stage boundary, audit **what is true only
in this session rather than on disk**. The source project's audit of this kind found
three gaps in twenty minutes, all of which would have cost a cold session hours.

The output is a decision entry containing:

- **State at handoff** — branch, status, open PRs, worktrees, what is implemented vs
  stubbed, what is next and whether it has notes waiting.
- **Things learned that are worth not relearning** — toolchain traps, API surprises,
  and, most valuably, *your own recurring failure mode*. The source project's was
  "asserting how a mechanism behaves instead of testing it," recorded after it happened
  four times.

---

## Extensions beyond the source project

Everything above was extracted from the source project. The mechanisms below were added
to this template afterward, each closing a loop the original left open. Same design rule
throughout: a promise became a check, and every check ships with fixtures.

### Closed loops in the checks

- **Log entries must have substance** (check 3). "The file changed" used to be the
  whole rule, and 'fixed stuff' satisfied it. Now the appended entry must carry
  `**Shipped:**`, `**Learned:**` and `**Live run:**` by name — the sections that pay
  for the file existing, and the two most often skipped.
- **Consume-and-record is one act** (check 4). Deleting a carry-forward note used to
  be the pass condition on its own, so a note could vanish silently —
  indistinguishable, later, from a note acted on. Removing a carry-forward block now
  requires `**Carry-forward consumed:**` in the same change's log entry.
- **Blocked prompts cannot start** (check 8). The Still open list's annotation is now
  machine-readable — `*(blocking: prompt 5)*` — and the check fails when the status
  line's "Next" is a blocked prompt. A question run into mid-prompt gets answered by
  whatever is expedient, then shipped as public API the real answer has to migrate;
  the source project's naming question accreted two families of config keys exactly
  this way.
- **Oversize is allowed; silent oversize is not** (check 11). Past the changed-line
  cap, the log entry needs an `**Oversize:**` justification. Forces the split
  conversation at the only moment it is cheap.
- **The checks are tested** (`scripts/test-checks.sh`). A broken check fails open —
  silently — so every rule has a fixture that triggers it and one that clears it, and
  a new check lands with its fixtures in the same PR. This suite is also the
  adaptation safety net: it is what tells you your project-specific edits to
  `check-docs.sh` did not quietly disable a rule.

### The loops

- **The mechanization loop, made observable** (check 12). "Make it mechanical rather
  than more emphatic" was the governing rule; now every `## Correction` entry must end
  with `**Mechanized as:**` or `**Not mechanizable because:**`. Either answer is fine.
  No answer is what the check forbids.
- **The failure-mode register** (check 13). Corrections carry a `**Category:**` tag
  with short, stable, reused names. At three occurrences the check demands a
  `**Rule raised:**` line naming what the habit became. The source project's most valuable
  self-observation — "asserting how a mechanism behaves instead of testing it" — was
  buried in one handoff entry; the register computes it instead.
- **The forced stage boundary** (check 10). When a status line reads N/N, nothing else
  lands until `## Retrospective — Stage N` exists in the log. The retrospective template
  bundles the four audits momentum skips: the CLAUDE.md prune, the docs audit, the
  register review, and the cold-start drill.
- **One-way ratchets** (check 14). `ratchets.txt` records ceilings;
  `measure-ratchets.sh` emits current values. Worse fails; better invites tightening
  the ceiling in the same PR. This is the check for rot no single PR causes and no
  per-PR review sees.


### The efficiency pass

A later review of this template applied its own standard to itself — process additions
are dependencies, each suspected of not being worth its cost — and made four cuts plus
one addition. Recorded here because the reasoning is the reusable part:

- **Two per-prompt dispatches became one.** The reviewer and harvester shared their
  main input; separate dispatches bought isolation that was already there. Per-prompt
  sub-agent cost halved.
- **Proportionality became explicit.** The review is skippable for trivial changes and
  the live run can honestly read N/A when there is no behavior to observe — with the
  skip written down. The requirement is honesty, not ritual; a process that demands
  ceremony for a typo fix trains you to fake the ceremony everywhere.
- **The README progress apparatus became opt-in.** Badge and table checks engage only
  when a stage badge exists in the README. A public front door earns the per-prompt
  upkeep; a private working repo does not. Once the claims exist they are checked, and
  the skip is printed, never silent.
- **The failure ledger was added** — the one addition, and it is passive. Every local
  check failure appends `date rule branch` to `discipline-stats.txt` (deduplicated per
  day; never written in CI). Nothing reads it mechanically. It exists so the
  retrospective's rule review — which rules fired never, which constantly — argues
  from counts instead of memory, and the zero-fire rules become deletion candidates.
  This is the pruning loop applied to the process itself: everything else had a
  ratchet except the checks.

What was deliberately *not* built: a toggle manifest for turning rules on and off,
with dormancy states and expiry. It solves a problem never yet observed, and the
system already contains the honest path — when a rule genuinely needs disabling, a
decision entry plus commenting out the check plus updating its fixture is one visible
diff. If that happens twice and feels clumsy, the correction entry's `**Mechanized
as:**` line is where the manifest gets built. Adding it preemptively is the accretion
pattern this whole document warns about.

### The sub-agent roles

Four roles, briefs in `SUBAGENT-BRIEFS.md`, wired into the lifecycle in `CLAUDE.md`.
The organizing insight: a sub-agent's value here is **context isolation, not
parallelism** — each role works precisely because the agent lacks the implementing
session's context, so the fastest way to ruin one is to paste the conversation in
alongside the brief.

- **Post-prompt review** (finishing step 2): prompt block + diff + remaining queue,
  nothing else — one dispatch doing both the adversarial check ("does the diff do what
  the order says; what landed that the fence excludes") and the carry-forward harvest.
  These were two dispatches in an earlier draft; they merged because they share their
  main input and neither contaminates the other — the second dispatch bought isolation
  that was already there. Skippable, honestly and in writing, for trivial changes.
- **JIT detail writer** (starting an unwritten prompt): plan item + notes + named
  source, *not* the previous session, so the new brief cannot inherit the last one's
  shape.
- **Cold-start drill** (retrospectives and handoffs): the repository and nothing else.
  A fresh agent states what is next and why; every gap between its answer and reality
  is a documentation bug. This is the only direct test of the claim the whole system
  makes — that the repo alone is sufficient.
- **Plan bootstrap** (once, at project creation): a `north-star.md` outline plus the
  template's format references, nothing else. Turns the vision document into
  `PLAN.md` and the stage-1 queue before there is a session to contaminate them;
  from then on the plan is authoritative and the north-star holds only the why.

Two rules keep the roles from corroding the system they serve. **Sub-agents advise;
artifacts decide** — output lands as a proposal in an existing artifact, never a new
channel, never an authority. And every proposal gets a recorded disposition — adopted,
edited, or rejected-because — since a rejected proposal is information too.

---

## Adaptation checklist

1. Copy everything: `CLAUDE.md`, `PLAN.md`, `BUILD-LOG.md`, `STAGE-1-PROMPTS.md`,
   `SUBAGENT-BRIEFS.md`, `ratchets.txt`, `scripts/`, `.githooks/`, `.claude/`,
   `.github/workflows/`, `Makefile`.
2. Replace every `<PLACEHOLDER>` — search for `<` to find them all.
3. In `scripts/check-docs.sh`: set the `STAGES` array and `SOURCE_DIR`, review
   `MAX_PROMPT_CHANGED_LINES` and `CATEGORY_LIMIT`, and fill in the dependency-policy
   and project-specific sections at the bottom. Those sections are the point of the
   script; the generic half is only scaffolding for them.
4. In `scripts/measure-ratchets.sh` and `ratchets.txt`: replace the two default
   metrics with what this project's rot actually looks like, and set the starting
   ceilings to the current measured values — a ratchet that starts loose is a ratchet
   that never engages.
5. Run `./scripts/test-checks.sh` after every edit to `check-docs.sh` — steps 3 and 4
   included. The suite is the only thing that catches an edit that quietly disabled a
   rule, because a broken check fails open. Update the fixture builder if you changed
   the machine-readable formats.
6. In `Makefile`: replace the `build` / `test` / `lint` / `fmt` bodies with your
   toolchain's commands. Keep `check`, `check-tests` and `hooks` as they are.
7. `make bootstrap` once after creating the repo. It installs the hooks and turns on
   server-side branch protection with `discipline`, `discipline-tests` and `secrets`
   as required status checks — the local hooks are a fast first line, not the
   enforcement — then prints the adaptation work that remains. Needs an authenticated
   `gh`; without one it says exactly what to set manually.
8. Write the stage 1 prompts. Full detail for the first three or four; scope, grouping
   and fence only for the rest. If the project starts from a `north-star.md` outline,
   dispatch the Plan bootstrap brief (`SUBAGENT-BRIEFS.md`) to draft both `PLAN.md`
   and the queue from it.
9. **Add the first project-specific mechanical check within the first week** — with
   its fixtures. The generic checks catch documentation drift. What actually keeps a
   project honest is a check that encodes *this* project's one non-negotiable
   architectural rule — the way the source project builds its protocol module on
   Linux in CI so that any accidental dependency on a platform API fails mechanically
   rather than being noticed in review.

## What to expect to get wrong

- **Writing the whole queue in detail up front.** It feels like progress and produces
  briefs that are wrong by the time they are read.
- **Letting the instruction file grow "just this once."** Raise the cap once and it is
  no longer a cap.
- **Recording a decision "after this prompt."** It does not happen, and the alternative
  you rejected is the part that gets lost first.
- **Treating the test suite as the acceptance gate.** It is a necessary gate, not a
  sufficient one.
- **Adding a rule when the fix is a check.** Ask, every time: what would make this fail
  loudly instead?
