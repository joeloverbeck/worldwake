# Combat Commitment Substrate

**Date**: 2026-03-21
**Status**: Draft report
**Context**: Investigation following `S16S09GOLVAL-003`

## Summary

`defend` is already a real authoritative action lifecycle in the engine, but the AI does not currently have an explicit combat-commitment substrate. That gap makes multi-agent defend behavior unstable and hard to test. The current runtime owns goals and plans such as `ReduceDanger`, but it does not model a durable combat commitment like "hold defensive posture against this threat set until one of these concrete stop conditions occurs."

The clean direction is not to hardcode "agents should defend longer." The clean direction is to add an explicit, state-backed commitment layer for combat response, then let AI and action execution read and update that commitment lawfully.

## Diagnosis

Today the stack is split like this:

1. Authoritative action layer:
   - `defend` has a lawful start/tick/commit/abort lifecycle.
   - `CombatStance::Defending` changes real combat math.
2. AI layer:
   - `ReduceDanger` is a reactive goal candidate.
   - `defend` is only one possible plan step under that goal.
   - the active action is freely interruptible.
3. Missing layer:
   - no explicit combat commitment artifact saying what danger the agent is currently committed against, why, and what keeps that commitment alive.

Because that middle layer is missing, defend is vulnerable to lawful but unstable branch switching:

- danger can disappear from candidate generation when pressure/evidence changes
- self-care can outrank danger immediately
- different agents can diverge for incidental local reasons before any defend lifecycle becomes established
- tests can seed authoritative defend occupancy, but that does not automatically create a matching AI-owned commitment

## Necessary Substrate

The minimum explicit substrate should include all of the following.

### 1. Combat Commitment State

Add a runtime-owned combat commitment record per agent. It should not be a vague boolean like `is_defending`. It should encode:

- commitment kind: defend, engage, flee, reposition
- threat provenance: concrete hostile entities and/or evidence set
- commitment start tick
- last validated tick
- intended tactical role
- explicit stop conditions
- optional paired goal key / planned branch identity

This should live in AI/runtime state, not only in the action instance.

### 2. Commitment Entry Rules

The engine needs an explicit rule for when `ReduceDanger` becomes a combat commitment rather than a one-tick reactive preference. Example triggers:

- direct current attacker
- visible hostile above danger threshold
- recent combat event affecting self
- observed ally under attack in same local combat context, if later supported

This is where the agent transitions from "danger is high" to "I am now committed to a combat response."

### 3. Commitment Continuation Rules

The commitment needs revalidation rules distinct from generic ranking. A defend commitment should continue while:

- the threat set is still valid or recently valid
- the current stance/action is still lawful
- no higher-priority override condition fires

This should be stronger than ordinary replanning noise, but still revisable under Principle 19.

### 4. Explicit Stop Conditions

The commitment must end for concrete reasons, not because ranking happened to drift:

- all concrete threats neutralized or gone
- threat no longer locally believed
- authoritative action invalidated
- self-care crosses a defined interrupt threshold
- a higher combat-mode commitment supersedes it

These reasons should be traceable.

### 5. Interrupt Policy Specific To Combat Commitments

Right now `defend` is just a freely interruptible action. That is too weak for stable combat commitment. The system needs a combat-specific interrupt policy:

- ordinary same-class motive noise should not eject a live defend commitment
- critical survival conditions still must
- threat collapse should end the commitment cleanly
- stronger combat alternatives may replace it if justified

This is not "make defend non-interruptible." It is "make interrupts commitment-aware."

### 6. Traceability

Decision traces should expose:

- commitment created
- commitment refreshed
- commitment continued
- commitment switched
- commitment ended
- exact reason

Without that, goldens and debugging will keep overfitting to incidental action order.

## Desirable Extensions

These are not required for the first pass, but they would make the design materially better.

### 1. Tactical Role Separation

Separate "reduce danger" into concrete tactical roles such as:

- hold stance
- counterattack
- withdraw
- protect target

The commitment artifact should record which role the agent is executing.

### 2. Shared Local Combat Context

For multi-agent stability, add a local combat-context view so nearby agents can independently perceive that they are responding to the same fight without introducing omniscient coordination.

### 3. Commitment Hysteresis

Add bounded hysteresis so agents do not thrash between defend and self-care every tick near the threshold boundary.

### 4. Group Coordination Artifacts

Only after the individual commitment layer is sound:

- ally protection commitments
- role differentiation
- explicit handoff ("I stop defending because ally took over")

That should come later, not in the first substrate pass.

## What Should Not Be Done

Avoid these shortcuts:

- hardcoding that combat always outranks hunger/wounds
- making `defend` permanently sticky regardless of local evidence
- adding a hidden squad manager that assigns combat behavior omnisciently
- solving this with ticket/test-only timing assumptions

Those would violate the project’s architecture goals.

## Recommended Implementation Order

1. Add runtime combat-commitment state and trace surface.
2. Teach AI read/interrupt phases to continue or end commitments explicitly.
3. Make `ReduceDanger` planning adopt a commitment-aware continuation path.
4. Add focused runtime tests for commitment creation/continuation/end.
5. Add defend-expiry goldens only after the commitment substrate is stable.

## Why This Is Worth Doing

This substrate would make combat behavior:

- more legible
- more testable
- more deterministic
- more extensible for future group tactics

Most importantly, it would make `defend` a real AI-owned commitment rather than just a transient action choice under a volatile reactive goal.
