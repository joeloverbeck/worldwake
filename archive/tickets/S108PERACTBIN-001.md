# S108PERACTBIN-001: Add BindingStrictness enum, ActionDef field, classification, and pure helper

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `ActionDef` gains a required `binding_strictness` field; every registered action receives an authorial classification. No behavioral change yet — the gate lands in S108PERACTBIN-002.
**Deps**: specs/S108-per-action-binding-strictness.md (D1, D2, and helper portion of D3)

## Problem

`ActionRequestMode::BestEffort` is permissive-by-default: when `resolve_affordance` in `crates/worldwake-sim/src/tick_step.rs` fails to reproduce an affordance, it synthesizes one from the raw requested targets without consulting any per-action binding contract. Identity-bound actions (`accuse`, `loot`, `heal`, `escort_to_safety`, etc.) can silently retarget to a different entity than the planner intended. There is no authoritative data to consult at the dispatch site — `TargetSpec` describes enumeration, not post-binding substitution policy.

This ticket lands the foundation: the `BindingStrictness` enum, the required `ActionDef::binding_strictness` field, a serde default for save/replay compatibility, the pure `check_binding_strictness` helper predicate, and authoritative classifications for every currently-registered action. The enforcement gate (T-002), revalidation gate (T-003), trace field (T-004), and integration tests (T-005) build on this foundation.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ActionDef` is defined at `crates/worldwake-sim/src/action_def.rs:12` with derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. The existing `assert_traits::<ActionDef>()` test at `action_def.rs:97` and the bincode roundtrip test at `action_def.rs:143` both exist and must be extended to cover the new field. No existing test in `action_def.rs`'s `#[cfg(test)]` block asserts the absence of a `binding_strictness` field, so the new field does not contradict an existing invariant.
2. The spec for S108 was just reassessed (`specs/S108-per-action-binding-strictness.md`, reassessment session 2026-04-18). D1 (enum variants), D2 (classification table), D3 (helper signature), serde-default rationale, and non-`Option` field commitment were all confirmed. The spec's D2 classification table is authoritative for this ticket.
3. Shared abstraction boundary under audit: `ActionDef` as the authoritative registration record consumed by sim dispatch, revalidation, candidate generation, and the decision trace. The new field is static authorial metadata, not runtime state (FND-3: no derived value promoted to truth; FND-26: state-mediated, not behavior-coupled).
4. Not applicable — no failing golden is motivating this ticket. This is infrastructure for S108's design, not a regression fix.
5. Not applicable — not a planner- or golden-driven ticket. No `GoalKind` under test.
6. Not applicable — no AI regression.
7. Not applicable — no ordering claim.
8. Not applicable — no heuristic removal.
9. Not applicable — stale-request/start-failure behavior lands in T-002, not here.
10. Not applicable — no political office claim.
11. Not applicable — no `ControlSource` manipulation.
12. Not applicable — no golden isolation.
13. Construction-site blast radius reassessed: `grep -rn "ActionDef \{"` found 164 sites across 39 files. Of these, the real action registrations are ~47 sites in `crates/worldwake-systems/src/*_actions.rs` and `needs.rs`; the remainder are test fixtures. All must compile after this ticket. This is a required consequence of making the field non-`Option` (per spec D2 compile-time completeness commitment), not a separate bug.
14. No mismatch discovered during reassessment. Spec D1/D2/D3 are aligned with current code after the 2026-04-18 reassess-spec pass.
15. Not applicable — no cumulative arithmetic.

## Architecture Check

1. The classifier is authoritative data declared once on `ActionDef` (FND-26: systems interact through state). Consumers (`resolve_affordance`, `plan_revalidation`, trace) read a single source; no cross-system derivation. The alternative — deriving strictness from `TargetSpec` at each consumption site — was rejected during reassessment because (a) almost no real actions use `TargetSpec::SpecificEntity` at registration, so a derived classifier would leave `ExactIdentity` nearly empty, and (b) different actions with the same `TargetSpec` semantically want different strictness (e.g., `pick_up` fungible vs. `loot` exact).
2. No backward-compatibility shim in the live authority path (FND-28). The serde default exists only at deserialization boundary for old saves; construction sites in code must explicitly name the strictness. Old saves deserialize as `ExactIdentity` (the most conservative, refuses substitution) — their first BestEffort request of a currently-permissive action will fail recoverably until re-saved, which is acceptable.

