# S148PORMOTBAC-006: IntentionFrame BDI extension with motive refs and lifecycle conditions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — extends `IntentionFrame` (`crates/worldwake-core/src/intention_frame.rs:138`) with five new authoritative fields: `motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, `causal_links`; migrates 18+ strict-literal construction sites and ~70 looser-match construction sites across 17 files
**Deps**: `archive/tickets/S148PORMOTBAC-005.md`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S148 PR-1's BDI extension folds the assessment's recommended motive-backed intention fields into `IntentionFrame`. Today's frame carries goal, domain, assumptions, state, established_at, last_progress_tick, stalled_ticks, patience_limit — but no record of *why* it was adopted (motives), *what holds it together* (claims, causal links), or *when it should resume or abandon* (lifecycle conditions). Spec S148 D6 adds five new fields; D8 (subsumed) defines the `explicit_claims: Vec<EntityId>` semantics against real artifact types (`ContentionGrant`-bearing facility queues, `SaleListing`-bearing lots, `ArtifactHeader`-bearing social artifacts); D9 (subsumed) documents the `causal_links_per_step_cap` contract (enforcement lands in ticket 007 where push sites materialize).

## Assumption Reassessment (2026-05-17)

1. Current `IntentionFrame` shape at `crates/worldwake-core/src/intention_frame.rs:138`: `goal: GoalKey, domain: IntentionDomain, assumptions: Vec<FrameAssumption>, state: FrameState, established_at: Tick, last_progress_tick: Option<Tick>, stalled_ticks: u32, patience_limit: u32`. Derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. **Does NOT derive `Default`** — every construction site enumerates fields explicitly. No spread-syntax escape hatch for full constructions; one partial-spread pattern exists in tests (`..frame_active.clone()`) which works for new fields when the cloned frame already has them.
2. Construction site blast radius from grep: 18 strict-literal sites (`^IntentionFrame {$`), 70+ looser-match sites across 17 production + test files. Strict-literal locations: `crates/worldwake-core/src/intention_frame.rs:261`, `crates/worldwake-ai/src/feasibility.rs:567`, `crates/worldwake-ai/src/failure_handling.rs:2170`, `crates/worldwake-ai/src/decision_runtime.rs:412`, `crates/worldwake-ai/src/agent_tick/frame.rs:506,521,964,2073`, `crates/worldwake-systems/src/perception.rs:1709,2150`, `crates/worldwake-systems/src/travel_actions.rs:861`, `crates/worldwake-systems/src/sleep_synthesis.rs:107`, `crates/worldwake-systems/src/needs_actions.rs:1655`, `crates/worldwake-ai/tests/golden_sleep_episode.rs:144`, `crates/worldwake-ai/tests/golden_harness/commodity_assumption_falsification.rs:282`, `crates/worldwake-ai/src/agent_tick/tests.rs:2383,2912,8355`. Counts >15 → effort tracks the count; >15 with no spread-syntax escape → Large.
3. Shared abstraction under audit: the core-resident `IntentionFrame` struct shape. Save/load impact: `IntentionFrame` is part of save state via `AgentBeliefStore`-adjacent serialization. New fields use `#[serde(default)]` per spec D6 so existing serialized state continues to deserialize without bumping `SAVE_FORMAT_VERSION` (= 90 at `crates/worldwake-sim/src/save_load.rs:7`).
4. New field types (all core-resident after ticket 005 lands): `motive_refs: Vec<MotiveSourceRef>` (core, `motive_source.rs:57`), `resume_conditions: Vec<IntentionResumeCondition>` (core, ticket 005), `abandon_conditions: Vec<IntentionAbandonCondition>` (core, ticket 005), `explicit_claims: Vec<EntityId>` (core, `ids.rs`), `causal_links: Vec<EventId>` (core, `ids.rs`). All have natural `Vec::new()` defaults so construction sites can append `motive_refs: Vec::new(), …` mechanically.
5. Existing tests exercising IntentionFrame state transitions (frame.rs `#[cfg(test)]` block from agent inspection): ~10 tests including `assess_commodity_availability_co_located_lot_returns_believed:356`, `populate_travel_produces_route_exists:440`, `populate_care_produces_target_alive_and_route:463`, `populate_escort_produces_target_alive_and_route:489`, `populate_errand_produces_route_exists:520`. Existing portfolio/golden tests also construct `IntentionFrame` fixtures.

