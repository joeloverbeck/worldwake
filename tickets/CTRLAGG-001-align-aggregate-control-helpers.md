# CTRLAGG-001: Align aggregate control helpers with lawful control semantics

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — authoritative control aggregation in `worldwake-core`, constraint/runtime inventory reads in `worldwake-sim`, affected action/system regressions
**Deps**: [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), [archive/tickets/completed/S01PROOUTOWNCLA-003-extend-can-exercise-control.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S01PROOUTOWNCLA-003-extend-can-exercise-control.md), [archive/tickets/HARCARGOACON-002-destination-aware-read-helpers.md](/home/joeloverbeck/projects/worldwake/archive/tickets/HARCARGOACON-002-destination-aware-read-helpers.md), [archive/tickets/AITRACE-002-planner-runtime-legality-provenance.md](/home/joeloverbeck/projects/worldwake/archive/tickets/AITRACE-002-planner-runtime-legality-provenance.md)

## Problem

The authoritative control contract is currently split in two incompatible ways:

- `can_exercise_control(actor, entity)` grants lawful control over unpossessed directly owned, faction-owned, and office-owned entities, plus contents of controlled containers
- `controlled_commodity_quantity(holder, kind)` and `controlled_unique_item_count(holder, kind)` still count only along the custody graph rooted at the holder's possessions

That means aggregate control reads can disagree with per-entity legality. A holder may lawfully control an unpossessed owned ground lot, yet aggregate quantity and unique-item checks can still report zero. This violates `FOUNDATIONS.md` Principles 3, 22, 24, and 25 because the derived summary is no longer a faithful cache of the concrete lawful state.

The mismatch is no longer theoretical. It surfaced during AITRACE-002 while tracing justice fine selection/start: place-local controlled quantity already used the lawful control predicate, while global aggregate quantity still used the narrower custody-only traversal. The architecture should not keep two different meanings of “controlled stock.”

## Assumption Reassessment (2026-03-27)

1. The live authoritative legality predicate is [`World::can_exercise_control()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs), not possession alone. Current focused coverage in [`crates/worldwake-core/src/world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) proves direct ownership, faction delegation, office delegation, container propagation, and possession-by-other blocking:
   - `can_exercise_control_enforces_possession_then_unpossessed_ownership`
   - `can_exercise_control_faction_member_on_faction_owned_unpossessed`
   - `can_exercise_control_office_holder_on_office_owned_unpossessed`
   - `can_exercise_control_flows_through_controlled_containers`
