# Stage 1 — The Prompts

**Status:** 0/11 complete. Next: prompt 1.

<!-- 11 is the template default, matching STAGES in scripts/check-docs.sh — change
both together. The line is machine-read; `make check` fails if they disagree. -->

Stage 1's work queue. Every numbered item in `PLAN.md`'s stage 1 is attached to exactly
one prompt here; a few prompts carry two or three items, and the largest item may be
split across two. The grouping is by **shared seam**, not by theme — two items belong
together when they touch the same code, and apart when they merely sound related.

Each block is self-contained and assumes the previous ones are done and merged. Every
prompt has a **Do not** section: the scope fence that keeps later work from leaking
backward. Standing rules — <toolchain, target, test framework, zero warnings,
dependency policy> — live in `CLAUDE.md` and load automatically.

Each prompt is one branch (`prompt-NN-slug`), one PR, squash-merged once CI is green.
Bump the **Status** line above in the same PR; `make check` fails if it is missing or
malformed.

### Prompts are written just-in-time, and that is deliberate

Prompts 1–<K> are written out in full. **Prompts <K+1>–<N> carry their scope, their
grouping and their fence, but not yet their detail** — and are to be fleshed out
immediately before they start, not now.

Writing a detailed brief for work that six intervening prompts will have reshaped
produces a brief that is confidently wrong, and a wrong brief is worse than a thin one
because it is followed. The `PLAN.md` items behind these are the durable statement of
intent; a prompt is the short-lived working document that turns one into a session.
Reorder, rescope and merge these freely as the stage teaches you things — that is what
`BUILD-LOG.md` preserves the history for.

### Carry-forward notes

When work on one prompt turns up something a later prompt needs to know, the note is
appended to **that prompt, in this file**, under a `### Carry-forward` heading:

```
### Carry-forward
- From prompt 4: <Transport> reports `.ready` before the handshake completes, so do
  not start the registration timer here — wait for the first byte. The seam is
  `Transport.stateDidChange`.
```

Notes live in the destination rather than a separate file because this file is already
re-read at the start of every prompt. There is no second place to remember to check.

A note names a file and a symbol — one that says "think about X" is worth little; one
that names the seam is worth a session. A note is **deleted when the prompt that
received it runs**, and the fact that it was applied is recorded in `BUILD-LOG.md`.
`make check` fails if a note is still attached to a prompt the status line says is
complete.

For notes aimed at **stage 2 or later**, where there is no prompt to attach to, append
the same block under the relevant numbered item in `PLAN.md`. Same rule: consumed and
deleted when that item is built.

---

## Prompt 1 — <title>

**Item:** <PLAN.md item name, verbatim — the doc check matches on this string.>

```
<The prompt itself, in a fenced block so it can be copied whole into a session.>

<Opening sentence: what this prompt makes true that was not true before.>

- <Constraint or design point, with the reason attached. A bullet that says what to do
  without saying why gets overridden by the first plausible-looking alternative.>
- <Name the specific file/type/function this lands on where it is already known.>
- <Say what is *not* obvious: the trap, the ordering constraint, the thing that looks
  like it should work and does not.>

Acceptance: <something a person does and observes, not "tests pass". Phrase it as a
live run wherever possible.>

Do not: <the scope fence. Name the later prompt each excluded thing belongs to, and
say in one clause why the split is right — a fence with a reason survives; a bare
prohibition gets argued with.>
```

---

## Prompt 2 — <title>

**Items:** <item name> · <item name>

```
<...>
```

---

## Prompt <K+1> — <title>

**Item:** <item name>

<Two to four lines: the scope, and why these items are one prompt rather than two.>

**Examined for a split and left whole / split out of prompt N**, because <reason>.
Revisit if <the condition that would change the answer>.

*To be written out before it starts.*

**Carry-forward** *(consumed when this prompt runs)*

- From prompt <N>: **<the headline, in bold — a note is skimmed before it is read>.**
  <The detail, naming the file and symbol. Say what the trap is, not just what exists.>
- From prompt <N>: **decide explicitly whether <A> composes with <B> or replaces it**,
  and say which in `BUILD-LOG.md`. <Why the ambiguity is dangerous.>

---

<!--
Once a prompt completes, append its outcome under the block, in this shape. Keep it to
two or three sentences and put the reasoning in BUILD-LOG.md, not here:

**Status:** complete. <What shipped differently from the block above, and where that was
recorded.> <Anything deliberately left without a test, and why.>
-->
