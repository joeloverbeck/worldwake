# S127QUAAWAACQ-006: Harvest action partial-success path + CommitTraceData.partial_quantity + payload requested_quantity

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — modifies `commit_harvest` to compute `actual = min(available, requested)`, renames `HarvestActionPayload.output_quantity` to `requested_quantity`, adds `partial_quantity: Option<Quantity>` field to `CommitTraceData` (touches all action-commit handler ctors), appends to `LastHarvestTrace` on commit, surfaces `partial_quantity` in the action-commit trace, bumps `SAVE_FORMAT_VERSION`
**Deps**: S127QUAAWAACQ-003, S127QUAAWAACQ-004, S127QUAAWAACQ-005

## Problem

S127 turns "the source ran dry mid-harvest" into a partial-success outcome rather than a hard failure (D7). When `available_quantity < requested_quantity` at commit time but `>= 1`, the action commits with `actual = min(available, requested)` units, drains the source, appends a `HarvestTraceEntry { partial: true }`, and surfaces `partial_quantity` through `CommitTraceData` so the AI tick step records the actual inventory delta and re-evaluates `is_satisfied` against `desired_min`. Per spec Question 2 (option b), partial-quantity rides through the existing `CommitTraceData` surface — `CommitOutcome` shape stays stable so we don't introduce a foundational-type shim across all action handlers (FND-28). This ticket also lands D11 part b — the action-commit trace formatter emits `quantity_actual / quantity_requested` for harvest commits — and bumps `SAVE_FORMAT_VERSION`.

## Assumption Reassessment (2026-04-26)

