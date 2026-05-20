# S153GOLDGAPSCALE-002: Office-vacancy → patrol-gap golden

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: None (substrate is archived: S140 artifact lifecycle, S59 expectation substrate, S148 portfolio slots, S151 route preferences)

## Problem

S153 D3 calls for a golden proving the vacancy → patrol-gap failure mode that FOUNDATIONS Canonical Scenario F (Office Vacancy → Succession Delay → Patrol Gap → Route Predation) demands from generic institutions: a magistrate dies, the office's legal effects suspend, the patrol expectations go overdue with no successor renewing them, the guards' obligation-duty slot loses its valid patrol candidate, and a bandit exploits the unpatrolled route — none of it from a hidden scenario flag. No golden currently exercises this vacancy→gap chain (existing goldens cover obligation issuance and patrol behavior, not the failure mode). This ticket adds that regression plus its determinism rerun (D6 slice) and falsification comment (D7 slice).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Substrate confirmed against current code: `DeadAt { tick }` is an ECS **component** (`crates/worldwake-core/src/combat.rs:77`), not an event tag. `ArtifactLegalEffect::Suspended { reason, suspended_at }` (`crates/worldwake-core/src/social_artifact.rs:103`). The overdue-detection system `check_overdue_expectations` (`crates/worldwake-systems/src/expectation_check.rs:7`) transitions `ExpectationState::Active → Overdue` when `tick.0 > deadline_tick.0 + grace_ticks` (`:62`) and commits a txn tagged `EventTag::System` + `EventTag::WorldMutation` carrying the updated `ExpectationStore` delta (`:34-35`). **There is no `ExpectationFailure` event tag.** `ExpectationState { Active, Overdue, Resolved, Expired }` and `ExpectationRecord { deadline_tick, grace_ticks, basis, ... }` (`crates/worldwake-core/src/expectation.rs:46`). `SlotKind::ObligationDuty` (`crates/worldwake-core/src/slot_kind.rs:4`). `GoalKind::Patrol { place }` (`crates/worldwake-core/src/goal.rs:161`). `RoutePreferenceEntry.dangerous_traversals` (`crates/worldwake-core/src/route_preference.rs`).
2. Spec reference: `specs/S153-golden-gaps-ai-architecture-scaling.md` D3 (post-reassessment — target module `crates/worldwake-ai/tests/scenarios/office_vacancy.rs`, run via `golden_ai`).
3. Shared boundary under audit: S140 office legal-effect (authoritative artifact lifecycle) + S59 expectation-overdue transition (systems layer) drive S148 portfolio slot dynamics (AI layer). The golden audits the AI slot-dynamics layer reacting to authoritative office-suspension + expectation-overdue state — it modifies neither layer.
4. Live `GoalKind` under test: `GoalKind::Patrol { place }`. **Highest-risk reassessment item — pin the full chain before implementing:** confirm (a) which artifact/component carries the "patrol duty" the `ObligationDuty` slot ranks, (b) how a patrol `Expectation` record is authored with `deadline_tick + grace_ticks`, and (c) whether office suspension, expectation-overdue, or both is the driver that empties the slot's valid patrol candidate. Spec is **test-only** (`Engine Changes: None`).
5. **Divergence stop-condition (precision rule 13):** if reassessment finds the chain is *not* producible by existing systems — e.g., patrol duties are not `Expectation`-backed, or office suspension does not propagate to invalidate the `ObligationDuty` slot's patrol candidate — STOP and surface the substrate gap to the user via the 1-3-1 rule. Do **not** add engine functionality to make the golden pass; that would violate the spec's test-only contract. Classify the gap as a separate substrate ticket.
6. AI-regression layer: golden E2E with full action registries (the chain spans death, artifact lifecycle, the expectation system, and the portfolio), not a needs-only harness.
7. Cumulative arithmetic (precision rule 7): the patrol `Expectation` uses an authored `deadline_tick + grace_ticks` (~200 ticks out). State the concrete deadline so the `Active → Overdue` transition is reachable within the scenario's tick budget and the bandit's traversal window aligns with the post-overdue gap.

## Architecture Check

1. Inline-fixture construction keeps the multi-system chain self-contained and replayable. Asserting on the real `ExpectationStore` delta (`WorldMutation` event) rather than a fictional `ExpectationFailure` tag keeps the proof surface honest against current code.
2. No backward-compatibility shims: net-new test coverage; no production path aliased. The "patrol gap" is read from authoritative office + expectation + slot state, never authored by a flag (FND-23, FND-1).

