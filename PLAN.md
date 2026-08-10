# <PROJECT> — Build Plan

<One paragraph: what this is and what "finished" looks like.> Each numbered item is
sized to be roughly one prompt / one focused work session, and each stage ends at a
point where the thing is genuinely usable.

---

## 0. Architecture & Decisions

### Module layout

| Module | Responsibility | I/O? |
|---|---|---|
| `<Module>` | <what it owns> | <none / yes> |

### Key technical choices

- **<Choice>.** <Why, in one or two sentences. The reasoning, not the restatement.>

### Settled

<Short prose paragraph of what is no longer up for discussion: repo layout, workflow,
dependency policy, distribution. Point at the decision entries in `BUILD-LOG.md` for the
reasoning rather than repeating it.>

### Still open

**This is the single list of open questions.** Anything awaiting a decision belongs
here, whatever `BUILD-LOG.md` entry first raised it — that log is append-only, so
questions buried in it are questions nobody finds. Delete an item from this list when it
is answered, and record the answer as a decision entry.

The blocking annotation is machine-read: `*(blocking: prompt 5)*` or
`*(blocking: stage 2 prompt 5)*`, or `*(not blocking)*`. `make check` refuses to let a
blocked prompt become the next one — a question run into mid-prompt gets answered by
whatever is expedient that session, then shipped as public API the real answer has to
migrate. Downgrading blocking → not blocking is a decision, and gets a decision entry.

- **<Question?>** *(blocking: prompt N)* <What makes it hard, which candidates were
  considered and why each fails, and where the answer has to be settled. If a later
  prompt has already shipped something that the answer will change, say so explicitly
  and name the symbol — that turns "decide this" into "decide this and migrate that".>

### Testing strategy

- <Unit-level: corpora, fixtures, upstream SHAs.>
- <Integration-level: fakes, containers, local servers.>
- **A scripted live run per prompt.** <How. State what makes it cheap enough to actually
  do — isolated config directory, a pre-seeded fixture, a scripted second party — because
  a live-run rule that is expensive is a live-run rule that gets skipped.>

### Carry-forward notes

Items below may carry a `### Carry-forward` block, appended when earlier work turns up
something that item needs to know. Consume and delete it when the item is built, and
record in `BUILD-LOG.md` that you did. This is the same convention the prompt files use,
extended to stages that have no prompt file yet.

---

## Stage 1 — <name> (<the one-line goal>)

Target: <the concrete end state>.

Stage 1 is broken into <N> prompts in **`STAGE-1-PROMPTS.md`**, which is authoritative
for scope, ordering, and status. It is deliberately not summarized here — two copies of
the same list drift, and the copy nobody edits is the one that gets read.

**Done when:** <the acceptance sentence, phrased as something a person does>.

---

## Stage 2 — <name>

1. **<Item name>.** <Two to five lines of scope. Enough that a prompt can be written
   from it months later; not so much that it duplicates the prompt.>

2. **<Item name>.** <...>

   ### Carry-forward
   - From stage 1 prompt 6: <what was learned, naming the file and symbol it lands on>.

**Done when:** <...>

---

## Stage 3 — <name>

<...>

---

## Stage 4 — Polish & release

<Packaging, signing, distribution, docs, the rename if there is one.>

---

## Suggested order of attack

<A short paragraph on sequencing and why — what unblocks what, and what is deliberately
left until the shape of something else is known.>
