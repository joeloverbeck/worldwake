# S175: Fatigue Collapse and Failed-Rest Traceability

## Summary

`MetabolismProfile.exhaustion_collapse_ticks` (`crates/worldwake-core/src/needs.rs:160`) names a per-agent fatigue critical-exposure threshold ("ticks at critical fatigue before the agent collapses") — but the field has no live consumer. `apply_deprivation_consequences` in `crates/worldwake-systems/src/needs.rs:387` applies wound creation for `hunger_critical_ticks` (Starvation) and `thirst_critical_ticks` (Dehydration), and accident state for `bladder_critical_ticks`, but has no branch for `fatigue_critical_ticks`. `DeprivationExposure.fatigue_critical_ticks` is incremented by the needs tick but never read. Fatigue is therefore an unbounded loop: an agent can spend any number of ticks at critical fatigue with no concrete consequence, the report's "exhaustion collapse" never happens, and S174's failed-rest carrier has no terminal failure path. This spec wires the unimplemented `exhaustion_collapse_ticks` into a concrete deprivation-wound and `DeathCause::NeedDeprivation { need: Fatigue }` chain, matching the existing starvation/dehydration pattern. The paired S174 spec supplies the failed-rest-opportunity records the collapse trace reads to expose the causal chain end-to-end.

