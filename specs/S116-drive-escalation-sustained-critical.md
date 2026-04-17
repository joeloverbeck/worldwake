# S116: Drive Escalation Under Sustained Critical Need

## Summary

Add a per-agent, per-need escalation mechanism that multiplies a need's motive score when the need has remained above its critical threshold for a profile-configured number of ticks. Escalation is runtime state (ticks-above-critical counters), not a weight rebalance. The multiplier grows gradually, is capped, and resets the moment the need falls below critical through the normal physical relief pathway. This turns "wash-cycle starvation" and its siblings — where a need's base utility weight loses motive scoring indefinitely to a higher-weight competitor even while the need sits at 900+ permille — into a recoverable homeostatic loop instead of a permanent equilibrium failure.

## Phase and Status

Phase 8 Adjunct: Survival Baseline Under Contention (post-`survival-contested.ron` report). Status: Draft.

## Crates

- `worldwake-core` — new `DriveEscalationProfile` universal agent component; new `DriveEscalationTracker` runtime-state component
- `worldwake-systems` — new `drive_escalation_system` SystemFn runs between metabolism and planning; reads HomeostaticNeeds + DriveThresholds + DriveEscalationProfile, writes DriveEscalationTracker
- `worldwake-ai` — motive scoring in `goal_ranking` consults `DriveEscalationTracker` and multiplies the per-need motive input by the current escalation multiplier before utility weighting
- `worldwake-sim` — new `EventTag::EscalationBegan` / `EventTag::EscalationEnded`; new `SystemId::DriveEscalation`
- `worldwake-cli` — `AgentDef.drive_escalation_profile` field, `spawn_agent()` always applies with default

## Dependencies

- Soft depends on S110 (Authoritative Decision History Events) for the new event tags, but does not block on it — can land with its own tags directly.
- No hard dependency on S109 or S112.

## Motivating Evidence

From `reports/scenario-analysis-report.md` (archived 2026-04-17 as `scenario-analysis-report-2026-04-17-exploited.md`):

- All 4 agents in `survival-contested.ron` (seed 306006, 1440 ticks) had dirtiness >= 750 permille for 703–901 ticks and all reached max=1000 permille.
- Wash action committed 3–5 times per agent; `relieve_wilderness` committed 22–27 times per agent (each adding +200 permille dirtiness via `wilderness_relief_dirtiness_penalty`).
- Section 7 decision timeline confirmed Wash goal was generated but consistently lost motive scoring to AcquireCommodity(Apple) at the food hub because `dirtiness_weight=625 < hunger_weight=700–750`.
- No agent died. The failure is a chronic equilibrium violation, not a survival collapse — precisely the regime the existing `MAX_CRITICAL_RUN_TICKS=400` golden tolerance was knowingly relaxed to accommodate (see `golden_survival_contested.rs:22-34`).

Root cause per Layer 2 analysis: **Priority Override** — relief is never impossible (affordance present, plan found) but fires at a frequency insufficient to keep the need below threshold. Current motive scoring has no time-dependent component, so a need with lower base utility weight can sit at critical indefinitely while higher-weight needs are actively managed.

## Design Goals

1. A need that stays above its critical threshold for N ticks escalates its motive score smoothly, not as a cliff.
2. Escalation respects per-agent variation (FND-22): different agents have different start thresholds and growth rates.
3. Escalation never reads world state — only the agent's own need values and its own tracker (FND-14).
4. Escalation resets through the normal physical relief pathway (the act of washing, eating, drinking) — not through a numerical clamp (FND-11).
5. Escalation events are visible in the event log for debuggability (FND-29) and in Section 7 decision-trace motive inputs.
6. The mechanism does not change goal ranking when no need is above critical, preserving current survival-baseline and survival-scattered behavior.

## Non-Goals

