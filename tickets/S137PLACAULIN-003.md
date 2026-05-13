# S137PLACAULIN-003: RepairKind variant migration and RepairAppliedPayload widening

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `RepairKind` enum, `RepairAppliedPayload`, `classify_accepted_repair` mapping, save-load
**Deps**: specs/S137-plan-causal-links-and-repair.md (D8)

## Problem

S137 D8 migrates `RepairKind` from 4 post-hoc-classification variants (`AlternateTarget`, `AlternateRoute`, `AlternateMerchant`, `AlternateRecipe`) to 5 search-axis variants (`RebindTarget`, `ReplaceProvider`, `InsertVerification`, `DowngradeToProgressBarrier`, `Abandon`) plus the addition of `RepairAppliedPayload.substitute_recipe: Option<RecipeId>` so the subsumed `AlternateRecipe` case preserves its discriminative information. The migration must land atomically per FND-28 — splitting across tickets would leave the workspace in a half-renamed state with 20 cross-crate call sites simultaneously referring to old and new variant names.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RepairKind` is defined at `crates/worldwake-core/src/decision_event_payload.rs:418-423` with 4 variants. `RepairAppliedPayload` at lines 409-415 carries `repair_kind: RepairKind` and `substitute_target: Option<EntityId>` but no `substitute_recipe`. Existing tests covering the legacy 4-variant emission live in `crates/worldwake-ai/src/agent_tick/planning.rs` `#[cfg(test)]` (boundary at line 2571): `classify_accepted_repair_prefers_alternate_merchant_over_anchor_change:3354`, `classify_accepted_repair_detects_alternate_recipe_for_same_output:3397`, `classify_accepted_repair_detects_alternate_route_for_same_anchor:3461`. Additional construction sites in `agent_tick/tests.rs` lines 8882, 8926, 8946, 8962, 8977, 8993, 9006, 9022 (all in `#[cfg(test)]`).
2. Spec `specs/S137-plan-causal-links-and-repair.md` D8 enumerates all 20 call sites with the variant mapping table and the `substitute_recipe` field addition. `SAVE_FORMAT_VERSION` is `80` after S137PLACAULIN-002 — bump to `81` required because the variant rename changes serialized representation (no `#[serde(rename)]` shim, per FND-28).
3. Shared boundary: the `RepairKind` enum surface across crates. Per `references/worldwake-validation-patterns.md` Existing Variant Payload Widening, the payload field addition + enum rename must land atomically across all 20 sites; intermediate workspace states fail to compile because pattern-match arms reference renamed variants.
4. **Equality-check semantics shift at `agent_tick/mod.rs:2375`**: the line currently reads `if accepted_repair.repair_kind == RepairKind::AlternateTarget`. After migration, `RebindTarget` covers anchor-change + merchant-change + recipe-change cases. The check's original semantic (anchor change specifically) must be preserved by inspecting `substitute_target.is_some()` together with absence of `substitute_recipe`. Implementer reassessment must read the surrounding code to determine the correct disambiguator predicate.
5. **Adjacent contradictions**: the post-hoc classifier `classify_accepted_repair` (planning.rs:1452-1526) currently emits one of the four legacy variants based on which axis differs between failed and selected plans. After migration: AlternateTarget/Merchant/Recipe branches all emit `RebindTarget` with appropriate `substitute_target`/`substitute_recipe`; AlternateRoute branch emits `ReplaceProvider`. The 3 new variants (`InsertVerification`, `DowngradeToProgressBarrier`, `Abandon`) have no emission sites in this ticket — they are forward declarations awaiting ticket 006's `plan_repair` module. Forward-declared variants are not "dead code" because they have a declared semantic and a near-term emission site; classified as required consequence per Divergence Protocol.

## Architecture Check

1. **FND-28-compliant atomic migration**: all 20 sites migrate in one ticket; no `#[serde(rename)]` shim, no deprecated alias. The 3 forward-declared variants (`InsertVerification`, `DowngradeToProgressBarrier`, `Abandon`) are not dead code — they are declared with intended semantics and emitted by ticket 006.
2. **Save-format bump preserves single-truth invariant**: existing save fixtures with legacy variant names cannot deserialize after this ticket — the bump signals the format change explicitly per CLAUDE.md determinism invariant.

## Verification Layers

1. Enum variant set + payload shape → focused unit tests (`RepairKind` variant count, `RepairAppliedPayload` field bincode roundtrip).
2. `classify_accepted_repair` mapping → existing planning.rs tests (updated to assert new variants).
3. Save-load version handling → focused unit test in `save_load.rs` asserting `SAVE_FORMAT_VERSION == 81` and that legacy fixtures fail with the expected `UnsupportedVersion` error rather than silently misdeserializing.
4. Equality-check semantic preservation at `agent_tick/mod.rs:2375` → focused runtime coverage in `agent_tick/tests.rs` asserting the original anchor-change-specific predicate still holds when only anchor differs vs. when merchant/recipe also differ.

## What to Change

### 1. `RepairKind` enum migration

In `crates/worldwake-core/src/decision_event_payload.rs:418-423`, replace:

```rust
pub enum RepairKind {
    AlternateTarget,
    AlternateRoute,
    AlternateMerchant,
    AlternateRecipe,
}
```

with:

```rust
pub enum RepairKind {
    RebindTarget,              // subsumes legacy AlternateTarget, AlternateMerchant, AlternateRecipe
    ReplaceProvider,           // subsumes legacy AlternateRoute
    InsertVerification,        // forward-declared; emitted by ticket 006
    DowngradeToProgressBarrier, // forward-declared; emitted by ticket 006
    Abandon,                   // forward-declared; emitted by ticket 006
}
```