## Verification Layers

1. Helper predicate correctness (`check_binding_strictness` returns correct `StrictnessGate` for each class × mode pair) -> focused unit tests in `affordance_query.rs`.
2. Serde roundtrip preserves the new field on `ActionDef` -> existing bincode roundtrip test extended in `action_def.rs`.
3. Classification completeness (every registered action has an explicit classification at its registration site) -> compile-time: `ActionDef` is a non-`Option` field with no `Default` impl, so omission fails to compile. This is a structural invariant proven by the compiler, not a runtime test.
4. This ticket is single-layer (authoritative static metadata + pure helper). No action trace, event-log delta, or decision trace surfaces are touched — those layers bind to this metadata in T-002, T-003, and T-004.

## What to Change

### 1. Define `BindingStrictness` enum and serde default function in `action_def.rs`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BindingStrictness {
    ExactIdentity,
    FungibleEquivalentCommodity,
    EquivalentWorkstationTagAtSamePlace,
    EquivalentRouteStep,
    AnyLegalTarget,
}

impl BindingStrictness {
    // Used only by #[serde(default = "…")] on ActionDef::binding_strictness.
    // Construction sites in code must explicitly name the strictness.
    pub(crate) fn exact_identity_default() -> Self {
        Self::ExactIdentity
    }
}
```

Export `BindingStrictness` from `crates/worldwake-sim/src/lib.rs`.

### 2. Add `binding_strictness` field to `ActionDef`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    // ...existing fields...
    pub handler: ActionHandlerId,
    #[serde(default = "BindingStrictness::exact_identity_default")]
    pub binding_strictness: BindingStrictness,
}
```

No `Default` impl on `ActionDef`. The field is non-`Option`; omission at literal construction is a compile error.

### 3. Add `check_binding_strictness` helper in `affordance_query.rs`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrictnessGate {
    SubstitutionAllowed(BindingStrictness),
    ExactIdentityRequired,
}

