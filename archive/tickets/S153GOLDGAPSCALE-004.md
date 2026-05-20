# S153GOLDGAPSCALE-004: Office-backed patrol duty assignment substrate

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — office patrol duty component, duty lifecycle system, patrol candidate/ranking integration, save-format version bump
**Deps**: `specs/S153-golden-gaps-ai-architecture-scaling.md`, `archive/tickets/S153GOLDGAPSCALE-002.md`

## Problem

`archive/tickets/S153GOLDGAPSCALE-002.md` was rejected because the requested office-vacancy → patrol-gap golden assumed production substrate that does not exist. Live patrol behavior is driven by `PatrolRoute` / `PatrolProfile`; `ExpectationStore` overdue state drives missing-person search/report motives, not patrol-duty validity. FOUNDATIONS Canonical Scenario F still requires the architecture to produce office vacancy → duty degradation → patrol gap → opportunistic route predation from generic institutional state, not from a hidden scenario flag.

This ticket added the missing office-backed patrol duty assignment substrate as first-class world state. Live reassessment split the full S153 D3 golden into `archive/tickets/S153GOLDGAPSCALE-005.md`; that successor later landed the end-to-end golden while this ticket remained closed on the production substrate and focused AI/system proof.

## Assumption Reassessment (2026-05-20)

1. Live patrol motive is `crates/worldwake-ai/src/ranking.rs::patrol_motive`: it reads `PatrolProfile`, `PatrolRoute`, unresolved thefts, believed office vacancies, and force contests. It does not read `ExpectationStore`.
2. Live expectation-overdue motive is `crates/worldwake-ai/src/ranking.rs::expectation_response_motive`: it applies to `GoalKind::SearchForMissing`, `GoalKind::ReportMissing`, and `GoalKind::ReportFound`. `ExpectationBasis::DutyAssignment` increases missing-person expectation weight only; it is not a patrol-duty lifecycle.
3. Live patrol assignment state is `crates/worldwake-core/src/patrol.rs::PatrolRoute` plus `PatrolProfile`. It records assigned places and patrol parameters, but no issuing office, assignee duty id, lifecycle state, renewal cadence, vacancy invalidator, or inspectable social artifact.
4. Shared boundary under audit: authoritative institutional duty state in `worldwake-core` / `worldwake-systems` must become the source that AI candidate generation, ranking, and portfolio admission use for office-backed patrol duties. Derived planner views may cache it, but they cannot become truth.
5. Information-path statement: an office-backed patrol duty is a social artifact/record. Agents may learn about it through local office records, notices, direct assignment, testimony, or observation of non-performance. This ticket must not give AI global knowledge of all duties.
6. Canonical end state: a guard patrols because they hold an active duty assignment issued by an office or lawful delegate. When that office is vacant and no successor/delegate renews the assignment, the duty degrades or lapses through explicit lifecycle state, and the guard's `ObligationDuty` slot no longer treats it as a valid patrol obligation.
7. If implementation finds an already-existing duty artifact that satisfies this contract, narrow this ticket to wiring and golden proof, but record the exact existing owner first.

## Architecture Check

1. A concrete duty assignment artifact aligns with FND-23, FND-25, and FND-25A: the duty has issuer, assignee, jurisdiction/route, lifecycle, legal effect, visibility/actionability, invalidators, and causal records.
2. AI integration aligns with FND-20 and FND-21: guards choose patrol because their beliefs and duty state make it a lawful commitment, and they can revise when the duty lapses or its assumptions break.
3. Systems remain state-mediated per FND-26: office lifecycle, duty maintenance, patrol candidate generation, and route-danger aftermath communicate through state and event history, not direct system calls.
4. No backward-compatibility shim: once office-backed duties become the canonical patrol-obligation path, update or remove stale test-only assumptions rather than preserving a parallel fake path.

## Verified Layers

1. Duty assignment creation / renewal / lapse -> authoritative duty component or record state plus append-only event-log delta.
2. Office vacancy or missing successor degrades/lapses office-backed duties -> duty lifecycle state and causal event history.
3. Active duty exposes a patrol candidate in the guard's `ObligationDuty` slot -> candidate-generation and decision-trace proof.
4. Lapsed/degraded duty removes or suppresses that patrol obligation -> decision trace showing `ObligationDuty` no longer ranks the patrol duty.
5. Patrol gap permits route predation only through ordinary actor behavior -> action trace / event log for route traversal or predation, with no hidden scenario flag.
6. Merchant or other observer learns the route danger through local evidence/report path -> `RoutePreferenceEntry` or successor route-danger belief state.
7. Determinism -> same seed produces equal event log hash and equal `ScenarioDiagnosticsReport`.

## Landed Changes

### 1. Add office-backed duty assignment state

Introduced `OfficePatrolDuty` on guard agents with issuing office/delegate, assignee, covered route places, created tick, renewal due tick, grace ticks, lifecycle state, actionability helper, and provenance.

### 2. Add lifecycle maintenance

Added `office_patrol_duty_lifecycle_system` under the patrol system slot. It degrades active duties after a vacant office misses renewal and lapses degraded duties after the grace window, emitting ordinary `System` / `WorldMutation` event-log records.

### 3. Wire AI patrol obligation consumption

Updated patrol candidate generation and patrol motive ranking so a lapsed office-backed duty suppresses `GoalKind::Patrol` emission and zeroes patrol motive. Existing non-office patrol routes remain lawful when no office duty component exists; `archive/tickets/S153GOLDGAPSCALE-005.md` later landed the end-to-end golden that exercises the new duty path as the office-vacancy contract.

