# Sub-agent briefs

Static reference — copy the relevant brief when dispatching a sub-agent, filling the
angle brackets. These exist because a sub-agent's value here is **context isolation**:
each role works precisely because the agent does *not* have the implementing session's
context, and the fastest way to ruin one is to paste the conversation in alongside the
brief. Give each agent exactly the inputs its brief names, and nothing else.

Two standing rules, from `CLAUDE.md`:

- **Sub-agents advise; artifacts decide.** Output lands as a proposal in an existing
  artifact — a carry-forward note, a log-entry section, a Still open item. Never a new
  file, never a new channel, never an authority.
- Every proposal gets a disposition — adopted, edited, or rejected-because — recorded
  where it landed. A rejected proposal is information too.

---

## 1. Post-prompt review *(finishing step 2 — one dispatch, two jobs)*

**Inputs:** the prompt block verbatim (including its Do-not fence and any carry-forward
notes it carried), the full diff, and the remaining queue — every not-yet-complete
prompt block plus the future-stage items in `PLAN.md`. **Not** the implementing
session, and not the build-log entry: the entry is the implementer's account, and this
review exists because the implementer spent a session rationalizing every deviation —
and is the worst person to spot what prompt 14 needs, because everything is obvious to
them right now.

One dispatch rather than two because the jobs share their main input and neither
contaminates the other; the second dispatch bought isolation that was already there.

> You are reviewing a change against the work order that produced it, on behalf of
> both the order and the work that has not started yet. You have the order, the diff,
> and the remaining work queue — nothing else. Produce three sections, with file and
> line references throughout:
>
> 1. **Does the diff do what the order says?** List each acceptance criterion and
>    each bullet of the order; for each, state whether the diff satisfies it and how
>    you can tell from the diff alone. Where you cannot tell from the diff alone, say
>    so — that is a finding, not a gap in your review.
> 2. **What landed that the "Do not" fence excludes?** Scope leaks backward through
>    plausible-looking additions. Name anything in the diff the order did not ask
>    for, whether or not it seems useful.
> 3. **Carry-forward proposals.** For each remaining prompt and plan item, ask: does
>    this diff create, move, rename, or constrain anything that work will touch?
>    Propose notes, each attached to one named prompt or item, in the form:
>    `- From prompt <N>: **<headline>.** <Detail naming the file and symbol; state
>    the trap, not just what exists.>` Propose only notes that would change how the
>    receiving prompt is written or executed — "be aware of X" is not a note.
>
> Do not soften findings and do not pad any list. An empty section is a valid answer.

**Disposition:** findings addressed or rejected-with-reason in the log entry's
`**Review:**` section; adopted notes appended to their prompts, with
`**Carry-forward raised:**` saying which proposals were rejected — a rejected proposal
is information too.

**Skip it for trivial changes** — docs-only, or far under the size cap with no fence
risk — and write `**Review:** skipped, trivial (<reason>)`. The honesty is the
requirement, not the ritual.

---

## 2. Just-in-time detail writer *(starting a prompt whose detail is unwritten)*

**Inputs:** the `PLAN.md` item(s) verbatim, the prompt's scope/grouping/fence stub and
its carry-forward notes, `CLAUDE.md`, and the current source files the notes name.
**Not** the previous prompt's session — the brief must not inherit the last prompt's
shape.

> Write the working detail for the prompt below, in the house style: an opening
> sentence stating what becomes true; bullets that each carry their reasoning; named
> files and symbols wherever the seam is already known; an acceptance paragraph
> phrased as something a person does and observes in a live run; and a "Do not"
> fence naming where each excluded thing belongs and why the split is right.
>
> The carry-forward notes attached to this prompt are constraints, not suggestions —
> the detail must visibly account for each one. Where a note and the plan item
> conflict, surface the conflict at the top rather than resolving it silently.

**Disposition:** the detail replaces the stub in the queue file, edited by this
session before starting.

---

## 3. Cold-start drill *(stage retrospective, and before planned context resets)*

**Inputs:** the repository. Nothing else — the drill measures whether the repo alone
is sufficient, which is the claim the entire system makes.

> You have just been handed this repository cold. Using only what is on disk, answer:
>
> 1. What is this project, and what stage is it at?
> 2. What is the next unit of work, exactly — and what would you read, in what order,
>    before starting it?
> 3. What open questions or constraints bear on that work?
> 4. Where were you left guessing? Name every place the documentation made you infer
>    rather than told you.
>
> Answer from the documents, citing them. Where documents disagree, report the
> disagreement rather than picking a side.

**Disposition:** the answer goes in the retrospective's `**Cold-start drill:**`
section, with every gap between it and reality treated as a documentation bug and
fixed before the next stage opens. No gaps is worth recording too — it is the only
direct evidence the system works.

---

## 4. Plan bootstrap *(once, when a project is created from the template)*

**Inputs:** `north-star.md` — the project's overall outline and destination — plus
`METHOD.md` and the placeholder `PLAN.md` and `STAGE-1-PROMPTS.md` as format
references. Nothing else exists yet, which is the point: this brief runs before there
is a codebase or a conversation to contaminate it.

> Turn the north-star document into the working artifacts. Produce:
>
> 1. **A `PLAN.md`** in the template's shape: stages that each end somewhere genuinely
>    usable, numbered items sized to one focused session, key technical choices with
>    their reasoning, and every question the north-star leaves open in the Still open
>    list, marked blocking or not. Where the north-star is ambiguous or contradicts
>    itself, that is a Still-open item — never resolve it silently.
> 2. **A `STAGE-1-PROMPTS.md`**: full detail for the first three or four prompts,
>    scope/grouping/fence stubs for the rest, and a correct status line matching the
>    prompt count.
>
> The north-star holds the why and the destination; reference it, do not paraphrase
> it into the plan — two copies of one intention drift.

**Disposition:** proposals reviewed and edited by the session, then committed as the
project's first PR — along with the `STAGES` total in `scripts/check-docs.sh`, which
must match the queue it just wrote. `north-star.md` stays in the repo as the durable
statement of intent, but from this moment `PLAN.md` is authoritative for the roadmap:
when they diverge, the plan wins and the divergence gets a decision entry.
