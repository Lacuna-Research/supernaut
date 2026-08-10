# dev-template

A project-scaffolding template for LLM-assisted software work, extracted from a real
working project. It exists to solve one problem: **an agent with no memory,
working across dozens of sessions, cannot be relied on to keep a promise it made in a
previous session.** Everything here is either a way to write things down so a cold
session finds them, or a way to make a machine check that they were written.

**`METHOD.md` is the document to read** — it explains every mechanism and why it
exists. This file is only the front door.

## What's in the box

| File | Role |
|---|---|
| `CLAUDE.md` | Operative rules only — hard-capped at 100 lines, mechanically |
| `PLAN.md` | Living roadmap, plus the single list of open questions |
| `STAGE-1-PROMPTS.md` | The work queue; carry-forward notes land here |
| `BUILD-LOG.md` | Append-only reasoning: decisions, corrections, retrospectives |
| `SUBAGENT-BRIEFS.md` | Dispatch briefs for the three sub-agent roles |
| `scripts/check-docs.sh` | The enforcement: 14+ checks run by pre-commit and CI |
| `scripts/test-checks.sh` | Fixtures for every check — a broken check fails open |
| `ratchets.txt` | One-way ceilings for slow-rot metrics |

## Using it

1. Create the new repo with **Use this template** on GitHub (this repo is marked as
   a template), so each project starts with clean history.
2. Drop a **`north-star.md`** in the root — the overall outline of what you are
   building and what finished looks like. The first working session turns it into
   `PLAN.md` stages and the stage-1 prompt queue via the **Plan bootstrap** brief in
   `SUBAGENT-BRIEFS.md`. The north-star stays as the durable statement of intent;
   the plan is authoritative for the roadmap from then on.
3. Run **`make bootstrap`** once. It installs the git hooks, turns on branch
   protection with the three discipline checks required (needs an authenticated
   `gh`), and prints the adaptation work that remains.
4. Work through the **adaptation checklist at the bottom of `METHOD.md`** — replace
   the `<PLACEHOLDER>`s, set the stage table and source directory in
   `check-docs.sh`, fill in the `Makefile` bodies, baseline the ratchets.
5. Run `./scripts/test-checks.sh` after any edit to `check-docs.sh` — it is the only
   thing that catches an edit that quietly disabled a rule.

Until the `Makefile` is adapted, CI's `build` job skips itself (loudly); the
`discipline`, `discipline-tests` and `secrets` jobs run for real from the first
commit, and are meant to be required status checks on `main`.
