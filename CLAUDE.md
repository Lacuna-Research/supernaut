# <PROJECT> — Project Instructions

<One-line description of what this is.>
`PLAN.md` roadmap · `STAGE-1-PROMPTS.md` work queue · `BUILD-LOG.md` history.

## Build standards

- <Language/toolchain and version. Strictness flags. Warnings-as-errors. Minimum
  platform/runtime target. Test framework.>
- **<Dependency policy — e.g. zero external dependencies.>** Changing it requires a
  decision entry in `BUILD-LOG.md` and an edit to `scripts/check-docs.sh`, which fails
  the build otherwise. Vendored fixtures must record their upstream commit SHA.
- <Any architectural rule that can be checked by a machine, and how it is checked.>

## Working method

One prompt from the current stage's queue per branch (`prompt-NN-slug`), per PR. Never
commit to `main` directly. Every `PLAN.md` item is attached to a prompt. **You merge
your own PRs** — squash-merge and delete the branch once CI is green. Stop and ask only
if CI is red, the work diverged from its prompt, or a decision is genuinely the user's.

**Starting a prompt:** re-read it in the queue, including any carry-forward block. If
its detail is unwritten, write it now — in a **fresh sub-agent** given the `PLAN.md`
item, its notes, and the relevant source, never the previous prompt's session — and
never long in advance.

**Finishing a prompt, before reporting done:**

1. Append a `BUILD-LOG.md` entry — deviations, deferrals, surprises, measurements.
   `make check` requires the Shipped, Learned, and Live run sections by name.
2. Run the **post-prompt review** — one sub-agent dispatch (brief in
   `SUBAGENT-BRIEFS.md`) doing both jobs: the adversarial check and the carry-forward
   harvest. Record dispositions in the entry's Review and Carry-forward raised
   sections. For a trivial change — docs-only, or far under the size cap with no fence
   risk — skip it and write `**Review:** skipped, trivial (<reason>)`; the honesty is
   the requirement, not the ritual. Consumed notes get acted on, deleted, and recorded
   in `**Carry-forward consumed:**` — the check requires the pair.
3. Push anything deferred into `PLAN.md` at the stage where it belongs.
4. Run it live, not only under test. <State the live-run recipe: real target, isolated
   config, whatever second party it needs.> A change with no observable behavior may
   record `**Live run:** N/A (<reason>)` — claiming a run that did not happen is the
   one dishonesty this file cares most about.
5. Bump the `**Status:**` line, then `make check`, merge, and leave the worktree — a
   prompt ends at the repo root, not in its worktree.

**Between prompts.** Record decisions *at the moment they are made*, never deferred:

- A choice with a rejected alternative → a decision entry in `BUILD-LOG.md`, with the
  reasoning and what would justify revisiting it.
- A change to scope or approach → edit `PLAN.md` in the same turn. Never answer "good
  idea, we'll do that" without writing it down.
- A question left open → the **Still open** list in `PLAN.md`, marked with the
  machine-readable blocking annotation. `make check` refuses to start a blocked prompt.

**Sub-agents advise; artifacts decide.** Sub-agent output lands as a proposal in an
existing artifact — never a new channel, never an authority — and accepting or
rejecting it is this session's call, recorded where the proposal landed.

## Enforced mechanically

`make check` (pre-commit + CI) enforces: the cap on this file, `BUILD-LOG.md`
append-only, a substantive log entry per `<SOURCE_DIR>/` change, status/README/plan
agreement, carry-forward notes neither outliving their prompt nor deleted unrecorded,
blocked prompts not starting, oversize changes justified, correction entries closing
the mechanization loop, recurring failure categories raising a rule, ratchets never
worsening, the stage retrospective existing, and the dependency policy. Git hooks and a
Stop hook guard the rest: no commits to `main`, no unrenamed worktree branch pushed, no
worktree outliving its merged PR. Local failures append to `discipline-stats.txt` —
passive telemetry the retrospective's rule review reads, so "this rule earns its keep"
is a counted claim rather than a remembered one. When a convention here proves
important, make it mechanical rather than writing it more emphatically — and **a new
or changed check lands with fixtures in `scripts/test-checks.sh` in the same PR**; a
broken check fails open, and open is silent.

## Stage boundaries

When a stage's status reads N/N, `make check` fails until `BUILD-LOG.md` carries a
`## Retrospective — Stage N` entry (template in the log): this file re-read and pruned,
docs audited, the rule review argued from `discipline-stats.txt`, the failure-category
register reviewed, and the **cold-start drill** — a fresh sub-agent, given only the
repo, states what is next and why; every gap between its answer and reality is a
documentation bug to fix before the next stage starts. Before any planned context
reset, run the same audit as a handoff entry.

## Maintaining these documents

Keep docs current without being asked; fix a stale doc in the same commit as the code
that staled it. Reasoning belongs in `BUILD-LOG.md`, not here — this file holds
operative rules, and that split keeps it under the 100-line cap, which is not to be
raised. Read the log's last entries or search it, never front to back; open questions
live only in `PLAN.md`'s **Still open** list. `PLAN.md` is a living roadmap: reorder,
rescope, delete freely; reference items by name, never number. Propose structural
changes here rather than making them silently; prunes need none.

## Where things live · Secrets

<Where the program writes state; it writes nothing to its own source tree, not even
gitignored. Config paths are public API. What sensitive material this project touches,
where redaction happens, and what must never be logged. If the repo is public, fixture
credentials must be recognisably fake and never a real-shaped token.>