## Architecture Check

1. FND-21 alignment: explicit `motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, and `causal_links` make every commitment traceable. The frame now carries the full evidence record for why it exists, what holds it together, and what conditions revise it.
2. FND-29A alignment: `causal_links: Vec<EventId>` is bounded by `CognitiveProfile.causal_links_per_step_cap` (already at `cognitive_profile.rs:125`) per spec D9. The cap contract is documented at the type definition; FIFO eviction at push sites lands in ticket 007 (where push sites materialize via the evaluator).
3. FND-28 clean migration: new fields appended at the end of the struct; `#[serde(default)]` annotations let pre-bump save state deserialize without a custom impl; no parallel struct, no shim, no version bump.
4. New fields are all `Vec<T>` with natural `Default` (`Vec::new()`); construction sites add five `: Vec::new()` initializers mechanically. No structural surprise.

## Verification Layers

1. `IntentionFrame` serde round-trip with new fields → focused unit test in `crates/worldwake-core/src/intention_frame.rs::tests` exercising serialization with non-empty vectors
2. Pre-bump serialized state tolerance → focused test asserting that an `IntentionFrame` serialized before the new fields were added still deserializes (constructs the new fields as `Vec::new()` via `#[serde(default)]`)
3. Construction-site migration completeness → workspace compilation under `cargo clippy --workspace --all-targets -- -D warnings` (any unmigrated `IntentionFrame {…}` literal fails compile)

## What to Change

### 1. Extend `IntentionFrame` with five new fields