Preserve existing derives: `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

### 2. `RepairAppliedPayload` widening

In `decision_event_payload.rs:409-415`, add:

```rust
pub struct RepairAppliedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub repair_kind: RepairKind,
    pub substitute_target: Option<EntityId>,
    #[serde(default)]
    pub substitute_recipe: Option<RecipeId>,
}
```

`#[serde(default)]` keeps omitted-field serde payloads lawful; save files still use the bumped current-format version and older versions remain rejected.

### 3. `classify_accepted_repair` mapping update

In `crates/worldwake-ai/src/agent_tick/planning.rs:1452-1526`, update the four return sites:

- Line 1481 (recipe branch): emit `RepairKind::RebindTarget` with `substitute_target: None`, `substitute_recipe: Some(<selected_recipe_id>)`.
- Line 1495 (counterparty branch): emit `RepairKind::RebindTarget` with `substitute_target: Some(selected_counterparty)`, `substitute_recipe: None`.
- Line 1505 (anchor branch): emit `RepairKind::RebindTarget` with `substitute_target: Some(<anchor_entity>)`, `substitute_recipe: None`.
- Line 1520 (route branch): emit `RepairKind::ReplaceProvider` with `substitute_target: None`, `substitute_recipe: None`.

### 4. Equality-check disambiguation at `agent_tick/mod.rs:2375`

Read the surrounding code to determine the predicate's original intent (likely anchor-change-specific behavior). Replace `repair_kind == RepairKind::AlternateTarget` with the corrected predicate. Most likely: `repair_kind == RepairKind::RebindTarget && substitute_target.is_some() && substitute_recipe.is_none()`. Reassessment must read the call site's surrounding flow to confirm.

### 5. SAVE_FORMAT_VERSION bump

In `crates/worldwake-sim/src/save_load.rs:6`, bump `SAVE_FORMAT_VERSION` from `80` to `81`. Update the load-current-format match at line 129 to reference `81`.

### 6. Test updates

Update the 3 existing `classify_accepted_repair_*` tests at planning.rs:3354, 3397, 3461 to assert the new variant set. Update the 8 sites in `agent_tick/tests.rs` to construct with the new variants and add `substitute_recipe: None` (or `Some(<id>)` where appropriate). Update the test at `decision_event_payload.rs:705` constructing `RepairAppliedPayload` with the legacy `AlternateTarget`.

### 7. Cross-crate site updates

- `crates/worldwake-ai/src/decision_runtime.rs:653` — replace `AlternateMerchant` with `RebindTarget` + populate `substitute_recipe: None`.
- `crates/worldwake-sim/src/save_load.rs:1157` — replace `AlternateMerchant` with `RebindTarget` + populate `substitute_recipe: None`.
- `crates/worldwake-cli/src/bin/observer.rs:5477` — replace `AlternateMerchant` with `RebindTarget` + populate `substitute_recipe: None`.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — enum + payload + tests at 705)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump + site at 1157)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — classify_accepted_repair at 1452-1526; tests at 3354, 3397, 3461)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — equality check at 2375)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — site at 653)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — 8 sites at 8882-9022)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — site at 5477)

## Out of Scope

- Emission sites for `InsertVerification`, `DowngradeToProgressBarrier`, `Abandon` — ticket 006.
- `RepairMemory.repairs` shape migration to key by `BreachSignature` — ticket 005.
- New decision-trace surface `RepairAttemptTrace` consuming the variant set — ticket 008.
- Observer rendering of the new `substitute_recipe` field — ticket 009.
- `DiscrepancyClearing` variant extension — subsumed into ticket 006.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai classify_accepted_repair` — all three classify tests pass with updated variant assertions.
2. `cargo test -p worldwake-ai agent_tick::tests` — 8 updated construction sites pass.
3. `cargo test -p worldwake-sim save_load` — version bump test passes; legacy fixture rejection test passes.
4. `cargo test --workspace` — workspace builds cleanly after atomic migration.
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Exactly 20 call sites migrated; no legacy variant names (`AlternateTarget`, `AlternateMerchant`, `AlternateRecipe`, `AlternateRoute`) remain anywhere in `crates/` after this ticket.
2. `SAVE_FORMAT_VERSION` is `81`; pre-`81` byte streams fail with `UnsupportedVersion`.
3. `classify_accepted_repair` emits `RebindTarget` for legacy target/merchant/recipe inputs and `ReplaceProvider` for route inputs.
4. `RepairAppliedPayload.substitute_recipe` carries `Some(_)` exactly when the legacy `AlternateRecipe` branch fires.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — modify `classify_accepted_repair_prefers_alternate_merchant_over_anchor_change`, `classify_accepted_repair_detects_alternate_recipe_for_same_output`, `classify_accepted_repair_detects_alternate_route_for_same_anchor` for new variants.
2. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — new test `save_format_version_is_81_after_repair_kind_migration` asserting the constant value and that a synthetic pre-`81` payload rejects with `UnsupportedVersion`.
3. `crates/worldwake-core/src/decision_event_payload.rs` `#[cfg(test)]` — new test `repair_applied_payload_substitute_recipe_roundtrips_through_bincode`.

### Commands

1. `cargo test -p worldwake-ai classify_accepted_repair`
2. `cargo test -p worldwake-sim save_load`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

Merge note: Ticket 002 bumps `SAVE_FORMAT_VERSION 79→80`; ticket 003 bumps `80→81`; ticket 004 is expected to bump the next current-format value for `PlanGuard.causal_links`; ticket 005 cascades after that (see Step 6 Merge-Order Constraints).
