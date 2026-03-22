# POLTRAC-003: Consolidate Political Trace Support/Timer Facts into Reusable Snapshots

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — narrow politics-trace schema cleanup in `worldwake-sim` and `worldwake-systems`, plus focused/golden assertion updates
**Deps**: [`archive/tickets/POLTRAC-001-political-system-trace-sink.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/POLTRAC-001-political-system-trace-sink.md), [`archive/tickets/completed/POLTRAC-002-extend-political-trace-with-timer-state-and-counted-support-snapshots.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/POLTRAC-002-extend-political-trace-with-timer-state-and-counted-support-snapshots.md), `docs/FOUNDATIONS.md`, `crates/worldwake-sim/src/politics_trace.rs`, `crates/worldwake-systems/src/offices.rs`

## Problem

`POLTRAC-002` added reusable `vacancy_timer` and `support_resolution` snapshots to the politics trace, but the authoritative explanation surface still duplicates part of the same meaning inside `OfficeSuccessionOutcome`. Today support-law timer/counting facts are split across:

- `OfficeSuccessionTrace.vacancy_timer`
- `OfficeSuccessionTrace.support_resolution`
- `OfficeSuccessionOutcome::WaitingForTimer`
- `OfficeSuccessionOutcome::SupportInstalled`
- `OfficeSuccessionOutcome::SupportResetTie`

That leaves two live authoritative representations of the same explanation substrate. The architecture is workable, but not ideal. It weakens Principle 25 by forcing consumers to decide which representation is canonical, and it brushes against Principle 26 by keeping an older embedded payload shape alive after the reusable snapshot shape exists.

The clean end state is:

- `OfficeSuccessionOutcome` names the semantic branch only, plus irreducible branch-specific identifiers such as installed holder or tied candidates
- reusable timer/support facts live in the snapshot structs only
- focused and golden consumers assert against the snapshot surface directly

## Assumption Reassessment (2026-03-22)

1. The live trace schema already contains the reusable snapshot substrate this cleanup wants to make canonical: [`VacancyTimerTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) and [`SupportResolutionTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs). This is not a request to invent new explanation data; it is a request to remove duplicated authoritative representation.
2. The duplication is current, concrete, and narrow:
   - [`OfficeSuccessionOutcome::WaitingForTimer`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) still carries `start_tick`, `waited_ticks`, `required_ticks`, and `remaining_ticks`
   - [`OfficeSuccessionOutcome::SupportInstalled`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) still carries `support`
   - [`OfficeSuccessionOutcome::SupportResetTie`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) still carries `support`
   while the same event can now also expose `trace.vacancy_timer` and `trace.support_resolution`