1. `crates/worldwake-systems/src/production_actions.rs:553-617` defines `commit_harvest` (confirmed during reassessment). Current path: `source.available_quantity.checked_sub(payload.output_quantity)` at line 589-597 fails with `ActionError::PreconditionFailed("resource source {workstation} lacks {:?} units for harvest")` when `available < requested`. New path: compute `actual = min(available, requested)`, succeed if `actual >= 1`, fail if `actual == 0` with the same precondition error.
2. `crates/worldwake-sim/src/action_payload.rs` defines `HarvestActionPayload` with `output_quantity: Quantity` (confirmed at line 321 during reassessment). Renaming to `requested_quantity` clarifies that the payload now carries the *target* (what the agent asked for), not the guaranteed delivery. All `harvest_payload(...)` call sites and helper construction sites must be updated.
3. `crates/worldwake-sim/src/action_handler.rs:13-30` defines `CommitOutcome { materializations: Vec<Materialization>, trace: Option<CommitTraceData> }` (confirmed). `CommitTraceData` lives nearby (exact location to confirm during implementation — likely same file or `commit_trace.rs`). Adding `partial_quantity: Option<Quantity>` to `CommitTraceData` requires updating every action-commit-handler ctor that constructs it, but most existing handlers can use struct-update syntax (`CommitTraceData { partial_quantity: None, ..existing }`) or rely on `Default` if `CommitTraceData` derives it. Confirm `Default` derive presence during implementation.
4. `LastHarvestTrace` component lands in ticket 004; the append helper `LastHarvestTrace::push(entry)` enforces the 8-entry ring cap. The commit handler reads (`get_component_last_harvest_trace`), pushes the new entry, and writes back (`set_component_last_harvest_trace`).
5. Shared boundary: the action-commit lifecycle. `CommitTraceData` flows from action handlers → `ActionTraceSink` (`crates/worldwake-sim/src/action_trace.rs`) → consumers (decision trace, observer rendering). Adding `partial_quantity` to the trace data is observed-only — no consumer is forced to read it; the formatter that surfaces `quantity_actual / quantity_requested` is added in this ticket.
6. The AI tick step reads `CommitTraceData.partial_quantity` after harvest commit to record the agent's actual inventory delta. Locate the AI-side commit consumer during implementation: `grep -rn "CommitTraceData\|materialization" crates/worldwake-ai/src/agent_tick/` to find the read site. Likely lives in `tick_step.rs` or `agent_tick.rs`.
7. `is_satisfied` semantic was set in ticket 002 (`inventory >= desired_min`). After this ticket's partial-success path, the agent's believed inventory reflects actual harvested quantity, so `is_satisfied` re-evaluation on the next planning tick correctly distinguishes "partial completion satisfied desired_min" from "partial completion still below desired_min."
8. `SAVE_FORMAT_VERSION` after ticket 005 is `52`; this ticket bumps to `53` (renames a payload field; bincode field-name changes don't strictly require a bump — but adding `partial_quantity` to `CommitTraceData` does, since traces may be replay-serialized). Bump to `53`.
9. Stale-request / contested-affordance boundary check: this ticket modifies authoritative commit-time behavior (no longer fails on shortage). Per Authoritative-to-AI Impact Rule (CLAUDE.md), trace through:
   - `get_affordances`: unchanged — affordance still exposes harvest at the source.
   - `generate_candidates`: unchanged — emitter still emits AcquireCommodity goals (ticket 007 lands quantity-aware emission).
   - `search_plan`: terminal ordering unchanged for full-quantity case; partial completion is a successful commit, so search doesn't see it as failure.
   - `BestEffort` action start: unchanged — the start handler's precondition check still validates `available >= 1`; the new partial-quantity logic kicks in at commit, not start.
   - `handle_plan_failure`: not invoked for partial completion (commit succeeds).
   - Payload revalidation: `requested_quantity` is a planner-synthesized value (from `desired_target` in ticket 007), so `with_payload_override_validator` registration is required — confirm this ticket adds the validator function. The validator confirms `requested_quantity <= source.available_quantity` at revalidation time? No — the validator confirms `requested_quantity` is within the agent's carry headroom and within the source's available_quantity at revalidation time; partial completion at commit is a separate path.
   - Goldens: ticket 008 covers.
12. Scenario isolation: this ticket has no golden of its own (ticket 008 owns goldens); focused unit/runtime tests cover the partial-success branch in isolation from full-success and depleted paths.
13. Adjacent contradictions: `CommitTraceData.partial_quantity = None` is the universal default for non-harvest handlers — they don't need touching beyond the ctor update. Ticket 008's goldens will exercise the partial path end-to-end.

### Auto-corrections (2026-04-27)

14. **`CommitTraceData` is a tagged enum, not a struct.** Live shape (`crates/worldwake-sim/src/action_handler.rs:39-41`):
    ```rust
    pub enum CommitTraceData { Tell(TellCommitTrace) }
    ```
    The ticket's "add `partial_quantity: Option<Quantity>` field" approach assumed a struct. **Correction applied:** add a new variant `Harvest(HarvestCommitTrace)` where `HarvestCommitTrace { partial_quantity: Option<Quantity>, requested_quantity: Quantity }`. The harvest commit handler returns `CommitOutcome::empty().with_trace(CommitTraceData::Harvest(...))` only when partial-success occurs (full-success returns `CommitOutcome::empty()` with `trace: None`). **Why safe:** directionally unambiguous given the existing per-action tagged-variant pattern; preserves `CommitOutcome` shape (FND-28); aligns with the existing `Tell(TellCommitTrace)` precedent (FND-26 — per-action state lanes); other action handlers do **not** need ctor changes (auto-correction supersedes the original "update all `CommitTraceData` constructors with `partial_quantity: None`" guidance and ticket text section 3 below). The single existing consumer site (`tell_actions.rs:1150 Some(CommitTraceData::Tell(trace))`) compiles unchanged because the match arm is non-exhaustive on shape — but the formatter at `action_trace.rs:339 match trace { CommitTraceData::Tell(tell) => ... }` requires a new arm.

15. **No AI tick step consumer of `payload.output_quantity` for inventory accounting.** `grep -rn "payload.output_quantity\|harvest.output_quantity" crates/worldwake-ai/src` returns only:
    - `plan_guard_build.rs:27` — guard `QuantitySource::PayloadCommodityQuantity` reads to construct `StatePredicate::ActorHoldsCommodity { min_quantity }` for plan expectations.
    - `goal_model.rs:1233` — planner hypothetical state mutation in `apply_planner_step` after `PlannerOpKind::Harvest`.
    Both represent the planner's *intended* outcome (rename to `requested_quantity` is semantically correct: the planner plans for the requested amount; partial-success is recovery via `is_satisfied` re-evaluation against `desired_min`). **Correction applied:** ticket section 5 ("AI tick step reads `partial_quantity` for inventory accounting") becomes a verification-only requirement — the agent's believed inventory after partial harvest reflects `actual` units automatically because the commit handler creates an `ItemLot` of exactly `actual` units, which the actor perceives through their belief view. Acceptance test #4 (`ai_tick_records_partial_inventory_delta`) verifies the contract via belief-view inspection of the actor's commodity quantity after a partial harvest scenario. **Why safe:** production semantics already line up — the lot carries `actual` units, perception is the canonical belief substrate (FND-14); no new direct consumer of `CommitTraceData` is required; the trace surface is observed-only for debug/golden assertion (FND-29). The validator path is unchanged: `requested_quantity` is the carry-headroom-bounded planner intent.

16. **`harvest_action_def` payload construction site (`crates/worldwake-systems/src/production_actions.rs:121-127`).** The action def carries a fixed canonical payload `ActionPayload::Harvest(HarvestActionPayload { ..., output_quantity: *output_quantity })` derived from the recipe at registration. After rename, this becomes `requested_quantity: *output_quantity`. Tests at `production_actions.rs:1234-1241` and `1238` need updating. **Correction applied:** rename construction site and existing test fixtures in lockstep.

17. **No `with_payload_override_validator` on harvest currently.** `register_harvest_actions` at `production_actions.rs:22-26` registers with `with_start_failure(...)` and `with_authoritative_payload_validator(validate_exclusive_facility_grant)` only. **Correction applied:** add `with_payload_override_validator(validate_harvest_payload_override)` per spec D8 / Authoritative-To-AI Impact point 6. Validator confirms `requested_quantity >= 1` and `requested_quantity <= carry_headroom` (computed from `view.carry_capacity(actor)` minus `view.load_of_entity(actor)`, divided by `load_per_unit(commodity)`). Pattern mirrors `validate_pick_up_payload_override` at `transport_actions.rs:771-807`.

18. **`commit_conditions` duplicate `TargetHasResourceSource` blocks the partial-success path.** `harvest_action_def` at `production_actions.rs:120` clones `preconditions` into `commit_conditions`, including `TargetHasResourceSource { min_available: requested_quantity }`. `tick_action.rs:157-169` evaluates `commit_conditions` BEFORE `commit_harvest` runs and aborts the action when any check fails. With this duplicate check in place, the partial-success case (`available < requested && available >= 1`) is short-circuited to a `CommitConditionFailed` abort and never reaches `commit_harvest`, so the partial trace and `CommitTraceData::Harvest` aftermath would never fire. **Correction applied:** filter `TargetHasResourceSource` out of `commit_conditions` (mirrors the existing `commit_craft` filter precedent at `production_actions.rs:189-194` which strips `TargetLacksProductionJob`). Source-availability becomes solely owned by `commit_harvest`, which now handles full / partial / depleted cases authoritatively. **Why safe:** start preconditions still gate action initiation against `min_available: requested`, so the action only begins when the source has the full requested amount. Mid-action depletion (the only path where `available < requested` at commit time) is explicitly the partial-success regime the spec authorizes (FND-10 — granular outcomes with aftermath).

19. **Depleted-failure trace persistence requires `AbortRequested`, not `PreconditionFailed`.** `tick_action.rs:188-220` shows that the runtime drops the WorldTxn on `Err(err) => Err(err)` for non-`AbortRequested` errors. Returning `ActionError::PreconditionFailed("source depleted...")` from `commit_harvest` rolls back the trace write the handler attempted. The ticket's invariant 1 ("trace appended on success **or failure**") and spec D7 both require persistence. **Correction applied:** the depleted-failure path returns `ActionError::AbortRequested(ActionAbortRequestReason::HarvestSourceDepleted { workstation })`, which routes through `finalize_failed_action` (`tick_action.rs:201-218`) and commits the txn including the failed-trace append. Required additions: (a) new `HarvestSourceDepleted { workstation: EntityId }` variant on `ActionAbortRequestReason` in `crates/worldwake-sim/src/action_handler.rs:296`; (b) match arm in `crates/worldwake-ai/src/failure_handling.rs:722` classifying it as `BlockingFact::MissingInput(commodity)` (closest behavioral analog — the source ran out of the commodity). **Why safe:** bounded fallout — one enum variant, one exhaustive-match arm; preserves spec invariant 1; the existing abort lifecycle already commits txn writes from the handler.

## Architecture Check

1. Surfacing partial-quantity through `CommitTraceData` (Question 2 option b) preserves `CommitOutcome` shape — no foundational-type shim per FND-28. The trace surface is already designed for action-specific outcome metadata; this is a natural extension.
2. Renaming `output_quantity → requested_quantity` clarifies the payload's contract. Per FND-28, the rename is atomic (no alias path).
3. The append-and-prune model for `LastHarvestTrace` matches FND-29A — append-only with bounded retention. Pruning is owned by `item_decay_system` (ticket 004), not the commit handler.
4. The validator registration for planner-synthesized `requested_quantity` payloads honors the Authoritative-to-AI Impact Rule's payload revalidation point (CLAUDE.md, point 6).

## Verification Layers

1. Full-success commit (`available >= requested`) preserves existing behavior: source decrements by `requested`, item lot of `requested` units, `partial_quantity == None` → focused unit test in `production_actions.rs` `#[cfg(test)]`.
2. Partial-success commit (`0 < available < requested`): source drains to 0, item lot of `available` units, `partial_quantity == Some(actual)`, `LastHarvestTrace` appends `{ quantity: actual, partial: true }` → focused unit test.
3. Failure on depleted source (`available == 0`): `ActionError::PreconditionFailed`, `LastHarvestTrace` appends `{ quantity: 0, partial: true }` → focused unit test. Map per `docs/precision-rules.md` Rule 9 to **focused authoritative runtime coverage** (action trace + authoritative world state via `EventTag::Inventory` delta).
4. AI tick step reads `partial_quantity` and records actual inventory delta → focused runtime test in `crates/worldwake-ai/src/agent_tick/` `#[cfg(test)]` block.
5. Action-commit trace formatter emits `quantity_actual / quantity_requested` for partial harvests → action-trace assertion in a focused or runtime test.
6. Payload revalidation rejects `requested_quantity > carry_headroom` → focused test in plan-revalidation infrastructure.
7. Save format rejects version `52` saves → existing infrastructure.

## What to Change

### 1. Rename `HarvestActionPayload.output_quantity` to `requested_quantity` in `crates/worldwake-sim/src/action_payload.rs:321`

Update all construction sites and helper functions (`harvest_payload(...)` and any builders) workspace-wide. Per FND-28 no alias path.

### 2. Modify `commit_harvest` in `crates/worldwake-systems/src/production_actions.rs:553-617`

Replace the `checked_sub` failure path with the partial-success logic per spec D7:

```rust
let available = source.available_quantity.0;
let requested = payload.requested_quantity.0;
let actual = available.min(requested);

if actual == 0 {
    // Append failed-harvest trace entry, then fail.
    let mut trace = txn.get_component_last_harvest_trace(workstation).cloned().unwrap_or_default();
    trace.push(HarvestTraceEntry {
        harvester: instance.actor,
        tick: txn.tick(),
        quantity: 0,
        partial: true,
    });
    txn.set_component_last_harvest_trace(workstation, trace)?;
    return Err(ActionError::PreconditionFailed(format!(
        "source {workstation} depleted during action"
    )));
}

source.available_quantity = Quantity(available - actual);
txn.set_component_resource_source(workstation, source)?;

let owner = resolve_output_owner(txn, instance.actor, workstation)?;
let lot = txn.create_item_lot_with_owner(
    payload.output_commodity,
    Quantity(actual),
    place,
    owner,
)?;
txn.add_target(lot);

let mut trace = txn.get_component_last_harvest_trace(workstation).cloned().unwrap_or_default();
trace.push(HarvestTraceEntry {
    harvester: instance.actor,
    tick: txn.tick(),
    quantity: actual as u16,
    partial: actual < requested,
});
txn.set_component_last_harvest_trace(workstation, trace)?;

record_successful_source_acquisition(...)?;

let partial_quantity = (actual < requested).then(|| Quantity(actual));
Ok(CommitOutcome {
    materializations: vec![],  // existing pattern; confirm during implementation
    trace: Some(CommitTraceData {
        partial_quantity,
        ..Default::default()
    }),
})
```

The exact ctor pattern depends on `CommitTraceData`'s current shape (confirm whether `Default` is derived during implementation).

### 3. Add `partial_quantity: Option<Quantity>` to `CommitTraceData`

Locate `CommitTraceData` (likely `crates/worldwake-sim/src/action_handler.rs` or `crates/worldwake-sim/src/commit_trace.rs` — confirm). Add the field with `#[serde(default)]`. Update all action-commit handler ctors that construct `CommitTraceData` to include `partial_quantity: None` (most can use struct-update syntax if `Default` is derived).

### 4. Surface `partial_quantity` in the action-commit trace formatter

Locate the action-commit trace formatter (likely `crates/worldwake-sim/src/action_trace.rs` or a goal-formatting helper). For harvest commits with `partial_quantity == Some(actual)`, format as `harvest commit: quantity_actual={actual} / quantity_requested={requested}` (the requested value is reachable through the action instance's payload).

### 5. AI tick step reads `partial_quantity` for inventory accounting

Locate the AI-side commit consumer in `crates/worldwake-ai/src/agent_tick/` (likely `tick_step.rs`). When the consumer sees a harvest commit with `partial_quantity == Some(actual)`, record the agent's believed inventory delta as `actual` (not `requested`). Where the consumer currently reads `payload.output_quantity` to update belief-state, replace with: prefer `commit_trace.partial_quantity.unwrap_or(payload.requested_quantity)`.

### 6. Add payload override validator for `requested_quantity`

Register a validator via `with_payload_override_validator` for the harvest action's planner-synthesized payloads. The validator confirms `requested_quantity <= carry_headroom` (computed from believed inventory + `CarryCapacity`) and `requested_quantity >= 1`. Source-side validation (`requested_quantity <= source.available_quantity`) is **not** part of revalidation — that check happens at commit, where partial-success now handles the `<` case gracefully.

### 7. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:6` — bump from `52` to `53`.

### 8. Add focused tests

- `commit_harvest_full_success` — pre-existing behavior preserved.
- `commit_harvest_partial_success` — `available < requested` produces partial materialization and `partial_quantity == Some(actual)`.
- `commit_harvest_depleted_failure` — `available == 0` fails with `PreconditionFailed` and appends a 0-quantity partial trace entry.
- `ai_tick_records_partial_inventory_delta` — runtime `agent_tick` test asserting believed inventory increments by `actual`, not `requested`.
- `harvest_payload_validator_rejects_overcarry` — revalidation rejects `requested_quantity > carry_headroom`.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — rename field at line 321)
- `crates/worldwake-sim/src/action_handler.rs` (modify — `CommitTraceData.partial_quantity` field)
- `crates/worldwake-sim/src/action_trace.rs` (modify — formatter surface for partial harvest; **Likely:** confirm exact module via `grep -n "harvest" crates/worldwake-sim/src/action_trace.rs` during reassessment)
- `crates/worldwake-systems/src/production_actions.rs` (modify — `commit_harvest` partial path)
- All harvest payload construction sites workspace-wide (modify — `output_quantity → requested_quantity`)
- All `CommitTraceData { … }` construction sites workspace-wide (modify — add `partial_quantity: None`)
- `crates/worldwake-ai/src/agent_tick/tick_step.rs` (modify — partial-quantity inventory accounting; **Likely:** confirm exact consumer site via `grep -rn "CommitTraceData" crates/worldwake-ai/src/agent_tick/` during reassessment)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — register validator for harvest payload)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION)