2. The aggregate helpers in [`crates/worldwake-core/src/world/ownership.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs) do not share that same substrate today:
   - `controlled_commodity_quantity(holder, kind)` walks `possessions_of()` and `direct_contents_of()` from the holder root
   - `controlled_unique_item_count(holder, kind)` uses the same custody-root traversal
   - `controlled_commodity_quantity_at_place(holder, place, kind)` instead enumerates lots at the place and filters them by `can_exercise_control(holder, entity)`
3. Shared abstraction boundary under audit: authoritative aggregate control reads in `worldwake-core` versus the per-entity lawful-control predicate that authoritative validation and place-local inventory reads already use.
4. Intended invariant: every aggregate helper whose name claims “controlled” must be a faithful derived summary of the same lawful control contract that `can_exercise_control()` enforces. Aggregate reads may be cached or factored differently, but they may not mean “possessed only” while per-entity legality means “lawfully controlled.”
5. This is not a planner-goal ticket. No live `GoalKind` needs to be changed. The relevant shared surfaces are lower-level:
   - authoritative action constraints in [`crates/worldwake-sim/src/action_validation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_validation.rs) via `Constraint::ActorHasCommodity` and `Constraint::ActorHasUniqueItemKind`
   - self-belief inventory reads in [`crates/worldwake-sim/src/per_agent_belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) via `commodity_quantity()` and `unique_item_count()`
   - domain start validation in [`crates/worldwake-systems/src/justice_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs), [`crates/worldwake-systems/src/trade_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs), and [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs), all of which rely on aggregate controlled quantity
6. Current focused coverage is asymmetric:
   - there is coverage proving custody-root counting in `possessions_and_controlled_commodity_queries_follow_custody_graph`
   - there is coverage proving delegated lawful control for unpossessed entities
   - there is no focused core test proving aggregate controlled quantity or unique-item count includes lawful unpossessed directly owned, faction-owned, or office-owned stock
   - there is no focused runtime test proving `Constraint::ActorHasCommodity` / `ActorHasUniqueItemKind` align with `can_exercise_control()` for unpossessed lawful stock
7. The repo already established a cleaner local-stock contract in [archive/tickets/HARCARGOACON-002-destination-aware-read-helpers.md](/home/joeloverbeck/projects/worldwake/archive/tickets/HARCARGOACON-002-destination-aware-read-helpers.md): place-local controlled quantity is derived from concrete per-lot controllability, not possession-only traversal. This ticket should extend that same semantic discipline to the global aggregate helpers.
8. This is a mixed-layer contract ticket. The first visible divergences occur at authoritative constraint/start boundaries, but the root contradiction lives one layer lower in the core aggregate-control helpers. The fix belongs at that shared source, not in justice/trade-specific workarounds.
9. First failure boundary under current code depends on caller:
   - `Constraint::ActorHasCommodity` / `ActorHasUniqueItemKind` can reject a lawful action before start in `action_validation`
   - `ensure_accessible_quantity()` in justice/trade/office action handlers can abort at authoritative start even when per-lot control would allow local resolution
   - self-belief `commodity_quantity()` / `unique_item_count()` for the acting agent can underreport controlled stock before AI emits or ranks downstream actions
10. `controlled_commodity_quantity_at_place()` is already closer to the intended architecture than `controlled_commodity_quantity()`. The clean change is not to narrow the place-local helper back down to possession-only semantics, but to promote aggregate helpers onto the same lawful-control substrate.
11. Adjacent contradiction classification:
   - required consequence in scope: aggregate “controlled” summaries must align with lawful control semantics
   - required consequence in scope: authoritative constraint and self-belief reads that depend on those summaries must inherit the corrected behavior
   - future cleanup, not required here: if performance later needs a cached control index, that should become a separate ticket after the contract is corrected in one place
12. Mismatch + correction: older cargo and production work could safely use `controlled_commodity_quantity()` as “controlled stock absent/present” when most relevant stock was possessed. The live architecture now materially depends on unpossessed owned and institutionally delegated ground stock. This ticket therefore corrects the aggregate helper contract itself rather than patching each downstream caller.
13. The concrete arithmetic under audit is simple but must stay exact: aggregate quantity must equal the sum of all live item lots of the requested commodity for which `can_exercise_control(holder, entity)` succeeds, regardless of whether those lots are directly possessed, inside controlled containers, or unpossessed on the ground. Unique-item count must satisfy the same rule over live unique items.

## Architecture Check

1. The clean architecture is one canonical authoritative control substrate, with aggregate helpers derived from it. That is cleaner than leaving per-entity legality and aggregate summaries to drift independently, and cleaner than adding domain-specific “accessible quantity” exceptions in justice, trade, office, or AI code.
2. This is cleaner than narrowing `can_exercise_control()` to match custody-only counting. The delegated ownership model from S01 is intentional and foundational to ownership, theft, and institution-controlled assets.
3. No backwards-compatibility aliasing or alternate “aggregate accessible quantity” API should be introduced. The existing `controlled_*` helpers should be corrected in place to mean one thing everywhere.

## Verification Layers

1. Aggregate commodity and unique-item reads include lawful unpossessed/delegated stock and still exclude possessed-by-other stock -> focused `worldwake-core` unit tests over `World`
2. Aggregate helpers remain faithful derived summaries of per-entity legality -> focused `worldwake-core` tests comparing aggregate results against concrete `query_item_lot` / `query_unique_item` filtered by `can_exercise_control()`
3. Authoritative constraint checks inherit the corrected aggregate semantics -> focused `worldwake-sim` validation tests for `Constraint::ActorHasCommodity` and `Constraint::ActorHasUniqueItemKind`
4. Domain actions that use aggregate controlled quantity do not retain divergent fallback semantics -> focused runtime/system tests at the earliest affected start boundary, preferably justice or office/trade start validation rather than later durable state proxies
5. If traces are needed later for cross-layer explanation, the strongest proof surface for this ticket remains lower-layer aggregate-control and validation tests; a new traceability ticket would be separate

## What to Change

### 1. Introduce one canonical aggregate-control substrate in `worldwake-core`

Refactor aggregate controlled-inventory helpers so they derive from the same lawful-control contract as `can_exercise_control()`.

Recommended shape:

- add an internal deterministic enumerator or fold helper in [`crates/worldwake-core/src/world/ownership.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs) that iterates live candidate entities of the relevant kind and retains only those the holder can currently control
- use that substrate to implement:
  - `controlled_commodity_quantity(holder, kind)`
  - `controlled_commodity_quantity_at_place(holder, place, kind)`
  - `controlled_unique_item_count(holder, kind)`