- Escalation on non-homeostatic drives (enterprise, social, obligation, bounty-posting). Only the 7 homeostatic need kinds.
- Escalation of feasibility ranking or plan-search priority. Escalation is a motive-score multiplier, consumed by goal ranking only.
- Drive dampening (the inverse — reducing motive score for over-indulged needs). Out of scope.
- Cross-agent broadcast of escalation ("everyone else panics when one agent escalates"). Out of scope; FND-7 applies.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Escalation reads the concrete per-need-tick counter; it does not introduce a new "urgency score" that abstracts away the physical state. The counter is authoritative. |
| FND-10 (Outcomes Are Granular) | Relief is already granular (wash reduces dirtiness, eat reduces hunger). Escalation does not change outcome granularity — it changes the priority at which the granular relief is pursued. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | The relief action itself is the physical dampener. Escalation's multiplier is not a clamp; it is a priority shift that routes attention to the physical action that actually changes the need. |
| FND-14 (World State Is Not Belief State) | Escalation input is the agent's own HomeostaticNeeds (agent-owned state) and DriveEscalationTracker (agent-owned state). No world-state reads. |
| FND-20 (Resource-Bounded Practical Reasoning) | Escalation is a small, bounded addition to motive scoring — one multiplier per need, computed from an authoritative per-agent counter. No planner-search changes. |
| FND-22 (Agent Diversity Through Concrete Variation) | `DriveEscalationProfile` is per-agent. Different agents escalate at different rates, preserving population heterogeneity. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | The escalation counter is concrete stored state per agent, reset deterministically by the relief action, with accountable origin (which tick crossed critical, which tick fell back below). |
| FND-26 (Systems Interact Through State) | `drive_escalation_system` reads HomeostaticNeeds (from metabolism) and writes DriveEscalationTracker. Goal ranking in `worldwake-ai` reads DriveEscalationTracker. No system calls another system imperatively. |
| FND-29 (Debuggability Is a Product Feature) | `EscalationBegan` / `EscalationEnded` events emit to the append-only log with tick, agent, need kind, current multiplier. Decision traces surface per-need `escalation_multiplier` alongside the existing `pressure` and `weight`. |
| FND-29A (Causal History Is Authoritative, Append-Only) | Escalation transitions emit events. The history explains why an agent finally chose wash at tick T. |

## Deliverables

### D1: `DriveEscalationProfile` universal agent component

New component in `crates/worldwake-core/src/drive_escalation_profile.rs`:

```rust
pub struct DriveEscalationProfile {
    /// Per-need configuration. Entries missing from the map use `default_per_need`.
    pub per_need: BTreeMap<NeedKind, DriveEscalationParams>,
    pub default_per_need: DriveEscalationParams,
}

pub struct DriveEscalationParams {
    /// How many ticks above the critical threshold before escalation begins.
    pub start_after_ticks: u32,
    /// Multiplier growth per tick after start. Applied as
    /// `multiplier_permille = min(cap, 1000 + (ticks_over_start * growth_per_tick))`.
    /// Example: 10 permille growth = 1% per tick over the start threshold.
    pub growth_per_tick: Permille,
    /// Hard cap on the multiplier. Prevents runaway. E.g., 3000 permille = 3×.
    pub max_multiplier: Permille,
}
```

Universal per the Agent Profile Scenario Contract (CLAUDE.md §5): every agent's motive scoring consults it. `Default` impl gives sensible engine-wide defaults; individual scenarios override per-need as needed.

### D2: `DriveEscalationTracker` runtime component

New component in `crates/worldwake-core/src/drive_escalation_tracker.rs`:

```rust
pub struct DriveEscalationTracker {
    /// Per-need tick counter of how long the need has been above its critical
    /// threshold. Reset to 0 when need falls back below critical.
    pub per_need_ticks_over_critical: BTreeMap<NeedKind, u32>,
}
```

Runtime-generated state, exempt from the Agent Profile Scenario Contract per CLAUDE.md §5 (purely emergent from simulation, like `ActiveGoal` / `WoundList`).

### D3: `drive_escalation_system` SystemFn

New SystemFn in `crates/worldwake-systems/src/drive_escalation.rs`:

```rust
pub fn drive_escalation_system(
    world: &mut World,
    sim: &mut SimulationState,
    tick: Tick,
) -> SystemResult {
    for (agent, needs, thresholds) in world.query_agents_with_needs_and_thresholds() {
        let mut tracker = world.get_component_drive_escalation_tracker(agent)
            .cloned()
            .unwrap_or_default();
        for need_kind in NeedKind::ALL {
            let above = needs.value(need_kind) >= thresholds.critical(need_kind);
            let count = tracker.per_need_ticks_over_critical.entry(need_kind).or_insert(0);
            let was_escalating = is_escalating(*count, &profile, need_kind);
            if above {
                *count = count.saturating_add(1);
            } else {
                *count = 0;
            }
            let now_escalating = is_escalating(*count, &profile, need_kind);
            if now_escalating && !was_escalating {
                sim.emit(EventTag::EscalationBegan { agent, need: need_kind });
            } else if !now_escalating && was_escalating {
                sim.emit(EventTag::EscalationEnded { agent, need: need_kind });
            }
        }
        world.set_component_drive_escalation_tracker(agent, tracker);
    }
    Ok(())
}
```

