# S125INSTREBOU-002: OfficeDef treasury authoring + scenario lints

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — scenario authoring + spawn path + new lint rules
**Deps**: [S125INSTREBOU-001](S125INSTREBOU-001.md)

## Problem

Today no scenario can author office-owned funds without using loose item lots co-located with unrelated scenes. The S125 proof case is exactly this: adding coin items to `Market Square` perturbs the theft-scene perception (S125 Evidence #6). S125 Deliverable D2 specifies an `OfficeDef.treasury` field that materializes a treasury container and seeds office-owned lots inside it, scoping the funds out of place-floor perception. This ticket lands that authoring surface and the lint rules that protect it.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `OfficeDef` lives at `crates/worldwake-cli/src/scenario/types.rs:104`. Construction sites: 3 total — 1 type definition + 2 in `crates/worldwake-cli/src/scenario/mod.rs:2819` and `mod.rs:3357` (both inside test/scenario fixtures). `spawn_office` exists in `crates/worldwake-cli/src/scenario/mod.rs`. No existing test for treasury authoring (the field does not yet exist). The 3 construction sites are well under the 50-site spread-syntax threshold; whether `..Default::default()` is currently used at the two call sites must be confirmed during implementation, but the count is informational.
2. S125 §4 (Scenario Authoring) specifies extending `OfficeDef` with `treasury: Option<TreasuryDef>` rather than reusing `ItemDef.location` resolution — the latter is intentionally not extended (S125 Out-of-Scope). Lints required: missing-seat / zero-quantity. Existing lint surface: `crates/worldwake-cli/src/scenario/lints.rs` carries 3 existing rules (`ProfileHomogeneity`, `UnreachableExplorationDrive`, `AuthoritativeHelperOnSnapshot`) that demonstrate the registration pattern.
3. Shared abstraction boundary: scenario `*Def` types + `spawn_*` materialization. `TreasuryDef` follows the existing `*Def` wrapper pattern.
4. Adjacent contradictions: none. The 2 existing `OfficeDef` construction sites each need an explicit `treasury: None` entry (or pick up `Default::default()` if `OfficeDef` derives `Default`; verify during impl).

## Architecture Check

1. Treasury authored as a structured field on `OfficeDef` rather than through global `items:` placement — this keeps office funds visually grouped with the office, avoids accidental loose-item authoring at the wrong place, and makes lint enforcement straightforward (FND-23 institutions are world state with their own assets; FND-3 concrete authored items, not abstract balances).
2. No backward compatibility: there is no prior treasury-authoring path to alias.

## Verification Layers

1. Scenario load → focused test exercising `spawn_office` with `TreasuryDef` and asserting container existence + `OwnedBy(container, office)` + lots inside the container with `OwnedBy(lot, office)`.
2. Lint coverage → focused tests for missing-seat / zero-quantity invalid cases.
3. Single-layer ticket — scenario CLI is not exercised through traces; runtime/golden coverage belongs to ticket 007.

## What to Change

### 1. `TreasuryDef`

Add a new struct in `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct TreasuryDef {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub container_name: Option<String>,
}
```

Default container name (when `None`) is `"<office_name> Treasury"`.

### 2. `OfficeDef.treasury` field

Add `pub treasury: Option<TreasuryDef>` to `OfficeDef`. Update the 2 existing construction sites at `mod.rs:2819` and `mod.rs:3357` (`treasury: None`).

### 3. `spawn_office` extension

When `treasury.is_some()`:
- Spawn a `Container` entity at the office's seat place.
- Set `OwnedBy(container, office)`.
- Spawn an item lot inside the container with the specified `commodity` / `quantity`, set `OwnedBy(lot, office)` so existing `controlled_item_lots_for(office)` enumerates it.

### 4. Lint rules

Add two new variants to `LintRule` (or equivalent enum) in `crates/worldwake-cli/src/scenario/lints.rs`:
- `TreasuryAuthoredWithMissingSeat` — fires when a treasury is authored but the office's seat name does not resolve to a place.
- `TreasuryAuthoredWithZeroQuantity` — fires when `treasury.quantity == Quantity(0)`.

Honor the existing `scenario_lint_overrides` suppression mechanism.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — `TreasuryDef` + `OfficeDef.treasury`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — `spawn_office` extension + 2 construction-site updates)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — new lint variants)

## Out of Scope

- Modifying `scenarios/survival-justice.ron` itself — deferred to ticket 007 where the golden lands.
- Allowing `ItemDef.location` to resolve to office/container names — S125 Out-of-Scope by design.
- Faction treasuries — S125 Non-Goal.
- Container-content perception scoping — see ticket 007 Assumption #6 for the boundary; if engine-level perception scoping turns out to be incomplete, it surfaces during ticket 007 and gets its own ticket per the 1-3-1 rule.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `spawn_office_with_treasury_creates_owned_container_and_lots`.
2. New focused test: `lint_rejects_treasury_with_zero_quantity`.
3. New focused test: `lint_rejects_treasury_when_office_seat_missing`.
4. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. After spawn, `controlled_item_lots_for(office)` enumerates the treasury's lots.
2. The treasury's lots are inside a `Container` entity owned by the office, not at the seat's place-floor.
3. Lint failures abort scenario load unless explicitly suppressed via `scenario_lint_overrides`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` (existing `#[cfg(test)]` block) — `spawn_office_with_treasury_creates_owned_container_and_lots`.
2. `crates/worldwake-cli/src/scenario/lints.rs` (existing `#[cfg(test)]` block) — `lint_rejects_treasury_with_zero_quantity` + `lint_rejects_treasury_when_office_seat_missing`.

### Commands

1. `cargo test -p worldwake-cli`
2. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added `TreasuryDef` and `OfficeDef.treasury` to the scenario authoring schema.
- Extended `spawn_office` to materialize an office-owned treasury container at the office seat, name it with either the authored `container_name` or `"<office_name> Treasury"`, put the authored lot inside it, and set `OwnedBy` on both the container and lot to the office.
- Added treasury lint rules for missing office-seat place references and zero treasury quantities, with the existing `scenario_lint_overrides` suppression path covering the new rules.
- Updated the two live manual `OfficeDef` literals to include `treasury: None`.

## Deviations

- The treasury container receives a `Name` component for lookup/display, but it is not inserted into the scenario name-resolution map. This keeps `ItemDef.location` and other scenario reference fields from gaining a new office/container placement alias, preserving the ticket's out-of-scope boundary.

## Verification Result

- Passed `cargo test -p worldwake-cli --lib -- --list`.
- Passed `cargo test -p worldwake-cli --lib scenario::tests::spawn_office_with_treasury_creates_owned_container_and_lots -- --exact`.
- Passed `cargo test -p worldwake-cli --lib scenario::lints::tests::lint_rejects_treasury_with_zero_quantity -- --exact`.
- Passed `cargo test -p worldwake-cli --lib scenario::lints::tests::lint_rejects_treasury_when_office_seat_missing -- --exact`.
- Passed `cargo test -p worldwake-cli --lib scenario::lints::tests::treasury_lint_override_suppresses_failure -- --exact`.
- Passed `cargo test -p worldwake-cli`.
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`.
- Passed `git diff --check`.
- Passed `./scripts/verify.sh` (live script gates included `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).