3. The authoritative closure boundary under cleanup is support-law succession resolution in [`resolve_support_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs). The ticket is about the explanation shape for:
   - timer-blocked waiting
   - support install
   - support tie reset
   - support no-eligible reset
   not about changing support-law semantics or office-holder mutation logic.
4. Existing focused coverage already proves the richer snapshot surface is live and useful:
   - [`support_succession_trace_records_install_with_resolution_snapshot()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs)
   - [`support_succession_trace_records_tie_reset_with_resolution_snapshot()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs)
   - [`support_succession_trace_records_no_eligible_reset_with_resolution_snapshot()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs)
   - [`succession_trace_records_vacancy_activation_and_timer_wait()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs)
5. Existing mixed-layer runtime coverage already exercises the real consumer path: [`golden_knowledge_asymmetry_race_informed_wins_office`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1762) proves the timed political race and now reads the snapshot surface directly. This follow-up should update remaining assertions to prefer snapshots rather than outcome-embedded timer/support payloads.
6. This is not a planner behavior ticket. The live `GoalKind` consumer remains `ClaimOffice` in Scenario 34, but that golden is only a verification surface here. No candidate generation, ranking, search, or `agent_tick` semantics are intended to change.
7. Ordering remains mixed-layer, but the contract under this ticket is not earlier-vs-later action ordering. The contract is authoritative trace shape: timer/counting facts should be read from snapshot structs, while semantic branch selection remains in `OfficeSuccessionOutcome`. Action trace and decision trace stay relevant only as external verification that the cleanup preserved real behavior.
8. This ticket is traceability cleanup, not heuristic removal. The missing substrate that `POLTRAC-002` introduced is already present. This follow-up pays the migration cost required by [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principle 26 instead of leaving two live authority-path representations of the same fact.
9. Principles 25, 26, and 27 all apply directly:
   - Principle 25: derived summaries must not compete with truth; snapshot structs should be the single explanation substrate
   - Principle 26: no backward compatibility in live authority paths; old outcome-embedded timer/support payloads should not coexist with the canonical snapshot path
   - Principle 27: debuggability is a product feature; consumers should have one authoritative explanation surface, not two
10. Scenario isolation is not central here because this is not a new golden branch ticket. The relevant real-world consumer remains Scenario 34, which is already isolated to co-located sated agents and knowledge asymmetry. That scenario should remain unchanged except for assertion shape if needed.
11. Mismatch + correction: no remaining active ticket in `tickets/` currently owns this cleanup. [`tickets/S19INSRECCON-004.md`](/home/joeloverbeck/projects/worldwake/tickets/S19INSRECCON-004.md) is a stale duplicate of already-delivered Scenario 34 work, and [`tickets/S19INSRECCON-005.md`](/home/joeloverbeck/projects/worldwake/tickets/S19INSRECCON-005.md) is docs-only. This ticket is the correct owner for the schema consolidation.

## Architecture Check

1. Consolidating timer/support facts into reusable snapshots is cleaner than leaving those facts duplicated inside both `OfficeSuccessionOutcome` and `OfficeSuccessionTrace`. It gives consumers one canonical explanation path and makes the outcome enum describe branch identity rather than carry partially duplicated payload detail.
2. This approach is more robust than adding helper methods that reconcile the two shapes at read time. A helper would preserve the duplication instead of removing it, which is exactly the kind of compatibility layer Principle 26 says to avoid.
3. This cleanup aligns with the current architecture because the snapshot structs already exist and are already being populated from authoritative state in `offices.rs`. The migration cost is bounded and localized.
4. No backwards-compatibility aliasing or shims should be introduced. Update all trace consumers and tests directly to the canonical snapshot surface.

## Verification Layers

1. Waiting/install/tie/no-eligible support-law events still expose the same timer and counted-support facts after cleanup -> focused authoritative trace tests in `crates/worldwake-systems/src/offices.rs`
2. `OfficeSuccessionOutcome` continues to identify the semantic branch correctly after payload removal -> focused authoritative trace tests in `crates/worldwake-systems/src/offices.rs`
3. Real timed political race remains explainable through action ordering + canonical politics snapshots + authoritative holder mutation -> `crates/worldwake-ai/tests/golden_offices.rs`
4. Later office-holder mutation is not being used as a proxy for trace-shape correctness; the trace data contract itself is asserted directly in focused tests, while golden authoritative state remains a separate proof surface
5. This is not a single-layer ticket because the architecture promise includes both the authoritative trace contract and at least one real mixed-layer consumer path

## What to Change

### 1. Simplify `OfficeSuccessionOutcome` to semantic branch data

Remove timer/support-count payload fields from outcome variants where that information is now duplicated by reusable snapshots. The intended shape is:

- waiting branch remains identifiable as waiting, but timer arithmetic comes from `trace.vacancy_timer`
- support install remains identifiable, but support totals come from `trace.support_resolution`
- tie reset remains identifiable, with tied candidates retained if still branch-specific, but tie support totals come from `trace.support_resolution`
- no-eligible reset remains identifiable without duplicate support arithmetic

Do not remove irreducible branch-specific identifiers when they are not duplicated elsewhere.

### 2. Update trace construction and summaries in `politics_trace.rs` and `offices.rs`

Adjust trace recording and `PoliticalTraceEvent::summary()` so summaries read from the canonical snapshot surface rather than from duplicated outcome payloads. Keep summaries concise and reconstructable from the trace itself.

### 3. Migrate focused and golden assertions to the canonical snapshot contract

Update focused offices trace tests and any golden consumers that still pattern-match on outcome-embedded timer/support payloads. The tests should assert:

- semantic branch from `OfficeSuccessionOutcome`
- timer/counting detail from `vacancy_timer` and `support_resolution`

That separation is the architecture this ticket is codifying.

## Files to Touch

- `crates/worldwake-sim/src/politics_trace.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify if export surface changes)
- `crates/worldwake-systems/src/offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify if any outcome pattern matches require adjustment)
- `crates/worldwake-ai/tests/golden_harness/timeline.rs` (modify if fixture trace construction needs updated outcome shape)

## Out of Scope

- Changing support-law political semantics
- Changing action ordering, AI planning, or consultation duration math
- Adding new trace sinks or cross-system timeline features
- Reworking force-law trace semantics beyond any mechanical pattern-match updates required by the enum cleanup

## Acceptance Criteria

### Tests That Must Pass

1. focused politics-trace tests in `crates/worldwake-systems/src/offices.rs` assert semantic branch separately from snapshot detail for waiting/install/tie/no-eligible support-law paths
2. `cargo test -p worldwake-systems support_succession_trace_records_install_with_resolution_snapshot`
3. `cargo test -p worldwake-systems support_succession_trace_records_tie_reset_with_resolution_snapshot`
4. `cargo test -p worldwake-systems support_succession_trace_records_no_eligible_reset_with_resolution_snapshot`
5. `cargo test -p worldwake-systems succession_trace_records_vacancy_activation_and_timer_wait`
6. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
7. `cargo test -p worldwake-ai`
8. `cargo test --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `vacancy_timer` and `support_resolution` become the single canonical source for timer/counting explanation data in the politics trace
2. `OfficeSuccessionOutcome` remains a semantic branch identifier, not a second storage location for snapshot facts
3. No authoritative political behavior changes
4. No compatibility shims, duplicated old/new event types, or read-time reconciliation wrappers are added

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs::succession_trace_records_vacancy_activation_and_timer_wait` — updated to assert waiting-branch identity separately from canonical `vacancy_timer`
2. `crates/worldwake-systems/src/offices.rs::support_succession_trace_records_install_with_resolution_snapshot` — updated to assert install-branch identity separately from canonical `support_resolution`
3. `crates/worldwake-systems/src/offices.rs::support_succession_trace_records_tie_reset_with_resolution_snapshot` — updated to assert tie-branch identity separately from canonical `support_resolution`
4. `crates/worldwake-systems/src/offices.rs::support_succession_trace_records_no_eligible_reset_with_resolution_snapshot` — updated to assert no-eligible branch identity separately from canonical empty `support_resolution`
5. `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office` — updated if needed so the real consumer asserts canonical politics snapshots rather than any outcome-embedded support/timer payloads

### Commands

1. `cargo test -p worldwake-systems support_succession_trace_records_install_with_resolution_snapshot`
2. `cargo test -p worldwake-systems support_succession_trace_records_tie_reset_with_resolution_snapshot`
3. `cargo test -p worldwake-systems support_succession_trace_records_no_eligible_reset_with_resolution_snapshot`
4. `cargo test -p worldwake-systems succession_trace_records_vacancy_activation_and_timer_wait`
5. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
6. `cargo test -p worldwake-ai`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`
