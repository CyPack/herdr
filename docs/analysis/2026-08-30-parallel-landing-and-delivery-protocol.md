# Parallel landing and delivery: what one day of two agents cost, and the rules that follow

2026-08-30. Two agents worked the same repository for one morning — one on the graphics and
flicker path, one on the remote-media and pane-audio path. Everything they built passed its own
tests. The morning still produced four separate incidents, none of them caused by bad code, all
of them caused by two agents sharing three resources without saying so out loud: **the landing
lock, the delivery pipe, the working tree, and the build fleet.**

This document records what happened, with times, and fixes the rules that follow from it. It is
written for the next pair of agents, who will otherwise rediscover all four.

---

## The four incidents

### I1 — the user waited 25 minutes for a fix that had already landed

`13a` (the standalone-graphics sync fix) landed on master at 09:47. The user, who was watching
for that exact fix, still saw the old behaviour at 10:02.

The deliverer had skipped every tick from 09:41 to 09:56:

```
09:41:35 pas: iniş sürüyor ve çalışan ağaç KİRLİ — karışık ağaç derlenmez (5 dk sonra yeniden)
09:46:35 pas: ...
09:51:36 pas: ...
09:56:37 iniş kilidi dolu ama ağaç <sha> ile tutarlı ve yük 4 ≤ 25 — teslim nice'lı yürüyor
```

Its skip condition is **landing lock held ∧ tree unstable**. Two agents landing back to back kept
the first half true; untracked files in the main checkout kept the second half true. The
deliverer never found a window.

**Landing is not delivering.** A merged commit changes nothing the user can see until the
deliverer builds and installs it. While two agents queue for the landing lock, the delivery pipe
is exactly the thing they are starving.

### I2 — eleven consecutive landing refusals

09:52–09:55, one agent's `wt.sh auto` was refused eleven times:

```
09:54:14 feat/pane-audio-supervisor END REFUSED reason=lock
09:55:37 AUTO STOP lock-wait-exhausted
```

The lock was held by the other agent's `cli-spec` landing, which had started at 09:48. Nobody was
wrong: landing is serial by design. What was missing was a **word before the attempt**. A
heads-up message crossed with the other agent's launch, so both were correct and both lost time.

### I3 — the main checkout's untracked files blocked delivery

The skip above was half-caused by untracked draft documents in `~/projects/herdr`. `.local/` is
git-ignored and safe to write; anything else in the main checkout is not. A scratch file in
`docs/` is indistinguishable, to the deliverer, from a half-finished edit.

### I4 — a test race that had been asleep for months woke up on a new build fleet

A gate rejected an unrelated branch three times on
`server::headless::tests::a_cell_size_change_sweeps_the_graphics_too`. The branch was innocent —
944 insertions, zero deletions, not one line of an existing file changed — and its own full suite
passed 6230/6230.

The cause was neither the branch nor the merge: `ClientWriter::test_channel` forwards through a
thread, and the test drained with a bare `try_recv`. On a saturated box the forwarder had not been
scheduled yet. The race dates from `1777e9bb`; it lost for the first time on 2026-08-30 because
**the build fleet was created the day before**, and full-suite gates finally ran on machines that
could be saturated.

A defect's birth date and its first sighting are different dates. "What did we change today" is
the wrong first question when the environment also changed.

---

## The rules

### R1 · Delivery outranks landing

Before taking the landing lock, look at when something last reached the user:

```
tail -3 ~/.local/state/herdr-auto-deliver/deliver.log
```

If the deliverer is mid-build or has been skipping, **wait**. A landing that delays a delivery
trades a user-visible fix for a merge-order preference.

### R2 · Announce before you take the lock, and say when you release it

The lock serialises correctly on its own; the message is for the *other agent's planning*, not for
correctness. One line before (`taking the lock, ~N minutes`) and one line after (`slot free`) is
enough. `wt.sh mail` survives both sessions dying; a live `SendMessage` is faster when both are up.
Send both when the message changes what the other agent would do next.

### R3 · The main checkout stays clean

Write scratch files, plans and measurements under `.local/` (git-ignored) or in the agent's own
scratchpad. Never leave untracked files in the main checkout: they are indistinguishable from
work in progress and they stop the delivery pipe.

### R4 · One owner per file, declared before the first edit

```
wt.sh claim <path>...      # announce
wt.sh claims               # who declared what
wt.sh overlap              # who is actually in whose files
```

A claim is not a lock. Its job is to make an intersection **visible before it becomes a conflict**,
which is the only form conflicts can be resolved cheaply in.

### R5 · A gate rejection is a result, not a diagnosis — and the measurement conditions are part of the result

The gate says so itself. Separating "the merge produced it" from "the test is fragile" needs the
same test run on trunk alone and on the branch alone — **under the gate's own conditions**. A
narrow filter run on an idle box is a different experiment from a full parallel suite on a busy
one, and treating one as evidence for the other wastes a full turn. Record every measurement as
*run · condition · result*, and treat an unmeasured cell as unmeasured, not as green.

The fleet's own log answers the ownership question without asking anyone:

```
~/projects/compute-fleet/state/runs.jsonl     # time · node · rc · source tree · command
```

That log is what proved trunk had already failed the same gate before the accused branch existed.

### R6 · Do not retract your own hypothesis on someone else's reading of the code

A colleague's "X calls Y directly" is a reading, not a measurement. If a hypothesis points at a
layer, open that layer yourself — it costs one `grep`. On 2026-08-30 a correct hypothesis
(asynchronous send, synchronous drain) was dropped because the channel was verified unbounded
while the *number of hops* was never checked. There was a forwarder thread.

### R8 · A defect is a class, not a point — sweep the layer and the dependency chain before closing it

The drain race was found in one test. The same wrong idiom lives in every graphics test in that
file, while `next_window_title` in the *same file* had it right all along with a 5 s
`recv_timeout`. One test failed; a convention was wrong.

So the moment a defect is understood, before the fix is written:

1. **Name the class.** "This assert blew up" is a point. "An asynchronous send is read with a
   synchronous drain" is a class. If the class cannot be named, the root cause is not found yet.
2. **Search for the class.** Who else writes it this way? A wrong convention spreads by copying,
   so the file where it was found is rarely its only home.
3. **Walk the dependency chain.** Who calls this layer, and what does it call? A defect visible in
   one layer often means the layer above and the layer below share its assumption.
4. **Check the movements before and after.** Along the same data path, does the invariant still
   hold one step earlier and one step later? Whatever broke ordering, timing or ownership here is
   usually also broken next door, where nobody has looked yet.
5. **Find the sibling that gets it right.** It gives the fix its shape, and the question "why are
   these two different?" traces how the defect spread.

The sweep ends when the class has been **enumerated**, not when the first instance is fixed. Every
further instance is either fixed in the same commit or written down by name. Silence is how the
second half of a class survives into the next session.

### R7 · Never skip the gate to get past a red you did not diagnose

`WT_SKIP_GATE=1` is not an agent's tool. When the gate refuses, the honest moves are: diagnose,
fix, or hand the decision to the человек who owns the trade-off — in that order.

---

## What this cost, and what it bought

Four incidents, roughly two hours of agent time and 25 minutes of user-visible delay. In exchange:
a test race that had been latent since `1777e9bb` is now understood and fixed, the delivery pipe's
skip condition is written down, and the fleet log is established as the arbiter of "whose run was
that".

The rules above are cheap. Every one of them is a sentence said before an action, or a file
written somewhere other than the main checkout.