### 4. Land the S153 D3 golden

Landed later in `archive/tickets/S153GOLDGAPSCALE-005.md`.

### 5. Truth-sync generated golden docs

Landed later in `archive/tickets/S153GOLDGAPSCALE-005.md`; this ticket landed no source golden metadata.

## Landed Files

- `crates/worldwake-core/src/patrol.rs`
- `crates/worldwake-core/src/component_schema.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/lib.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-sim/src/save_load.rs`
- `crates/worldwake-systems/src/lib.rs`
- `crates/worldwake-systems/src/patrol.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `specs/S153-golden-gaps-ai-architecture-scaling.md`
- `archive/tickets/S153GOLDGAPSCALE-005.md`

## Out of Scope

- The scaled-contention golden and route-blocker helper (`tickets/S153GOLDGAPSCALE-003.md`).
- A general duty system for every possible institutional duty kind beyond the patrol duty path needed for Canonical Scenario F.
- Omniscient town-manager code, hidden scenario flags, or planner-only fake duty state.
- Backward-compatibility support for a parallel non-duty patrol-obligation path once the new path is canonical.

## Acceptance Result

### Completed Proof

1. Focused core test proves `OfficePatrolDuty` component serialization.
2. Focused systems tests prove office-vacancy degradation and lapse with event-log mutation.
3. Focused AI tests prove lapsed office duties suppress `GoalKind::Patrol` candidates and zero patrol motive.
4. Focused save-format test proves the current format version was bumped to 96 for the persisted component.

### Deferred Proof

1. `cargo test -p worldwake-ai --test golden_ai office_vacancy` landed later in `archive/tickets/S153GOLDGAPSCALE-005.md`.
2. `python3 scripts/golden_inventory.py --write --check-docs` landed later in `archive/tickets/S153GOLDGAPSCALE-005.md` because no golden metadata changed here.
3. Broader `cargo test -p worldwake-ai` / clippy gates remain pre-PR proof; this ticket's final source diff is covered by the focused commands in `## Verification Result`.

### Invariants Result

1. Patrol-gap behavior emerges from concrete office/duty lifecycle state, not a hidden scenario flag.
2. Duty state remains inspectable and causally reconstructable after lapse/degradation.
3. AI reads only lawful belief/local/institutional record paths; no global duty omniscience.
4. Planner/ranking views may derive summaries from duty state but cannot become authoritative truth.

## Test Plan Result

### Focused Tests

1. `crates/worldwake-core/src/patrol.rs` — `office_patrol_duty_roundtrips_through_bincode`.
2. `crates/worldwake-systems/src/patrol.rs` — `office_patrol_duty_degrades_when_vacant_office_misses_renewal`, `degraded_office_patrol_duty_lapses_after_grace_window`.
3. `crates/worldwake-ai/src/candidate_generation.rs` — `lapsed_office_patrol_duty_suppresses_patrol_candidate`.
4. `crates/worldwake-ai/src/ranking.rs` — `lapsed_office_patrol_duty_zeroes_patrol_motive`.
5. `crates/worldwake-sim/src/save_load.rs` — `save_format_version_is_96_after_s153_office_patrol_duty_landing`.

### Commands Run And Deferred

1. Run here: `cargo test -p worldwake-core patrol::tests::office_patrol_duty_roundtrips_through_bincode`
2. Run here: `cargo test -p worldwake-systems office_patrol_duty`
3. Run here: `cargo test -p worldwake-ai lapsed_office_patrol_duty`
4. Run here after save-version rename: `cargo test -p worldwake-sim save_format_version`
5. Landed later in `archive/tickets/S153GOLDGAPSCALE-005.md`: `cargo test -p worldwake-ai --test golden_ai office_vacancy`
6. Landed later in `archive/tickets/S153GOLDGAPSCALE-005.md`: `python3 scripts/golden_inventory.py --write --check-docs`
7. Pre-PR broad gate, not run for this focused substrate closeout: `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-20.

- Added `OfficePatrolDuty` and its lifecycle/provenance types as persisted authoritative state registered through the typed component schema.
- Added patrol-system lifecycle maintenance that degrades/lapses office duties when the issuing office is vacant past renewal/grace windows.
- Exposed office patrol duties through the runtime/planning belief surfaces and snapshot state.
- Updated patrol candidate generation and patrol ranking so lapsed office duties no longer produce a valid patrol obligation.
- Bumped `SAVE_FORMAT_VERSION` to 96 for the persisted component shape.
- Created the successor ticket now archived at `archive/tickets/S153GOLDGAPSCALE-005.md` for the remaining S153 D3 office-vacancy golden and generated-doc refresh.

## Deviations

- The original ticket bundled production substrate and the full office-vacancy golden. This landed the substrate and focused proof only; the golden is now a separate follow-up because it has its own authored scenario, route-outcome, route-danger-learning, determinism, and generated-doc ownership.
- Existing non-office `PatrolRoute` / `PatrolProfile` behavior remains available when an agent has no `OfficePatrolDuty`. The new canonical office-vacancy path is the office-duty component; the follow-up golden must exercise that path rather than removing legacy patrol fixtures in this substrate ticket.

## Verification Result

- Passed `cargo test -p worldwake-core patrol::tests::office_patrol_duty_roundtrips_through_bincode`.
- Passed `cargo test -p worldwake-systems office_patrol_duty`.
- Passed `cargo test -p worldwake-ai lapsed_office_patrol_duty`.
- Passed `cargo test -p worldwake-sim save_format_version`.