The important architectural rule is shared semantics, not a specific helper name. Do not keep custody-root traversal in one helper while place-local reads use `can_exercise_control()`.

### 2. Keep control semantics concrete and deterministic

The corrected aggregate helpers must:

- include unpossessed directly owned stock
- include unpossessed faction-owned stock for members
- include unpossessed office-owned stock for current office holders
- include contents of controlled containers
- exclude stock currently possessed by someone else
- ignore archived or missing entities
- preserve deterministic ordering where enumeration is observable in tests

### 3. Re-verify downstream authoritative readers at the earliest failure boundary

Update focused runtime coverage so downstream readers depending on aggregate helpers are proven against the corrected contract:

- `Constraint::ActorHasCommodity`
- `Constraint::ActorHasUniqueItemKind`
- one affected start-validation path such as justice fine start or office/trade commodity transfer start

The goal is not broad golden churn. The goal is to prove that the shared runtime contract now matches the corrected core semantics.

## Files to Touch

- `crates/worldwake-core/src/world/ownership.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — focused tests)
- `crates/worldwake-sim/src/action_validation.rs` (modify — focused tests, and code only if needed)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify only if a helper assumption or focused test needs correction)
- `crates/worldwake-systems/src/justice_actions.rs` (modify tests; code only if reassessment finds a remaining domain-specific mismatch after the core fix)
- `crates/worldwake-systems/src/office_actions.rs` (modify tests if chosen proof surface uses office transfer start)
- `crates/worldwake-systems/src/trade_actions.rs` (modify tests if chosen proof surface uses trade transfer start)

## Out of Scope

- Changing the semantics of `can_exercise_control()`
- Adding new AI trace payloads
- Rewriting cargo or enterprise goal logic
- Introducing a parallel “accessible quantity” or “lawfully controlled quantity” public API just to preserve the old aggregate behavior
- Performance caching/indexing work unless a minimal internal factorization is needed for code clarity

## Acceptance Criteria

### Tests That Must Pass

1. Focused `worldwake-core` tests prove aggregate controlled quantity and unique-item count include lawful unpossessed directly owned, faction-owned, and office-owned stock
2. Focused `worldwake-core` tests prove aggregate controlled quantity still excludes stock possessed by another actor even when ownership or delegation would otherwise allow control
3. Focused `worldwake-sim` validation tests prove `Constraint::ActorHasCommodity` and `Constraint::ActorHasUniqueItemKind` align with the corrected aggregate-control contract
4. One focused runtime/system regression proves an affected start boundary no longer diverges from per-lot lawful control semantics
5. Existing suite: `cargo test --workspace`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. Every authoritative helper named `controlled_*` must mean lawful current control, not merely custody-root reachability
2. Aggregate controlled summaries must stay derived from concrete live entities and the same control predicate used by per-entity legality, never from a separate hidden rule set

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-core/src/world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) — add focused aggregate-control tests for direct ownership, faction delegation, office delegation, possessed-by-other blocking, and unique-item counting; this is the strongest proof surface for the contract repair
2. [`crates/worldwake-sim/src/action_validation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_validation.rs) — add focused constraint tests proving `ActorHasCommodity` and `ActorHasUniqueItemKind` now follow lawful control rather than custody-only stock
3. One of [`crates/worldwake-systems/src/justice_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs), [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs), or [`crates/worldwake-systems/src/trade_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) — add a focused start-boundary regression proving the corrected core contract propagates to a real action path

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo test --workspace`
5. `cargo clippy --workspace`