#[must_use]
pub fn check_binding_strictness(
    def: &ActionDef,
    mode: ActionRequestMode,
) -> StrictnessGate {
    match (mode, def.binding_strictness) {
        (ActionRequestMode::Strict, _) => StrictnessGate::SubstitutionAllowed(def.binding_strictness),
        (ActionRequestMode::BestEffort, BindingStrictness::ExactIdentity) => StrictnessGate::ExactIdentityRequired,
        (ActionRequestMode::BestEffort, class) => StrictnessGate::SubstitutionAllowed(class),
    }
}
```

Export `StrictnessGate` from `lib.rs`. `requested_affordance_matches` is unchanged.

### 4. Classify every registered action per spec D2

Apply exact classifications at each registration site using the spec's D2 table. Key examples:

- `crates/worldwake-systems/src/justice_actions.rs:62` — `accuse` → `ExactIdentity`; also `fine`, `exile` → `ExactIdentity`.
- `crates/worldwake-systems/src/combat.rs:402,460,496,548,582,816,856` — `attack`, `defend`, `loot`, `bury`, `heal`, `queue_for_corpse_use`, `queue_for_care_target` → `ExactIdentity`.
- `crates/worldwake-systems/src/transport_actions.rs:58,94,126,158` — `pick_up`, `put_down`, `drop_item` → `FungibleEquivalentCommodity`; `steal` → `ExactIdentity`.
- `crates/worldwake-systems/src/travel_actions.rs:28` — `travel` → `EquivalentRouteStep`.
- `crates/worldwake-systems/src/patrol_actions.rs:28` — `patrol` → `EquivalentRouteStep`.
- `crates/worldwake-systems/src/needs_actions.rs` — `eat`, `drink` → `FungibleEquivalentCommodity`; `sleep` → `AnyLegalTarget`; `toilet`, `wash` → `EquivalentWorkstationTagAtSamePlace`; `relieve_wilderness` → `AnyLegalTarget`.
- `crates/worldwake-systems/src/facility_queue_actions.rs:35` — `queue_for_facility_use` → `EquivalentWorkstationTagAtSamePlace`.
- `crates/worldwake-systems/src/investigate_actions.rs`, `search_actions.rs` — `investigate`, `search_place` → `AnyLegalTarget`.
- Remaining social/epistemic/office/report/stock/artifact actions listed in spec D2 table: apply the stated classification.
- Recipe-backed harvest/craft actions registered via `production_actions.rs` and `action_registry.rs`: `EquivalentWorkstationTagAtSamePlace`.

### 5. Update all test fixture construction sites

`grep -rn "ActionDef \{"` found 164 sites total. For test fixtures outside `worldwake-systems/src/*_actions.rs`, use `binding_strictness: BindingStrictness::ExactIdentity` as the default unless the test semantically requires a permissive class. Files with material test-fixture `ActionDef` construction include:
- `crates/worldwake-sim/src/action_def.rs` (4 sites in `#[cfg(test)]`)
- `crates/worldwake-sim/src/tick_step.rs` (5 sites)
- `crates/worldwake-sim/src/start_gate.rs` (4 sites)
- `crates/worldwake-sim/src/interrupt_abort.rs` (2 sites)
- `crates/worldwake-sim/src/tick_action.rs` (3 sites)
- `crates/worldwake-sim/src/action_handler_registry.rs` (9 sites)
- `crates/worldwake-sim/src/action_def_registry.rs` (2 sites)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (2 sites)
- `crates/worldwake-ai/src/goal_model.rs` (32 sites)
- `crates/worldwake-ai/src/plan_revalidation.rs` (5 sites)
- `crates/worldwake-ai/src/planning_state.rs` (1 site)
- `crates/worldwake-ai/src/decision_trace.rs` (1 site)
- Remaining sites in `worldwake-systems` test modules and other integration tests.

Every site must be updated in this ticket or the workspace will not compile.

### 6. Extend the bincode roundtrip test

`crates/worldwake-sim/src/action_def.rs` — `sample_action_def` at line 48 and the destructuring match at line 104 must include `binding_strictness`. The roundtrip test at line 143 automatically covers it via `assert_eq!(roundtrip, action_def)`.

### 7. Unit tests for `check_binding_strictness`

Add tests in `crates/worldwake-sim/src/affordance_query.rs` under `#[cfg(test)]` covering:
- Strict mode + every strictness class → `SubstitutionAllowed(class)`.
- BestEffort + `ExactIdentity` → `ExactIdentityRequired`.
- BestEffort + each non-exact class → `SubstitutionAllowed(class)`.

## Files to Touch

- `crates/worldwake-sim/src/action_def.rs` (modify — enum, field, test fixtures, roundtrip)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — helper, unit tests)
- `crates/worldwake-sim/src/lib.rs` (modify — re-exports)
- `crates/worldwake-sim/src/tick_step.rs` (modify — test fixtures)
- `crates/worldwake-sim/src/start_gate.rs` (modify — test fixtures)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify — test fixtures)
- `crates/worldwake-sim/src/tick_action.rs` (modify — test fixtures)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — test fixtures)
- `crates/worldwake-sim/src/action_def_registry.rs` (modify — test fixtures)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — test fixtures)
- `crates/worldwake-systems/src/justice_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/combat.rs` (modify — real classification, 14 sites)
- `crates/worldwake-systems/src/transport_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/travel_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/needs.rs` (modify — real classification, 1 site)
- `crates/worldwake-systems/src/facility_queue_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/search_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/ask_about_person_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/report_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify — real classification, 6 sites)
- `crates/worldwake-systems/src/office_actions.rs` (modify — real classification, 10 sites)
- `crates/worldwake-systems/src/stock_actions.rs` (modify — real classification, 6 sites)
- `crates/worldwake-systems/src/escort_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/bandit_camp_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — real classification)
- `crates/worldwake-systems/src/production_actions.rs` (modify — real classification for recipe-backed defs)
- `crates/worldwake-systems/src/action_registry.rs` (modify — classification for recipe-backed defs)
- `crates/worldwake-systems/src/perception.rs` (modify — test fixtures)
- `crates/worldwake-systems/src/facility_queue.rs` (modify — test fixtures)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test fixtures, 32 sites)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — test fixtures)
- `crates/worldwake-ai/src/planning_state.rs` (modify — test fixtures)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — test fixtures)
- Remaining integration test files under `crates/*/tests/` constructing `ActionDef` literals.

## Out of Scope

- Wiring the gate into `resolve_affordance` or surfacing `ExactIdentityRequired` — lives in S108PERACTBIN-002.
- Plan-revalidation gate — lives in S108PERACTBIN-003.
- Decision trace field — lives in S108PERACTBIN-004.
- Integration/golden tests for end-to-end behavior — lives in S108PERACTBIN-005.
- Removing or consolidating `revalidate_exact_target_step` — spec's Open Migration Work, follow-up spec.

## Acceptance Criteria

### Tests That Must Pass

1. New unit tests in `crates/worldwake-sim/src/affordance_query.rs` covering every `(ActionRequestMode, BindingStrictness)` pair.
2. Extended bincode roundtrip in `crates/worldwake-sim/src/action_def.rs::action_def_roundtrips_through_bincode` preserves `binding_strictness`.
3. Existing suite: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `ActionDef` has no `Default` impl; `binding_strictness` has no `Default` on the field. Omitting the field in a literal construction fails to compile.
2. `BindingStrictness` is authoritative static metadata on `ActionDef`. It is not stored in any ECS component, is not written at runtime, and is not derived from any other field (FND-3, FND-27).
3. Serde default deserializes missing `binding_strictness` as `ExactIdentity`, preserving save/replay compatibility without introducing a live-path shim (FND-28: compat only at representation boundary).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/affordance_query.rs` — new `#[cfg(test)]` tests for `check_binding_strictness` covering every class × mode pair.
2. `crates/worldwake-sim/src/action_def.rs` — extend `sample_action_def` to include `binding_strictness`; the existing `action_def_roundtrips_through_bincode` and `action_def_requires_all_expected_fields_with_concrete_non_optional_semantics` pick up the new field automatically.

