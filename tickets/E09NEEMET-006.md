# E09NEEMET-006: Deprivation consequences — wounds, forced collapse, involuntary relief

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends metabolism system with consequence logic
**Deps**: E09NEEMET-005 (metabolism system must exist), E09NEEMET-001 (WoundList must exist)

## Problem

Critical unmet needs must create concrete downstream effects. Without consequences, the survival loop has no teeth and agents can remain at max deprivation indefinitely without meaningful impact. This ticket implements the four deprivation consequences: starvation wounds, dehydration wounds, fatigue collapse, and bladder accidents.

## Assumption Reassessment (2026-03-10)

1. `DeprivationExposure` tracks ticks at critical level — confirmed it will exist after E09NEEMET-003.
2. `MetabolismProfile` contains `starvation_tolerance_ticks`, `dehydration_tolerance_ticks`, `exhaustion_collapse_ticks`, `bladder_accident_tolerance_ticks` as `NonZeroU32` — confirmed in spec.
3. `WoundList` and `WoundCause::Deprivation(Starvation|Dehydration)` will exist after E09NEEMET-001.
4. Forced collapse means the metabolism system must be able to request an action state change (cancel current action, start forced sleep). This interacts with the scheduler / action framework.
5. Involuntary relief creates a `Waste` item lot at the agent's location — `CommodityKind::Waste` exists.

## Architecture Check

1. Consequence logic runs as part of the needs system tick (step 5 in the spec's per-tick list), after deprivation counters are updated.
2. Forced collapse emits an `InputEvent` or equivalent signal rather than directly manipulating action state — maintaining system decoupling (Principle 12).
3. Waste creation uses the existing item creation patterns from E04.
4. Each consequence emits an event to the `EventLog` for causal tracing.

## What to Change

### 1. Extend `needs_system` in `crates/worldwake-systems/src/needs.rs`

After deprivation counter updates (from E09NEEMET-005), add step 5:

**Starvation consequence**:
- If `hunger_critical_ticks >= metabolism.starvation_tolerance_ticks.get()`:
  - Add `Wound { cause: WoundCause::Deprivation(Starvation), .. }` to agent's `WoundList`
  - Reset `hunger_critical_ticks` to 0 (wound was just inflicted; next wound requires another full tolerance period)
  - Emit event with `EventTag` for deprivation harm

**Dehydration consequence**:
- Same pattern as starvation but with thirst/dehydration thresholds and `DeprivationKind::Dehydration`

**Fatigue collapse**:
- If `fatigue_critical_ticks >= metabolism.exhaustion_collapse_ticks.get()`:
  - Signal forced sleep / collapse (mechanism: write a collapse marker component or enqueue an input event)
  - The collapse interrupts any current action
  - Reset `fatigue_critical_ticks` to 0

**Bladder accident**:
- If `bladder_critical_ticks >= metabolism.bladder_accident_tolerance_ticks.get()`:
  - Set `bladder` to `Permille(0)` (relief occurred)
  - Increase `dirtiness` by a fixed amount (e.g., `Permille(200)`)
  - Create `CommodityKind::Waste` item lot at agent's current location
  - Reset `bladder_critical_ticks` to 0
  - Emit event

### 2. Collapse marker or signal mechanism

Define a lightweight way for the needs system to signal "this agent must collapse." Options:
- A) A `CollapseRequest` component written to the agent, consumed by the scheduler next tick
- B) Enqueue an `InputEvent::SystemOverride` variant

Recommend A — a transient component is simpler and follows state-mediated coupling (Principle 12).

### 3. Event emissions

Each consequence must emit an `EventRecord` to the `EventLog` with appropriate `EventTag` and `CauseRef`.

## Files to Touch

- `crates/worldwake-systems/src/needs.rs` (modify — add consequence logic)
- `crates/worldwake-core/src/needs.rs` or `components.rs` (modify — add `CollapseRequest` component if approach A)
- `crates/worldwake-core/src/component_schema.rs` (modify — register `CollapseRequest` if approach A)
- `crates/worldwake-core/src/event_tag.rs` (modify — add deprivation-related event tags if needed)

## Out of Scope

- How the scheduler handles `CollapseRequest` (that's E08 scheduler extension or E09NEEMET-005 follow-up)
- Wound healing / progression (E12)
- Pain derivation from wounds (E13)
- AI awareness of deprivation (E13)
- Disease from dirtiness (future phase)

## Acceptance Criteria

### Tests That Must Pass

1. Agent at critical hunger for `starvation_tolerance_ticks` receives a wound with `WoundCause::Deprivation(Starvation)`.
2. Agent at critical thirst for `dehydration_tolerance_ticks` receives a wound with `WoundCause::Deprivation(Dehydration)`.
3. Agent at critical fatigue for `exhaustion_collapse_ticks` triggers collapse signal.
4. Agent at critical bladder for `bladder_accident_tolerance_ticks` has bladder reset to 0, dirtiness increased, and waste item created at location.
5. Deprivation counters reset after consequence fires — a second wound requires another full tolerance period.
6. Each consequence emits an event to the event log.
7. Non-critical agents receive no consequences regardless of tick count.
8. Existing suite: `cargo test --workspace`

### Invariants

1. Deprivation wounds use the shared `WoundList` / `WoundCause` types — same carrier as combat wounds.
2. Consequences propagate through shared state only (Principle 12).
3. Conservation: waste item created by bladder accident is a real entity at a real location.
4. All `Permille` values stay in valid range after consequence application.
5. No stored fear or wellness scores created by this system.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` (tests) — starvation wound, dehydration wound, collapse trigger, bladder accident, counter reset, event emission

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