In `crates/worldwake-core/src/intention_frame.rs:138`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionFrame {
    pub goal: GoalKey,
    pub domain: IntentionDomain,
    pub assumptions: Vec<FrameAssumption>,
    pub state: FrameState,
    pub established_at: Tick,
    pub last_progress_tick: Option<Tick>,
    pub stalled_ticks: u32,
    pub patience_limit: u32,
    // New in S148:
    #[serde(default)]
    pub motive_refs: Vec<MotiveSourceRef>,
    #[serde(default)]
    pub resume_conditions: Vec<IntentionResumeCondition>,
    #[serde(default)]
    pub abandon_conditions: Vec<IntentionAbandonCondition>,
    #[serde(default)]
    pub explicit_claims: Vec<EntityId>,
    #[serde(default)]
    pub causal_links: Vec<EventId>,
}
```

Add required `use` lines for `MotiveSourceRef`, `IntentionResumeCondition`, `IntentionAbandonCondition`, `EventId` (all core-resident after ticket 005).

Document the `explicit_claims` semantics inline as a doc-comment naming the valid entity types: `ContentionGrant`-bearing facility-queue grants (per `crates/worldwake-core/src/contention.rs:43`), `SaleListing`-bearing lots (per `crates/worldwake-core/src/trade.rs:25`), `ArtifactHeader`-bearing social artifacts (per `crates/worldwake-core/src/social_artifact.rs`). D8's narrative lives in spec text; the type-level contract is the doc-comment.

Document the `causal_links` cap contract as a doc-comment naming `CognitiveProfile.causal_links_per_step_cap` as the bound. FIFO eviction enforcement lives at push sites in ticket 007; this ticket's contract is "callers SHOULD bound this vector by `causal_links_per_step_cap`."

### 2. Migrate strict-literal construction sites (18 production sites)

For each of the 18 strict-literal `IntentionFrame {` sites, append the five new fields with `Vec::new()`:

```rust
IntentionFrame {
    goal,
    domain: IntentionDomain::Travel { destination },
    assumptions: Vec::new(),
    state: FrameState::Active,
    established_at: Tick(3),
    last_progress_tick: None,
    stalled_ticks: 0,
    patience_limit: 30,
    motive_refs: Vec::new(),
    resume_conditions: Vec::new(),
    abandon_conditions: Vec::new(),
    explicit_claims: Vec::new(),
    causal_links: Vec::new(),
}
```

Construction sites where the caller has concrete motive context (e.g., the agenda-manager promotion path that knows the motive contributions backing the new intention) populate `motive_refs` from the actual motive source rather than `Vec::new()`. The default population happens in ticket 007's evaluator and the per-tick agenda-manager wiring; this ticket's role is to make every site compile with the new shape.

### 3. Migrate looser-match construction sites (~52 additional test/fixture sites)

Grep `IntentionFrame \{` workspace-wide and apply the same field-append to every remaining site. Tests in `agent_tick/tests.rs` have ~20 sites; `agent_tick/frame.rs` tests have ~16 sites; the rest are distributed across other test files. The `..frame_active.clone()` partial-spread test pattern continues to work once the cloned frame has the new fields.

### 4. RON scenario audit

Grep `scenarios/**/*.ron` for `IntentionFrame` references. Since `IntentionFrame` is runtime-generated state (not scenario-authored), there should be no RON references. If any are found, they require migration; otherwise the audit confirms no scenario impact.

## Files to Touch

- `crates/worldwake-core/src/intention_frame.rs` (modify — extend struct definition with five new fields + doc-comments; migrate construction site at line 261; extend existing tests)
- `crates/worldwake-ai/src/feasibility.rs` (modify — construction site at line 567)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — construction site at line 2170)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — construction site at line 412 + 4 additional looser sites per the per-file count)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — strict sites at 506, 521, 964, 2073 + ~12 looser sites; this is the highest-count file)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — strict sites at 2383, 2912, 8355 + ~17 looser sites)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — 1 looser site)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — 2 looser sites)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — 2 looser sites)
- `crates/worldwake-ai/src/interrupts.rs` (modify — 4 looser sites)
- `crates/worldwake-systems/src/perception.rs` (modify — strict sites at 1709, 2150 + 1 looser site)
- `crates/worldwake-systems/src/travel_actions.rs` (modify — strict site at 861)
- `crates/worldwake-systems/src/sleep_synthesis.rs` (modify — strict site at 107 + 1 looser site)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — strict site at 1655)
- `crates/worldwake-core/src/delta.rs` (modify — 1 looser site if the macro expansion materializes IntentionFrame construction)
- `crates/worldwake-ai/tests/golden_sleep_episode.rs` (modify — strict site at 144)
- `crates/worldwake-ai/tests/golden_harness/commodity_assumption_falsification.rs` (modify — strict site at 282)

## Out of Scope

- The resume/abandon condition evaluator and `Discrepancy::AbandonConditionFired` variant (ticket 007)
- `causal_links` push sites and FIFO-eviction enforcement (ticket 007 — push sites only materialize in the evaluator)
- Observer rendering of new IntentionFrame fields (ticket 009)
- Golden coverage exercising the new fields (ticket 010)
- Per-call-site population of `motive_refs` with real motive sources (ticket 007 wires the agenda-manager promotion path; this ticket initializes to `Vec::new()` at construction)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core intention_frame` — new and migrated tests pass: serde round-trip including non-empty new fields; pre-bump serialized state tolerance test (deserializes serialized-without-new-fields blob and verifies new fields are `Vec::new()`)
2. Existing `agent_tick/frame.rs::tests` (10+ tests) pass after construction-site migration
3. Existing suite: `cargo test --workspace`
4. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `IntentionFrame` carries exactly five new fields (`motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, `causal_links`); all `Vec<T>`; all `#[serde(default)]`.
2. Every existing `IntentionFrame {` construction site compiles after migration (the absence of unmigrated sites is enforced by the compiler since the struct does not derive `Default`).
3. `SAVE_FORMAT_VERSION` is not bumped (= 90 stable); pre-S148 serialized `IntentionFrame` state deserializes via `#[serde(default)]` with the new fields populated as `Vec::new()`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/intention_frame.rs::tests` — add: `intention_frame_serde_round_trip_with_new_fields`, `intention_frame_deserializes_pre_s148_state_with_serde_default`
2. `crates/worldwake-ai/src/agent_tick/frame.rs::tests` — existing tests continue passing after fixture migration; no new tests required (lifecycle tests land with ticket 007's evaluator)

### Commands

1. `cargo test -p worldwake-core intention_frame`
2. `cargo test -p worldwake-ai agent_tick::frame`
3. `cargo test --workspace` (the wide test surface catches construction-site fix-ups)
4. `./scripts/verify.sh`