This spec is intentionally narrow: it implements the consequence path for an already-named profile field, plus the matching forensic record. It does NOT introduce new sleep mechanics (S174's responsibility) or new wound categories beyond extending `DeprivationKind`.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (`DeprivationKind::Exhaustion` variant; no new components)
- `worldwake-systems` (extend `apply_deprivation_consequences` with fatigue branch; extend `WoundCause::Deprivation` consumers)
- `worldwake-ai` (extend `FailedRestOpportunity` consumer in `SurvivalForensicExtractor` to mark fatigue critical windows that terminated in collapse)
- `worldwake-cli` (scenario for collapse golden)

## Dependencies

- `archive/specs/S17-wound-lifecycle-golden-suites.md` — provides the `Wound` / `WoundList` / `WoundCause` substrate the new variant extends.
- `archive/specs/S81-golden-gaps-simulation-remediation.md` — provides the `DeathCause::NeedDeprivation` death path the new variant terminates into. The S81 substrate currently fires for `NeedDeprivation { need: Hunger }` and `Thirst`; this spec adds `Fatigue`.
- `archive/specs/S120-survival-critical-window-forensics.md` — provides `CriticalWindowReport` / `CriticalWindowFrame` the collapse path is recorded into.
- `archive/specs/S174-shelter-sleep-surfaces-safe-rest.md` — provides `FailedRestOpportunity` records the collapse-trace reads. S175 reads but does not modify S174's forensic state.

## Design Goals

- `exhaustion_collapse_ticks` becomes a live profile parameter with a concrete consequence path equivalent to `starvation_tolerance_ticks` and `dehydration_tolerance_ticks`.
- `DeprivationKind::Exhaustion` joins `Starvation` and `Dehydration` as a wound cause. The existing wound severity ladder (S17) carries it; eventual wound-load death fires `DeathCause::NeedDeprivation { need: Fatigue }`.
- The end-to-end causal chain — accumulated `fatigue_critical_ticks` → `Exhaustion` wound creation → wound load exceeds capacity → death — is provable from authoritative state and event log, exactly as the existing hunger-deprivation chain is provable.
- Failed-rest-opportunity records from S174 thread through the critical-window report so a future reader can answer "this agent collapsed from exhaustion because it failed to rest N times for these specific reasons."
- The forensic chain answers Cluster 1's hardest question: "why did this agent collapse from fatigue when sleep was available somewhere?"

## Non-Goals

- No new sleep mechanics. All sleep / rest-site / wake-reason work is in `archive/specs/S174-shelter-sleep-surfaces-safe-rest.md`.
- No new wound category beyond `DeprivationKind::Exhaustion`. `WoundCause::Deprivation(Exhaustion)` consumes the existing severity/load/death substrate from S17.
- No "rescue" / "carry-to-safety" actions. If a future spec wants other agents to revive collapsed actors, that is its own carrier; this spec defines collapse as a need-deprivation death by default, identical to starvation/dehydration semantics.
- No incapacitation-without-death intermediate state. The existing wound system already handles wound-load-driven incapacitation gracefully; `Exhaustion` wounds participate in that without a new component. The spec does not introduce an `Incapacitated` flag.
- No bladder-equivalent "accident" branch. Bladder has a non-wound accident path (creating waste at the agent's place); fatigue has a wound path matching starvation/dehydration. The two deprivation patterns are distinct; this spec aligns fatigue with the wound pattern.
- No backward-compatibility shim around the unimplemented field. The field becomes live; tests that depended on fatigue having no consequence are updated.
- No reuse of `DeathCause::CombatWounds` for exhaustion-driven wound-load death. The death cause is `NeedDeprivation { need: Fatigue }`, matching the existing `Hunger`/`Thirst` chain.
- No HTN method. No planner change. S175 is a needs-system + wound-system spec only.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-1 (Emergence) | Collapse emerges from sustained fatigue critical exposure produced by ordinary world processes (failed-rest opportunities from S174, repeated unsafe rest, contested shelters); no scripted "you collapsed" event |
| FND-3 (Concrete state) | `DeprivationKind::Exhaustion` wounds are concrete world state with stable identity and lifecycle; no `exhaustion_score` |
| FND-4 (Persistent identity) | Exhaustion wounds participate in the existing `WoundId` identity system; survive replay |
| FND-8 (Aftermath has duration / cost) | Exhaustion wounds reduce capacity (existing wound-load substrate); death is a granular outcome, not a flag flip |
| FND-10 (Aftermath leaves state) | Collapse leaves a death event, a corpse, a wound list snapshot, and a critical-window report — every consequence is inspectable |
| FND-11 (Positive feedback / dampener) | Repeated rest failure → rising fatigue critical exposure → exhaustion wounds → eventual death; collapse-by-death is the terminal dampener |
| FND-19 (Agent symmetry) | Identical exhaustion-collapse semantics for human and AI agents |
| FND-26 (Systems via state) | Needs system reads `DeprivationExposure.fatigue_critical_ticks` + `MetabolismProfile.exhaustion_collapse_ticks`, writes `WoundList`; wound system reads `WoundList`, writes `Death`; no system commands another |
| FND-28 (No backcompat) | `exhaustion_collapse_ticks` becomes live; no parallel "old fatigue path"; tests that previously assumed no fatigue consequence are updated |
| FND-29 (Debuggability) | "Why did this agent collapse?" answerable from `DeprivationExposure` history, `WoundList` snapshot, and `CriticalWindowReport.failed_rest_opportunities` chain |
| FND-29A (Causal history) | Append-only `EventTag::Death` + wound creation events + critical-window-report aggregation |
| FND-31 (Validation) | Scenarios prove (a) repeated-failed-rest → exhaustion wounds → death chain, (b) recovery is possible if rest becomes available before terminal wound load, (c) `exhaustion_collapse_ticks` is a working profile parameter |

## Deliverables

### D1. `DeprivationKind::Exhaustion` variant

```rust
// crates/worldwake-core/src/wounds.rs:30
pub enum DeprivationKind {
    Starvation,
    Dehydration,
    Exhaustion,   // new
}
```

The variant participates in:
- `WoundCause::Deprivation(DeprivationKind::Exhaustion)` (the existing `WoundCause` variant).
- `WoundList::find_deprivation_wound(DeprivationKind::Exhaustion)` (the existing accessor).
- `worsen_or_create_deprivation_wound` (existing helper in `worldwake-systems/src/needs.rs`) — no signature change; the helper takes a `DeprivationKind` and a needs reading.

No new wound severity ladder, no new wound-load contribution function, no new death cause. The variant slots into the existing pattern as a third equal sibling.

### D2. Wire `exhaustion_collapse_ticks` into `apply_deprivation_consequences`

Extend `apply_deprivation_consequences` in `crates/worldwake-systems/src/needs.rs:387` (the function S173 cited as the authoritative consequence path) with a fatigue branch matching the existing starvation/dehydration shape:

```rust
if exposure.fatigue_critical_ticks >= profile.exhaustion_collapse_ticks.get() {
    worsen_or_create_deprivation_wound(
        &mut wound_list,
        world.get_component_wound_list(entity),
        DeprivationKind::Exhaustion,
        needs.fatigue,
        tick,
    );
    exposure.fatigue_critical_ticks = 0;
    wounds_changed = true;
}
```

The reset-on-wound-creation pattern (`exposure.fatigue_critical_ticks = 0`) mirrors the starvation/dehydration code paths (`needs.rs:407, 420`). Each interval of sustained fatigue critical exposure creates or worsens one Exhaustion wound, then the counter restarts; this preserves the existing "wound severity grows with repeated tolerance crossings" semantics.

### D3. Recovery clears the fatigue critical-exposure counter

The existing `needs.rs` tick path already resets `fatigue_critical_ticks` to zero when fatigue drops below critical (this is the pre-existing reset semantics for all five critical-tick counters). No code change is required for recovery; rest naturally clears the counter via the existing path.

The reset semantics imply: an agent that rests *just enough* to drop fatigue below critical resets the collapse timer. Repeated short partial recoveries between failed-rest opportunities can keep the agent alive indefinitely — exactly the design intent the report describes ("each partial sleep does reduce fatigue, so repeated short sleeps cumulatively help"). Collapse fires only when the agent cannot recover even briefly across `exhaustion_collapse_ticks`.

### D4. Death from exhaustion-wound load

No new code path is required. The existing wound-load → death system (per S17/S81) computes total wound load across `WoundList`, including all `WoundCause::Deprivation(_)` wounds regardless of `DeprivationKind` variant. When total load exceeds the agent's capacity, the existing death code at `crates/worldwake-core/src/combat.rs:188` fires `DeathCause::NeedDeprivation { need: <need_id> }`. The `need_id` is computed from the dominant deprivation wound: this spec adds the case where `DeprivationKind::Exhaustion` is the dominant contributor, in which case `need: HomeostaticNeedId::Fatigue` is the recorded cause.

The dominant-deprivation lookup is the small mapping change required: `crates/worldwake-core/src/combat.rs:175-188` (death cause attribution) must map `DeprivationKind::Exhaustion` to `HomeostaticNeedId::Fatigue` in the existing match. This is a single new match arm.

### D5. `CriticalWindowReport` exposes `exhaustion_collapse_observed` flag

`CriticalWindowReport` (`crates/worldwake-ai/src/survival_forensics.rs:19`) gains a new field:

```rust
pub exhaustion_collapse_observed: bool,
```

Set to `true` when the critical window ends with an Exhaustion wound creation event for the focal agent, OR when the focal agent dies with `DeathCause::NeedDeprivation { need: Fatigue }` during the window. The flag is the downstream-facing signal that a forensic reader can use to identify exhaustion-collapse critical windows without iterating wound events directly.

This is a derived view; the authoritative state is the wound list + death event. The flag exists for golden-test ergonomics and CLI surfacing — both consumers of S174's `FailedRestOpportunity` records can pair them with this flag to prove the end-to-end chain.

### D6. Scenario contract: `exhaustion_collapse_ticks` is scenario-overridable

`MetabolismProfile.exhaustion_collapse_ticks` is already a profile field; it is already scenario-overridable via `MetabolismProfileDef`. No new contract surface is required. The scenario golden in D7 uses a low override (e.g., `nz(60)`) to make collapse reachable inside a tractable simulation horizon.

### D7. Scenarios

#### Scenario A — Exhaustion Collapse Cascade (`survival-exhaustion-collapse.ron`)

Topology: one place with no `RestCapacity` (rough-sleep only) and a hostile agent that periodically interrupts sleep, ensuring every Sleep attempt aborts via `SleepFailureCause::HostileProximity`.

Agent: one tired agent with `exhaustion_collapse_ticks` lowered (e.g., `nz(60)`).

Assertions:
1. Agent accumulates `DeprivationExposure.fatigue_critical_ticks` over repeated interrupted rough-sleep attempts.
2. At `fatigue_critical_ticks >= 60`, an `Exhaustion` wound is created in the agent's `WoundList` (`WoundCause::Deprivation(DeprivationKind::Exhaustion)`).
3. `DeprivationExposure.fatigue_critical_ticks` resets to 0 immediately after wound creation.
4. Continued failure → second Exhaustion wound (or severity escalation of the existing wound, per S17 worsening rules).
5. Wound load eventually exceeds agent capacity; `EventTag::Death` fires with `DeathCause::NeedDeprivation { need: HomeostaticNeedId::Fatigue }`.
6. No post-death actions start (existing invariant from S81).
7. `CriticalWindowReport.exhaustion_collapse_observed == true` for the window containing the collapse.
8. `CriticalWindowReport.failed_rest_opportunities` (from S174 D8) lists every interrupted-sleep event leading to collapse, with `SleepFailureCause::HostileProximity` for each.
9. Deterministic replay: identical wound creation tick, identical death tick, identical recorded failed-rest opportunities.

#### Scenario B — Recovery Before Collapse (`survival-exhaustion-recovery.ron`)

Topology: hostile-interrupted shelter at place X; safe place Y reachable via short travel.

Agent: tired agent that fails rest at X (one or more times), then travels to Y, then sleeps successfully.

Assertions:
1. Agent accumulates `fatigue_critical_ticks` during failed rest attempts at X.
2. Agent reaches Y, completes a sleep that drops fatigue below critical.
3. `fatigue_critical_ticks` resets to 0.
4. No `Exhaustion` wound is created.
5. `CriticalWindowReport.exhaustion_collapse_observed == false`.
6. `CriticalWindowReport.failed_rest_opportunities` lists the failed attempts at X, but the window ends in successful rest.
7. Deterministic replay.

This scenario proves the dampener — repeated partial recovery genuinely prevents collapse, so the collapse path is not a one-way death spiral.

#### Scenario C — Profile Field Liveness (focused unit-style test)

A focused test (not a full scenario) verifies that:
1. With `exhaustion_collapse_ticks: nz(60)`, an agent at sustained critical fatigue creates an Exhaustion wound at tick 60.
2. With `exhaustion_collapse_ticks: nz(120)`, the same agent creates the wound at tick 120.
3. The field is read at each tick's `apply_deprivation_consequences` call, not cached at agent spawn.

## Authoritative-to-AI Impact Analysis

This spec changes a needs-system consequence path. Per CLAUDE.md's 7-point Authoritative-to-AI Impact Rule:

1. `get_affordances` — N/A: no change to action affordances.
2. `generate_candidates` — N/A: no change to candidate generation.
3. `search_plan` — N/A: no planner change.
4. `BestEffort` action start — **flag**: an agent with an Exhaustion wound has reduced capacity per the existing wound-load substrate; the same precondition checks that apply to other deprivation-wound holders apply to Exhaustion-wound holders without modification.
5. `handle_plan_failure` — N/A.
6. Payload revalidation — N/A.
7. Golden tests — **flag**: Scenarios A and B (above) plus the focused test (Scenario C) gate the spec. All goldens that exercise long-fatigue scenarios must be reviewed for assumption drift (any test that relied on fatigue having no consequence past critical exposure will need to either adjust its horizon or set `exhaustion_collapse_ticks` high enough to remain unaffected).

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Without a fatigue collapse path, repeated rest failure has no terminal outcome. The S174 `FailedRestOpportunity` chain dead-ends; "this agent failed to rest 50 times and then... nothing" is the current state. Collapse closes the loop and is what makes failed-rest forensically meaningful.

2. **New entities/relations/records**:
   - `DeprivationKind::Exhaustion` enum variant.
   - `CriticalWindowReport.exhaustion_collapse_observed: bool` field.
   - No new components.
   - No new event-tag variants. `EventTag::Death` with `DeathCause::NeedDeprivation { need: Fatigue }` is the death record; the existing wound-creation event is the wound record.

3. **Actions that mutate them**:
   - `apply_deprivation_consequences` (needs.rs tick) writes Exhaustion wounds when threshold crossed (D2).
   - Existing wound-load → death code in combat.rs writes Death event when wound load exceeds capacity (D4); only the death-cause-attribution match is extended.
   - Existing fatigue-recovery reset semantics clear the exposure counter (D3).

4. **Information production and travel**: Wound creation and death events are local to the agent. They appear in the authoritative event log and are observable by co-located agents via ordinary perception. The forensic record `exhaustion_collapse_observed` is a derived view over the event log.

5. **Conserved quantities**: No new conserved resource. Wound load is the existing accumulator per S17.

6. **Scarce capacities and contention**: None new.

7. **Partial failures and aftermath**: 
   - Sustained critical fatigue → first Exhaustion wound (mild severity).
   - Continued sustained critical fatigue → wound worsening (S17 ladder).
   - Wound load capacity exceeded → death.
   - Recovery before threshold crossed → counter resets; no wound; agent continues.

8. **Positive feedback loops**:
   - Repeated rest failure → rising fatigue critical exposure → first Exhaustion wound → wound reduces capacity → may interfere with future rest opportunities → more failed rests. The dampener is point 9.

9. **Physical dampeners**:
   - Rough-sleep recovery floor from S174: rough sleep is always available and always partially restorative.
   - Death: wound load exceeds capacity → terminal end of the loop. This is the same dampener that bounds starvation and dehydration.
   - `exhaustion_collapse_ticks` is a profile parameter, so per-agent variation (per FND-22) governs how fast the collapse fires.

10. **Agent learning**: None added by this spec.

11. **How agents can be wrong**: Agents with stale rest-site beliefs may continue to fail rest. That belief mismatch is the S174 design surface; this spec just terminates the chain when the mismatch persists.

12. **Lifecycle states**:
    - Exhaustion wound: created → worsened → healed (existing wound lifecycle from S17) → cleared from list on heal.
    - `exhaustion_collapse_observed`: window-scoped boolean; persists with the critical window report.

13. **Temporal resolution**: Wound creation and exposure-counter reset happen in `apply_deprivation_consequences` at the needs-tick boundary. Death from wound load happens at the wound-system tick boundary. No simultaneity concerns beyond the existing needs/wound tick ordering.

14. **Boundary conditions**: Not applicable — fatigue collapse is local.

15. **Derived views**: `CriticalWindowReport.exhaustion_collapse_observed` is derived from wound creation events + death events. The authoritative state is the wound list + event log.

16. **Causal records**:
    - Existing wound-creation event records when an Exhaustion wound was created.
    - Existing `EventTag::Death` records the cause with `DeathCause::NeedDeprivation { need: Fatigue }`.
    - `CriticalWindowReport.failed_rest_opportunities` (from S174) records the upstream failed-rest chain.
    - Combined chain: failed-rest opportunities → fatigue critical exposure → exhaustion wound creation → wound load → death — all inspectable from authoritative state.

17. **Target patterns**:
    - Repeated unsafe rough sleep → exhaustion wound → second wound → death.
    - Repeated interrupted shelter sleep with no recovery between → exhaustion wound → death.
    - Failed rest followed by a successful recovery → no wound (the dampener works).

18. **Save/load and replay**: All new state is enum-variant extension or boolean field on existing types. Replay-deterministic. No new save-format consideration.

## Stored State vs. Derived Read-Model List

| Type | Classification | Authority |
|------|----------------|-----------|
| `DeprivationKind::Exhaustion` | Stored value (inside `WoundCause::Deprivation(_)`) | Authoritative on wound creation |
| Existing `Wound` / `WoundList` / `WoundCause` substrate | Stored authoritative state | S17 substrate; unchanged |
| `DeathCause::NeedDeprivation { need: Fatigue }` | Stored authoritative event payload | Existing death-event payload; new attribution case |
| `CriticalWindowReport.exhaustion_collapse_observed` | Derived view over wound and death events | View; not authoritative |
| `DeprivationExposure.fatigue_critical_ticks` (existing) | Stored authoritative needs-state counter | Pre-existing; now read by D2 |
| `MetabolismProfile.exhaustion_collapse_ticks` (existing) | Stored authoritative profile parameter | Pre-existing; now read by D2 |

## Planner-formalism analysis

No planner change. Sleep candidate generation is owned by S174; this spec changes only the consequence side of the needs system. Goal schema, candidate emission, and search semantics are untouched.

## Belief-View Accessor Source-Class Declarations

No new accessors. The fatigue critical exposure counter and Exhaustion wound creation are authoritative state; agents observe wounds on themselves via self-knowledge (per the existing wound-perception path). Other agents observe co-located wound state per FND-14A.

## Agent Profile Scenario Contract

`MetabolismProfile.exhaustion_collapse_ticks` is already a scenario-authorable profile field. No new contract surface required. Scenarios in D7 use this field's authoring to set realistic collapse horizons.

## Component Registration

No new components. No registration changes.

## Cross-System Interactions (FND-26)

| System | Read | Write |
|--------|------|-------|
| Needs system (`worldwake-systems::needs::apply_deprivation_consequences`) | `DeprivationExposure.fatigue_critical_ticks`, `MetabolismProfile.exhaustion_collapse_ticks`, `HomeostaticNeeds.fatigue`, `WoundList` | `WoundList`, `DeprivationExposure.fatigue_critical_ticks` (reset) |
| Combat / death system (`worldwake-core::combat`) | `WoundList`, `DeprivationKind` (for cause attribution) | `EventTag::Death` payload with `DeathCause::NeedDeprivation { need: Fatigue }` |
| Survival forensics (`worldwake-ai::survival_forensics`) | Wound creation events, death events, S174 `FailedRestOpportunity` records | `CriticalWindowReport.exhaustion_collapse_observed` |

No system commands another. All interaction is via authoritative state, event log, and forensic-extractor read paths.

## Open Questions

1. Should the `exhaustion_collapse_observed` flag also fire for incapacitation-without-death (e.g., an agent rendered unable to act by wound load but not yet dead)? The current design ties the flag to either wound creation OR death. If S174 / S175 ticket-time review reveals incapacitation as a distinct meaningful state worth flagging separately, a sibling flag is added then.
2. Should the dominant-deprivation attribution at death break ties deterministically when multiple deprivation wounds contribute equally? The existing code in `combat.rs:175-188` already has a tie-break (last-wound-wins or highest-severity-wins; verify at ticket time). If `Exhaustion` ties with `Starvation` and `Dehydration` exactly, the tie-break behavior is unchanged from current — fatigue is just a third equal player. If a future iteration wants `Exhaustion` to attribute differently (e.g., "exhaustion contributed to collapse but agent died of starvation"), that is a separate refinement spec.
