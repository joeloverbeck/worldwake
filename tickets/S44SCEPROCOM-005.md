# S44SCEPROCOM-005: Documentation — spec-drafting-rules.md + CLAUDE.md profile contract

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

No documentation exists to prevent future specs from repeating the S44 gap — adding agent profile components to the ECS without making them scenario-definable. Without a documented contract, every new profile spec risks silently creating agents that lack the new capability when spawned from scenarios.

## Assumption Reassessment (2026-04-03)

1. `docs/spec-drafting-rules.md` exists at the expected path. Currently has 16 numbered sections covering phase distinction, layer precision, coverage gaps, ordering, verification surfaces, etc. No section about agent profile scenario completeness. Confirmed.
2. `CLAUDE.md` has a "Critical Invariants" section listing non-negotiable design rules. No mention of scenario profile completeness. Confirmed.
3. The spec-drafting-rules.md is referenced by `tickets/README.md` and `CLAUDE.md` — additions to it are automatically picked up by the ticket authoring workflow.
4. This ticket is purely documentation — no code changes, no behavior changes.

## Architecture Check

1. Documentation prevents recurrence of a real architectural gap. The cost is one new section in spec-drafting-rules.md and one bullet in CLAUDE.md. The benefit is that every future spec that adds an agent profile is reminded to wire it into the scenario system.
2. No backwards-compatibility concerns — purely additive documentation.

## Verification Layers

1. Documentation completeness -> manual review: spec-drafting-rules.md has a new section with the 5-point checklist
2. CLAUDE.md invariant -> manual review: Critical Invariants section includes scenario profile completeness
3. Single-layer (documentation) — no code verification needed

## What to Change

### 1. Add Agent Profile Scenario Contract to spec-drafting-rules.md

In `docs/spec-drafting-rules.md`, add as a new numbered section (after the last existing section):

```markdown
## 17. Agent Profile Scenario Contract

Every spec that adds a new ECS component registered on `EntityKind::Agent` that
affects agent behavior must:

1. Classify the component as **universal** (every agent needs it to function as
   a reasoning, perceiving, socially-participating agent) or **role-specific**
   (only relevant for agents in specific roles).
2. Add the component to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`.
   If the component contains `EntityId` references, create a `*Def` wrapper type
   with string names (following the `MerchandiseProfileDef` / `PatrolRouteDef` pattern).
3. Add the `set_component_*` call in `spawn_agent()` in
   `crates/worldwake-cli/src/scenario/mod.rs`:
   - Universal: `unwrap_or_default()` — always applied.
   - Role-specific: conditional `if let Some(...)` — applied only if present in RON.
4. Universal profiles must have a `Default` impl.
5. Runtime access to universal profiles on known agents uses `expect()`, not
   silent fallback.

Components that are purely runtime-generated state (ActiveGoal, IntentionFrame,
WoundList, etc.) are exempt — they emerge from simulation, not configuration.

Any new ECS component that affects agent behavior must be exercisable through the
scenario system. If a component changes what an agent can do, perceive, decide, or
communicate, a scenario author must be able to configure it. Silent absence of
behavioral components is a bug, not a feature.
```

### 2. Add scenario profile completeness invariant to CLAUDE.md

In the "Critical Invariants" section of `CLAUDE.md`, add:

```markdown
- **Scenario profile completeness** — every agent profile component registered on
  `EntityKind::Agent` must be scenario-definable via `AgentDef` + `spawn_agent()`.
  Universal profiles are always applied (with defaults). See `docs/spec-drafting-rules.md`
  section 17 for the checklist.
```

## Files to Touch

- `docs/spec-drafting-rules.md` (modify) — add section 17
- `CLAUDE.md` (modify) — add invariant to Critical Invariants

## Out of Scope

- Code changes to types.rs, mod.rs, or any Rust source
- Scenario RON updates
- Golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Invariants

1. `docs/spec-drafting-rules.md` section 17 contains the 5-point agent profile checklist
2. `CLAUDE.md` Critical Invariants section contains the scenario profile completeness bullet
3. Both documents are consistent — CLAUDE.md references spec-drafting-rules.md section 17

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `grep -c "Agent Profile Scenario Contract" docs/spec-drafting-rules.md` — should return 1
2. `grep -c "Scenario profile completeness" CLAUDE.md` — should return 1
