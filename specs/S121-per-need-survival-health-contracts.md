# S121: Per-Need Survival Health Contracts

## Summary

Extend the scenario-authored `survival_health_contract` introduced by S119 so survival goldens can express different authored-critical run envelopes per need instead of one coarse global cap. This is required for scenarios like `survival-contested.ron`, where the authored self-care contract intentionally does not require `Wash`, but the current all-needs `max_authored_critical_run_ticks` still applies an equally strict dirtiness bound and therefore overstates what the authored scenario is claiming to prove.

## Phase and Status

Phase 7 Adjunct: Survival Stability Hardening. Status: Draft.

## Crates

- `worldwake-cli` — scenario schema support for richer authored survival-health contracts
- `worldwake-ai` — survival golden harness helpers consume the richer contract
- `worldwake-core` — no changes
- `worldwake-sim` — no changes
- `worldwake-systems` — no changes

## Dependencies

- `archive/specs/S119-authored-survival-health-contracts.md` (completed 2026-04-18)

## Motivating Evidence

After `S119AUTHSURVHC-001` retrofitted the survival goldens to read authored scenario contracts instead of file-local constants, `golden_survival_contested::all_agents_survive_1440_ticks` still failed on April 18, 2026 with:

- `Agent A dirtiness exceeded authored critical pm(900) for 1167 consecutive ticks (max allowed: 400)`

At the same time, the authored contested scenario intentionally declares:

- `required_self_care_families: [Eat, Drink, Sleep, Relieve]`

That combination reveals a contract-model gap, not just a bad number:

1. The scenario explicitly does not claim that every agent must wash.
2. The current survival-health contract still applies one uniform `max_authored_critical_run_ticks` limit to hunger, thirst, fatigue, bladder, and dirtiness.
3. Raising that one global cap enough to admit lawful contested dirtiness would also weaken hunger/thirst/fatigue falsification, which is architecturally wrong.

The contract needs more expressive power so scenario-authored truth can remain concrete without flattening unlike needs into one coarse envelope.

## Design Goals

1. Survival scenarios can author different maximum authored-critical run bounds per need.
2. Existing scenario contracts remain concrete and scenario-local; no CI-only override file or second truth path.
3. Goldens falsify only the needs the scenario explicitly claims to bound at a given severity.
4. The contract stays deterministic and human-readable in scenario RON.

## Non-Goals

- Changing live survival behavior.
- Auto-deriving per-need bounds from traces.
- Weakening existing scenarios by default.
- Replacing focused lower-layer diagnostics or S120-style forensics.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-1 (No Magic Numbers) | Per-need survival bounds are still authored explicitly, not buried in test-local constants or ad hoc exceptions. |
| FND-3 (Concrete State Over Abstract Scores) | The contract speaks in concrete needs (`hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness`) rather than a vague aggregate health score. |
| FND-22 (Agent Diversity Through Concrete Variation) | Different scenarios can express different need tolerances without flattening all survival behavior into one global envelope. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | Golden helpers remain derived readers of authored scenario truth rather than inventing a second contract. |
| FND-31 (Validation and Falsification Are First-Class) | Goldens can now falsify the exact need-specific envelope the scenario claims, instead of over- or under-asserting unlike needs. |

## Deliverables

### D1: Per-need authored critical-run contract

Extend the S119 contract surface with a per-need limit structure, for example:

```rust
pub struct SurvivalCriticalRunLimitsDef {
    pub hunger: Option<u32>,
    pub thirst: Option<u32>,
    pub fatigue: Option<u32>,
    pub bladder: Option<u32>,
    pub dirtiness: Option<u32>,
}

pub struct SurvivalHealthContractDef {
    pub max_authored_critical_run_ticks: u32,
    pub max_idle_window_ticks_with_elevated_need: u32,
    pub elevated_need_floor: Permille,
    pub required_self_care_families: Vec<NeedsActionFamily>,
    pub critical_run_limits: Option<SurvivalCriticalRunLimitsDef>,
}
```

Semantics:

- `max_authored_critical_run_ticks` remains the scenario-wide default.
- `critical_run_limits.<need>` overrides the default only for that need.

### D2: Shared helper support

Update the shared survival-golden helpers so authored-critical assertions:

1. read the per-need override when present
2. otherwise fall back to the scenario-wide default
3. continue to compare against each agent's authored `DriveThresholds`

### D3: Retrofit contested contract truthfully

Retrofit `scenarios/survival-contested.ron` and `golden_survival_contested.rs` to use the richer contract so contested no longer over-asserts dirtiness if wash is not part of the authored self-care claim.

### D4: Documentation

Update `docs/golden-e2e-testing.md` so survival-health contracts explicitly support per-need authored-critical bounds when one scenario's lawful self-care envelope differs across need families.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: the richer contract still flows only from scenario-authored input to test/read tooling.
2. **Positive-feedback analysis**: none; static authored input cannot feed back into runtime behavior.
3. **Concrete dampeners**: per-need authored-critical limits are explicit falsification dampeners on what the scenario claims is healthy.
4. **Stored state vs. derived read-model**:
   - **Stored/authored**: per-need survival contract in scenario input
   - **Derived**: golden run trackers, need-run summaries, idle-window summaries

## SystemFn Integration

None.

## Component Registration

None.

## Cross-System Interactions

- `worldwake-cli` loads the richer authored contract.
- `worldwake-ai` goldens consume it through the shared harness.
- No simulation system reads the contract at runtime.

## Validation and Falsification

### Focused tests

1. Loader coverage for `critical_run_limits` deserialization.
2. Shared helper coverage that per-need overrides beat the scenario-wide default for only the targeted need.

### Golden / integration tests

3. `golden_survival_contested.rs` consumes the richer contract and stays green under the authored scenario truth.

## Outcome

To be filled in at completion.