Registered with `SystemId::DriveEscalation`. Ordering: after `metabolism_system` (so counters reflect this tick's need values) and before `agent_tick` (so planning reads the updated tracker).

### D4: Motive scoring integration in `worldwake-ai`

In `crates/worldwake-ai/src/goal_ranking.rs` (or the live equivalent):

```rust
fn compute_motive_score(
    need_kind: NeedKind,
    pressure: Permille,
    weight: Permille,
    tracker: &DriveEscalationTracker,
    profile: &DriveEscalationProfile,
    thresholds: &DriveThresholds,
) -> u32 {
    let base = pressure.value() as u32 * weight.value() as u32;
    let ticks_over = tracker.per_need_ticks_over_critical.get(&need_kind).copied().unwrap_or(0);
    let multiplier_permille = escalation_multiplier(ticks_over, profile.params_for(need_kind));
    (base * multiplier_permille.value() as u32) / 1000
}
```

Decision-trace enrichment: `MotiveInput` gains a `escalation_multiplier: Permille` field. Section 7 output prints `Hunger(pressure=186, weight=750, multiplier=1500, score=209250, recovery_relevant=true)` when escalation is active.

### D5: Event tags and trace emission

New `EventTag` variants in `worldwake-sim`:

```rust
EventTag::EscalationBegan { agent: EntityId, need: NeedKind, tick: Tick, multiplier_permille: Permille }
EventTag::EscalationEnded { agent: EntityId, need: NeedKind, tick: Tick, duration_ticks: u32 }
```

These hook into the append-only event log (FND-29A). Observer renders them in Section 4 raw event sample and in a new per-agent "Escalation Events" subsection of Section 2.

### D6: Scenario integration

`crates/worldwake-cli/src/scenario/types.rs` gets `AgentDef.drive_escalation_profile: Option<DriveEscalationProfileDef>`. `spawn_agent()` applies with `unwrap_or_default()` (universal per CLAUDE.md §5). Every existing scenario continues to pass because default escalation starts well after critical — 100 ticks above critical — which is the same boundary as the mechanical SUSTAINED_CRITICAL_NEED anomaly.

### D7: Golden coverage

Three new goldens in a new `crates/worldwake-ai/tests/golden_drive_escalation.rs`:

1. **`dirtiness_wash_cycle_under_priority_override`** — 2-agent scenario with `dirtiness_weight=625`, `hunger_weight=750`, `wilderness_relief_dirtiness_penalty=200`, wash at Spring Basin (2 hops from food source). Assert: each agent performs ≥ 4 wash cycles over 800 ticks, and each agent's max consecutive dirtiness-critical run is < 250 ticks.

2. **`escalation_respects_belief_only_planning`** — agent with no belief about any wash-capable facility. Force dirtiness above critical for 400 ticks via wilderness relief. Assert: agent never plans Wash (because no belief supports it — FND-14 guard), yet escalation multiplier grows to its cap without errors. Escalation does not synthesize beliefs.

3. **`escalation_fades_after_relief`** — agent with critical dirtiness performs wash. Assert: within 1 tick of dirtiness falling below critical, `DriveEscalationTracker.per_need_ticks_over_critical[Dirtiness] == 0` and `EscalationEnded` event emitted.

### D8: Retrofit existing survival goldens

After S116 lands, tighten `golden_survival_contested.rs::MAX_CRITICAL_RUN_TICKS` from 400 to 200. The comment at lines 22-34 that documents the deferred wash-starvation dynamic gets replaced with a note referencing S116. Same tightening for `golden_survival_scattered.rs` if applicable.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Escalation state is derived entirely from the agent's own HomeostaticNeeds + DriveThresholds, both of which are agent-owned components. The `drive_escalation_system` reads those components and writes `DriveEscalationTracker` on the same agent. Motive scoring reads the agent's own tracker. No cross-agent, no global-state reads. FND-7 and FND-14 preserved.

2. **Positive-feedback analysis**: Escalation is part of a **negative** (homeostatic) feedback loop: need rises → escalation grows → relief action prioritized → relief performed → need falls → escalation resets. There is no positive-feedback loop introduced. The existing positive-feedback loop (wilderness relief raises dirtiness, which raises further wilderness-relief pressure) is unchanged — escalation interrupts it by elevating wash above food pursuit once dirtiness has accumulated.

3. **Concrete dampeners**: The dampener is the **wash action itself** (or the analogous relief action for other needs). This is a physical world process — the agent travels to a wash-capable facility, harvests or possesses water, and commits the multi-tick wash action, which reduces dirtiness by the normal mechanism. No numerical clamp; the multiplier cap exists to prevent unbounded growth, but equilibrium is reached through physical relief, not through the cap.

4. **Stored state vs. derived read-model**:
   - **Stored state (authoritative)**: `DriveEscalationProfile` (profile component, per agent), `DriveEscalationTracker.per_need_ticks_over_critical` (runtime component, per agent).
   - **Derived (per-tick computation)**: `escalation_multiplier(ticks_over, params)` — a pure function of the stored counter and the profile. Not persisted. Recomputed each planning tick.
   - **Event log**: `EscalationBegan` / `EscalationEnded` — authoritative append-only history.

## SystemFn Integration

New system: `SystemId::DriveEscalation`. Ordering:

1. `metabolism_system` (existing) — updates HomeostaticNeeds
2. **`drive_escalation_system` (new)** — updates DriveEscalationTracker based on updated needs
3. `perception_system` (existing)
4. `agent_tick` / planning (existing) — motive scoring reads DriveEscalationTracker via `DriveEscalationProfile::multiplier_for`

## Component Registration

| Component | Classification | `AgentDef` field | `spawn_agent()` path |
|-----------|----------------|------------------|----------------------|
| `DriveEscalationProfile` | Universal | `drive_escalation_profile: Option<DriveEscalationProfileDef>` | Always applied, `unwrap_or_default()` |
| `DriveEscalationTracker` | Runtime | (not in AgentDef) | Initialized empty by `metabolism_system` first tick |

`DriveEscalationProfileDef` in `scenario/types.rs` mirrors the runtime type but uses `Permille` for all fractional fields. `Default` impl returns engine-wide defaults: 100-tick start delay, 10-permille growth per tick, 3000-permille cap.

## Cross-System Interactions (FND-26)

All interactions are state-mediated:
- `metabolism_system` → writes HomeostaticNeeds → `drive_escalation_system` reads HomeostaticNeeds
- `drive_escalation_system` → writes DriveEscalationTracker → `goal_ranking` (AI) reads DriveEscalationTracker
- `goal_ranking` → produces ranked goals → planning search consumes them (unchanged)
- Relief action handlers → write HomeostaticNeeds (need reduction) → next tick's `drive_escalation_system` observes sub-critical need → resets counter → emits `EscalationEnded`

No system calls another directly. No cross-crate imperative dispatch.

## Risks and Open Questions

1. **Interaction with existing planner margin-based commitment (S74)**: Escalation could thrash commitment if the multiplier changes mid-plan. Mitigation: escalation updates once per tick (not mid-plan); margin-based commitment already tolerates per-tick motive changes.
2. **Interaction with Obligation Satiation (S96)**: Satiation dampens obligation motive; escalation amplifies need motive. The interaction is orthogonal (different goal categories) and composes cleanly.
3. **Default parameters**: 100-tick start delay is tentative. Must be calibrated during implementation against all three survival scenarios to avoid breaking `survival-baseline.ron` and `survival-scattered.ron`. Calibration happens during implementation; if the default produces undesired behavior on those scenarios, tighten the defaults before final landing.
4. **Escalation on `fatigue`**: Fatigue relief is sleep, which is already a commonly-chosen goal. Unlikely to change fatigue behavior meaningfully. Verify during golden runs.

## Verification Plan

1. `cargo test -p worldwake-ai --test golden_drive_escalation` — 3 new goldens pass
2. `cargo test -p worldwake-ai --test golden_survival_contested` — retrofit `MAX_CRITICAL_RUN_TICKS=200` passes
3. `cargo test -p worldwake-ai --test golden_survival_scattered` — still passes
4. `cargo test -p worldwake-ai --test golden_survival_baseline` — still passes
5. `cargo clippy --workspace --all-targets -- -D warnings` — clean
6. Observer re-run of `survival-contested.ron` with `/scenario-analysis` — expected: 0 sustained-critical anomalies, `MAINTENANCE_STARVATION` (if S117 has landed) also absent