### Commands

1. `cargo test -p worldwake-sim action_def::tests::action_def_roundtrips_through_bincode -- --exact`
2. `cargo test -p worldwake-sim affordance_query::tests::strict_mode_allows_every_binding_strictness_class -- --exact`
3. `cargo test -p worldwake-sim affordance_query::tests::best_effort_requires_exact_identity_for_exact_identity_actions -- --exact`
4. `cargo test -p worldwake-sim affordance_query::tests::best_effort_allows_non_exact_binding_strictness_classes -- --exact`
5. `cargo build --workspace`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Added `BindingStrictness` to `crates/worldwake-sim/src/action_def.rs` as required static `ActionDef` metadata with a serde-only `ExactIdentity` default for older save/replay payloads, and re-exported the new types from `crates/worldwake-sim/src/lib.rs`.
- Added `StrictnessGate` plus the pure `check_binding_strictness` helper and focused unit coverage in `crates/worldwake-sim/src/affordance_query.rs`; strict mode admits every class, while best-effort rejects only `ExactIdentity`.
- Classified the live action registrations per S108 D2 and updated all compile-exhaustive `ActionDef` fixtures across `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` to keep the non-optional field honest.

## Deviations

- The draft ticket expected `crates/worldwake-systems/src/action_registry.rs` and `crates/worldwake-systems/src/facility_queue.rs` edits, but the live branch did not require code changes there after compile fallout was enumerated with `cargo test --workspace --no-run`.
- The drafted focused commands were too loose for honest proof and would have run zero tests. Verification used exact fully qualified test IDs resolved from `cargo test -p worldwake-sim -- --list`.
- `cargo fmt --all` produced a formatting-only spill in `crates/worldwake-cli/src/bin/observer.rs`; the post-ticket review treats that file as local handoff fallout rather than owned production scope.

## Verification Result

- Passed `cargo test -p worldwake-sim action_def::tests::action_def_roundtrips_through_bincode -- --exact`
- Passed `cargo test -p worldwake-sim affordance_query::tests::strict_mode_allows_every_binding_strictness_class -- --exact`
- Passed `cargo test -p worldwake-sim affordance_query::tests::best_effort_requires_exact_identity_for_exact_identity_actions -- --exact`
- Passed `cargo test -p worldwake-sim affordance_query::tests::best_effort_allows_non_exact_binding_strictness_classes -- --exact`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