## Out of Scope

- Multi-slot harvest start handler (scanning `queues[..]` to pick a free slot) — ticket 007.
- Candidate generation deriving `desired_target` from agent state — ticket 007.
- Ranking integration with `SourceReliability.average_wait_ticks` — ticket 007.
- End-to-end goldens proving partial-success in scenario context — ticket 008.

## Acceptance Criteria

### Tests That Must Pass

1. `commit_harvest_full_success` — full-quantity commit preserves existing behavior; `partial_quantity == None`.
2. `commit_harvest_partial_success` — `actual = min(available, requested)`, source drains to 0, item lot of `actual`, `partial_quantity == Some(actual)`, `LastHarvestTrace` records `partial: true`.
3. `commit_harvest_depleted_failure` — `available == 0` fails; `LastHarvestTrace` records `quantity: 0, partial: true`.
4. `ai_tick_records_partial_inventory_delta` — agent's believed inventory increments by `actual`, not `requested`.
5. `harvest_payload_validator_rejects_overcarry` — revalidation rejects `requested_quantity > carry_headroom`.
6. Existing harvest-related goldens still pass (named during reassessment).
7. Existing suite: `cargo test --workspace`.

### Invariants

1. Harvest commit always appends exactly one `HarvestTraceEntry` to the source's `LastHarvestTrace` (success or failure).
2. `CommitOutcome.trace.partial_quantity == Some(actual)` iff `actual < requested` and `actual >= 1`; `None` otherwise (including for non-harvest handlers).
3. Believed agent inventory after harvest commit reflects `actual`, not `requested` (FND-3 — concrete state, not the planner's intent).
4. Per FND-29A, `LastHarvestTrace` appends are append-only — no commit path mutates an existing entry.
5. `SAVE_FORMAT_VERSION = 53`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/production_actions.rs` `#[cfg(test)]` — three commit paths (full, partial, depleted).
2. `crates/worldwake-ai/src/agent_tick/tick_step.rs` `#[cfg(test)]` — runtime test for partial inventory accounting.
3. `crates/worldwake-ai/src/plan_revalidation.rs` `#[cfg(test)]` — overcarry validator test.
4. Update existing harvest-related tests (recorded during reassessment) to match the new payload field name.

### Commands

1. `cargo test -p worldwake-systems commit_harvest`
2. `cargo test -p worldwake-ai ai_tick_records_partial harvest_payload_validator`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`

## Outcome

Completed on 2026-04-27.

- Renamed `HarvestActionPayload.output_quantity` → `requested_quantity` (`crates/worldwake-sim/src/action_payload.rs:325`) and propagated across all destructure / construction sites: `affordance.rs`, `affordance_query.rs`, `production_actions.rs` (registration + tests), `goal_model.rs` (planner hypothetical + test fixtures), `plan_guard_build.rs`, `plan_revalidation.rs` test fixtures, `search/tests.rs` test fixtures.
- Extended the `CommitTraceData` enum with a `Harvest(HarvestCommitTrace)` variant carrying `requested_quantity: Quantity` and `partial_quantity: Option<Quantity>` (`crates/worldwake-sim/src/action_handler.rs:39-49`, re-exported from `lib.rs`). Other action handlers' `CommitOutcome` constructors are unchanged.
- Rewrote `commit_harvest` (`crates/worldwake-systems/src/production_actions.rs:558-650`) to compute `actual = min(available, requested)`, succeed for `actual >= 1` (with partial trace data when `actual < requested`), and return `ActionError::AbortRequested(ActionAbortRequestReason::HarvestSourceDepleted { workstation })` for `actual == 0` so that `finalize_failed_action` commits the failed-trace append. Both branches push a `HarvestTraceEntry` onto `LastHarvestTrace`.
- Filtered `TargetHasResourceSource` out of harvest `commit_conditions` (`production_actions.rs:120-129`) so the partial-success and depleted-failure paths are reachable; the start-time precondition still gates action initiation.
- Registered `validate_harvest_payload_override` (`production_actions.rs:752-779`) on the harvest handler via `with_payload_override_validator`. Validator confirms `requested_quantity >= 1` and `requested_quantity <= believed_carry_headroom`.
- Added the `format_commit_trace` formatter arm for `CommitTraceData::Harvest` (`crates/worldwake-sim/src/action_trace.rs:360-368`) emitting `harvest commit: quantity_actual={n} / quantity_requested={n}`.
- Added `ActionAbortRequestReason::HarvestSourceDepleted { workstation }` (`crates/worldwake-sim/src/action_handler.rs:362-364`) and the matching `=> None` arm in `crates/worldwake-ai/src/failure_handling.rs:765-771` (consolidated with the existing `ViolationNoLongerActive` arm).
- Bumped `SAVE_FORMAT_VERSION` 52 → 53 (`crates/worldwake-sim/src/save_load.rs:6`).
- Added focused tests in `crates/worldwake-systems/src/production_actions.rs` `#[cfg(test)]`: `commit_harvest_full_success_emits_no_partial_trace`, `commit_harvest_partial_success_emits_partial_trace_and_drains_source`, `commit_harvest_depleted_failure_records_zero_quantity_partial_trace`, `ai_tick_records_partial_inventory_delta`, `harvest_payload_validator_rejects_overcarry`. Also augmented `register_harvest_actions_creates_recipe_backed_action_defs` with an assertion that `commit_conditions` no longer duplicates `TargetHasResourceSource`.

## Deviations

- The original ticket assumed `CommitTraceData` was a struct gaining a `partial_quantity: Option<Quantity>` field. Live shape is a tagged enum (`Tell(TellCommitTrace)`); auto-corrected to add a new `Harvest(HarvestCommitTrace)` variant matching the existing per-action variant precedent (FND-26). No other action handler's commit-outcome constructors needed updating. Recorded as Assumption Reassessment item 14.
- Spec D7 / ticket invariant 1 require trace persistence on the depleted-failure branch, but `tick_action.rs:220` drops the WorldTxn for non-`AbortRequested` errors. The depleted-failure branch was promoted to `ActionError::AbortRequested(HarvestSourceDepleted)` so `finalize_failed_action` commits the trace append. Bounded fallout: one new enum variant + one match arm. Recorded as item 19.
- Section 5 of the ticket prescribed wiring an "AI tick step consumer" of `commit_trace.partial_quantity`. Live AI code does not read `payload.output_quantity` for inventory accounting (the agent's believed inventory is established through perception of the `ItemLot` the commit creates, which now carries `actual` units, not `requested`). Section 5 is satisfied by the production semantics; the `ai_tick_records_partial_inventory_delta` test verifies the contract via `InventoryBeliefView::commodity_quantity` rather than wiring a new direct CommitTraceData consumer. Recorded as item 15.
- Filtering `TargetHasResourceSource` out of harvest's `commit_conditions` was an unstated but necessary subscope: without it, the duplicated precondition check at `tick_action.rs:157-169` fires before `commit_harvest` runs and blocks the partial-success and depleted-failure paths entirely. Mirrors the existing `commit_craft` precedent that filters `TargetLacksProductionJob`. Recorded as item 18.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib commit_harvest` (3/3 ok).
- Passed `cargo test -p worldwake-systems --lib ai_tick_records_partial` (1/1 ok).
- Passed `cargo test -p worldwake-systems --lib harvest_payload_validator` (1/1 ok).
- Passed `cargo test --workspace` (no failures across all crates).
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh` (fmt + workspace tests + both clippy variants + scenario-coverage --check; exit 0).