## Verification Layers

1. Office becomes vacant after the magistrate's death -> authoritative world state (`ArtifactLegalEffect::Suspended`) + event-log delta.
2. Each guard's `ObligationDuty` slot initially ranks the patrol duty -> decision trace (slot-indexed candidate).
3. Each patrol `Expectation` transitions `Active → Overdue` -> event-log delta (the `ExpectationStore` delta on the `WorldMutation`+`System` event).
4. With expectations overdue and no successor, the `ObligationDuty` slot has no valid patrol duty; `EconomicOpportunity`/`SocialMotive` can win -> decision trace (slot dynamics).
5. The bandit traverses the unpatrolled route with no guard interception -> action trace / event-log (route-traversal event, absence of a guard intercept action).
6. A traveling merchant records the dangerous traversal -> authoritative core state (`RoutePreferenceEntry.dangerous_traversals`).
7. Determinism (D6): two same-seed runs produce a byte-identical event log AND an equal `ScenarioDiagnosticsReport`.

## What to Change

### 1. New golden module `office_vacancy.rs`

Inline fixture: a town with a magistrate office and 2 guards holding patrol duties (each backed by an S59 `Expectation` with an authored `deadline_tick ~200` + `grace_ticks`), plus a bandit on a route and a traveling merchant. Kill the magistrate (`DeadAt`), advance ticks, and assert the six-step chain (D3 assertions 1–6) per the Verification Layers above. Pin the chain mechanics per Assumption Reassessment items 4–5 before writing assertions.

### 2. Register the module and add the falsification comment

Add `pub mod office_vacancy;` to `tests/scenarios/mod.rs`. Add a `// Falsification:` comment block (D7): e.g., "If a guard keeps patrolling after its patrol `Expectation` goes `Overdue` with no successor renewing it, the office-vacancy → patrol-gap chain is not emerging from world state."

### 3. Determinism rerun (D6)

Run twice at the same seed; assert byte-identical event log and equal `ScenarioDiagnosticsReport`.

### 4. Regenerate golden-inventory docs

Run `python3 scripts/golden_inventory.py --write --check-docs` and commit the regenerated inventory.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `pub mod office_vacancy;`)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerate)
- `docs/generated/golden-scenario-index.md` (modify — regenerate)
- `To be confirmed:` `docs/generated/golden-scenario-details/<office_vacancy>.md` (regenerate output path created by `scripts/golden_inventory.py`; confirm exact filename after running the generator)

## Out of Scope

- No production code changes — test-only. If the vacancy→gap chain is not producible by existing systems, STOP per Assumption Reassessment item 5; do not add engine functionality here.
- No committed RON scenario file (inline fixture); RON backing is optional.
- The false-rumor-justice (D2 → ticket 001) and scaled-contention (D4 → ticket 003) goldens.
- No new golden-harness helper (this scenario uses neither D5 helper).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_office_vacancy_*` passes, asserting: office `Suspended` after magistrate death; `ObligationDuty` slot initially ranks patrol; patrol `Expectation`s transition to `Overdue` (asserted on the `ExpectationStore` delta of the `WorldMutation`+`System` event, **not** a fictional `ExpectationFailure` tag); slot loses its valid patrol candidate; bandit traverses unpatrolled route with no interception; merchant records `dangerous_traversals`.
2. Determinism: two same-seed runs produce a byte-identical event log and an equal `ScenarioDiagnosticsReport`.
3. Existing suite: `cargo test -p worldwake-ai --test golden_ai`
4. Golden-inventory consistency: `python3 scripts/golden_inventory.py --check-docs`

### Invariants

1. The patrol gap emerges from authoritative office + expectation + slot state — never from a hidden scenario flag (FND-1, FND-23).
2. Guards revise office-dependent commitments when the backing expectation's assumptions are invalidated (FND-21).
3. Determinism: byte-stable replay under `ChaCha8Rng` + `BTreeMap`-ordered authoritative state (CLAUDE.md Critical Invariants).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/office_vacancy.rs` — new golden proving the office-vacancy → patrol-gap chain.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai office_vacancy`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `scripts/verify.sh`
