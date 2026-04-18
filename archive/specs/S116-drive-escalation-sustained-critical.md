# S116: Drive Escalation Under Sustained Critical Need

**Status**: COMPLETED

## Summary

Add a per-agent, profile-configured motive-score multiplier that grows with the number of consecutive ticks a homeostatic need has spent above its critical threshold. Escalation is **not** new stored state: it is a read-time function of the existing `DeprivationExposure` counter (extended to cover dirtiness) and a new `DriveEscalationProfile` universal agent component. The multiplier grows gradually, is capped, and falls back to `1×` the moment the need drops below critical through the normal physical relief pathway. This turns "wash-cycle starvation" and its siblings — where a need's base utility weight loses motive scoring indefinitely to a higher-weight competitor even while the need sits at 900+ permille — into a recoverable homeostatic loop instead of a permanent equilibrium failure.

## Phase and Status

Phase 8 Adjunct: Survival Baseline Under Contention (post-`survival-contested.ron` report). Status: Draft.

## Crates

- `worldwake-core` — new `DriveEscalationProfile` universal agent component; extension of existing `DeprivationExposure` with `dirtiness_critical_ticks`; new `EventTag::Escalation`; new `HomeostaticNeedId::ALL` constant
- `worldwake-systems` — existing `needs_system` extended to (a) maintain `dirtiness_critical_ticks`, (b) emit escalation begin/end transitions
- `worldwake-sim` — `GoalBeliefView` / `BeliefView` gain `deprivation_exposure` and `drive_escalation_profile` accessors
- `worldwake-ai` — `ranking.rs` reads exposure + profile through `RankingContext`, applies multiplier in `drive_score` and `relevant_self_consume_factors`; `RankedDriveMotiveInput` surfaces `escalation_multiplier`
- `worldwake-cli` — `AgentDef.drive_escalation_profile` field, `spawn_agent()` always applies with default

## Dependencies

- Soft relationship with S110 (Authoritative Decision History Events): this spec reuses the existing append-only event channel and does not block on S110.
- No hard dependency on S109 or S112.

## Motivating Evidence

From `reports/scenario-analysis-report.md` (survival-contested, seed 306006, 1440 ticks):

- All 4 agents in `survival-contested.ron` had dirtiness ≥ 750 permille for 703–901 ticks and all reached max=1000 permille. Observed maximum consecutive critical runs: 313 / 315 / 319 / 372 ticks.
- Wash action committed 3–5 times per agent; `relieve_wilderness` committed 22–27 times per agent (each adding +200 permille dirtiness via `wilderness_relief_dirtiness_penalty`).
- Section 7 decision timeline confirmed Wash goal was generated but consistently lost motive scoring to `AcquireCommodity(Apple)` at the food hub because `dirtiness_weight=625 < hunger_weight=700–750`.
- No agent died. The failure is a chronic equilibrium violation, not a survival collapse — precisely the regime the existing authored contested survival-health bound of `max_authored_critical_run_ticks: 400` was knowingly relaxed to accommodate before the later S116 tightening.

Root cause per Layer 2 analysis: **Priority Override** — relief is never impossible (affordance present, plan found) but fires at a frequency insufficient to keep the need below threshold. Current motive scoring has no time-dependent component, so a need with lower base utility weight can sit at critical indefinitely while higher-weight needs are actively managed.

Note on residual architectural bottleneck (not solved here): `wash_preconditions` (`needs_actions.rs:196`) requires `TargetDirectlyPossessedByActor(0)` with `CommodityKind::Water`. Under water contention, agents preferentially drink rather than save water for wash. Motive-score escalation cannot on its own override an affordance-level water-possession gap; see D8 for the tightening it does justify and the follow-up this leaves open.

## Design Goals

1. A need that stays above its critical threshold for N ticks escalates its motive score smoothly, not as a cliff.
2. Escalation respects per-agent variation (FND-22): different agents have different start thresholds and growth rates.
3. Escalation never reads world state — only the agent's own `HomeostaticNeeds`, `DriveThresholds`, `DeprivationExposure`, and `DriveEscalationProfile`, all of which are agent-owned components (FND-14).
4. Escalation resets through the normal physical relief pathway (the act of washing, eating, drinking) — not through a numerical clamp (FND-11).
5. Escalation transitions are visible in the authoritative event log (FND-29A) and in Section 7 decision-trace motive inputs (FND-29).
6. The mechanism does not change goal ranking when no need is above critical, preserving current survival-baseline and survival-scattered behavior within a bounded tolerance.
7. No new authoritative representation of "ticks at critical" is introduced: the existing `DeprivationExposure` counter is extended rather than duplicated (FND-28).

## Non-Goals

- Escalation on non-homeostatic drives (enterprise, social, obligation, bounty-posting, pain, danger). Only the 5 homeostatic need kinds tracked by `HomeostaticNeedId`: `Hunger`, `Thirst`, `Fatigue`, `Bladder`, `Dirtiness`.
- Escalation of feasibility ranking or plan-search priority. Escalation is a motive-score multiplier, consumed by goal ranking only.
- Drive dampening (the inverse — reducing motive score for over-indulged needs). Out of scope.
- Cross-agent broadcast of escalation ("everyone else panics when one agent escalates"). Out of scope; FND-7 applies.
- Resolving the wash-water-possession bottleneck described in the Motivating Evidence. Out of scope here; see D8.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Escalation reads the concrete per-need tick counter (`DeprivationExposure.*_critical_ticks`); it does not introduce an "urgency score" that abstracts away the physical state. The counter is authoritative. |
| FND-10 (Outcomes Are Granular) | Relief is already granular (wash reduces dirtiness, eat reduces hunger). Escalation does not change outcome granularity — it changes the priority at which the granular relief is pursued. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | The relief action itself is the physical dampener: traveling to a wash-capable facility, possessing water, and committing the multi-tick wash action all physically reduce dirtiness, which resets the counter, which drops the multiplier to 1×. The `max_multiplier` cap exists only as a defensive upper bound; it is not the dampener and must not fire in healthy operation. |
| FND-14 (World State Is Not Belief State) | Escalation inputs are the agent's own components (`HomeostaticNeeds`, `DriveThresholds`, `DeprivationExposure`, `DriveEscalationProfile`). No world-state reads. |
| FND-20 (Resource-Bounded Practical Reasoning) | Escalation is a small, bounded addition to motive scoring — one multiplier computed per need per ranking pass. No planner-search changes. |
| FND-22 (Agent Diversity Through Concrete Variation) | `DriveEscalationProfile` is per-agent. Different agents escalate at different rates, preserving population heterogeneity. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Weakly applicable: the escalation counter is not *learned* state, but it does have accountable origin (the tick the need crossed critical) and accountable termination (the tick it fell back below). In FND-22A terms it is short-term exposure accounting rather than a learned summary. |
| FND-26 (Systems Interact Through State) | `needs_system` writes `DeprivationExposure`. `ranking.rs` reads `DeprivationExposure` and `DriveEscalationProfile` through `GoalBeliefView`. No cross-system imperative calls; no new SystemFn needed. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | The multiplier itself is a pure read-time computation over the authoritative counter and profile. It is never stored. The counter and profile are the source of truth. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Extending `DeprivationExposure` with `dirtiness_critical_ticks` keeps a single authoritative representation of "consecutive ticks at critical per need". No parallel tracker component is introduced. |
| FND-29 (Debuggability Is a Product Feature) | Decision traces surface per-need `escalation_multiplier` alongside the existing `pressure`, `weight`, and `score` on `RankedDriveMotiveInput`. |
| FND-29A (Causal History Is Authoritative, Append-Only) | Escalation begin/end transitions emit `EventTag::Escalation` events from inside `needs_system`, carrying the need id and multiplier / duration in the structured payload (see D4). History answers "why did this agent finally pick wash at tick T?". |

## Deliverables

### D1: `HomeostaticNeedId::ALL` and extended `DeprivationExposure`

In `crates/worldwake-core/src/needs.rs`:

1. Add `pub const ALL: [Self; 5] = [Self::Hunger, Self::Thirst, Self::Fatigue, Self::Bladder, Self::Dirtiness];` on `HomeostaticNeedId`, consistent with the `CommodityKind::ALL` / `SystemId::ALL` pattern.
2. Extend `DeprivationExposure` with `pub dirtiness_critical_ticks: u32`. Default remains all-zero. All existing `*_critical_ticks` semantics are preserved.
3. Add a keyed accessor on `DeprivationExposure`:
   ```rust
   impl DeprivationExposure {
       pub fn ticks_at_critical(&self, need: HomeostaticNeedId) -> u32 {
           match need {
               HomeostaticNeedId::Hunger => self.hunger_critical_ticks,
               HomeostaticNeedId::Thirst => self.thirst_critical_ticks,
               HomeostaticNeedId::Fatigue => self.fatigue_critical_ticks,
               HomeostaticNeedId::Bladder => self.bladder_critical_ticks,
               HomeostaticNeedId::Dirtiness => self.dirtiness_critical_ticks,
           }
       }
   }
   ```
4. Add a keyed accessor on `HomeostaticNeeds` and `DriveThresholds` for symmetric read in `needs_system` and `ranking.rs`:
   ```rust
   impl HomeostaticNeeds {
       pub fn value(&self, need: HomeostaticNeedId) -> Permille { ... }
   }
   impl DriveThresholds {
       pub fn critical(&self, need: HomeostaticNeedId) -> Permille { ... }
   }
   ```

`DeprivationExposure` remains a `Component` and is already registered in `component_tables.rs`. No new component table entry is required for the counter.

### D2: `DriveEscalationProfile` universal agent component

New component in `crates/worldwake-core/src/drive_escalation_profile.rs`:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DriveEscalationProfile {
    /// Per-need configuration. Entries missing from the map fall back to `default_per_need`.
    pub per_need: BTreeMap<HomeostaticNeedId, DriveEscalationParams>,
    pub default_per_need: DriveEscalationParams,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DriveEscalationParams {
    /// How many ticks above the critical threshold before escalation begins.
    pub start_after_ticks: u32,
    /// Multiplier growth per tick after start, expressed in permille.
    /// Applied as `multiplier_permille = min(cap, 1000 + ticks_over_start * growth_per_tick)`.
    /// Example: `Permille(10)` = 1% growth per tick over the start threshold.
    pub growth_per_tick: Permille,
    /// Hard cap on the multiplier in permille units. E.g.,
    /// `MultiplierPermille(3000)` = 3×.
    /// Defensive upper bound only; physical relief is expected to reset the
    /// counter well before the cap fires.
    pub max_multiplier: MultiplierPermille,
}

impl DriveEscalationProfile {
    pub fn params_for(&self, need: HomeostaticNeedId) -> DriveEscalationParams {
        self.per_need.get(&need).copied().unwrap_or(self.default_per_need)
    }
}
```

Universal per the Agent Profile Scenario Contract (CLAUDE.md §5): every agent's motive scoring consults it. `Default` impl gives engine-wide defaults: `start_after_ticks: 100`, `growth_per_tick: Permille(10)`, `max_multiplier: MultiplierPermille(3000)`. Individual scenarios may override per-need.

Register the component in `component_tables.rs` with `insert_drive_escalation_profile` / `get_drive_escalation_profile` / `has_drive_escalation_profile` / `iter_drive_escalation_profile` following the `DriveThresholds` pattern. Add save/load round-trip coverage.

Add a pure helper in the same module (used by `ranking.rs`):

```rust
pub fn escalation_multiplier(
    ticks_over_critical: u32,
    params: DriveEscalationParams,
) -> MultiplierPermille {
    if ticks_over_critical <= params.start_after_ticks {
        return MultiplierPermille::new_unchecked(1000);
    }
    let over_start = ticks_over_critical - params.start_after_ticks;
    let raw = 1000u32
        .saturating_add(over_start.saturating_mul(u32::from(params.growth_per_tick.value())));
    let capped = raw.min(u32::from(params.max_multiplier.value())).min(u32::from(u16::MAX));
    MultiplierPermille::new_unchecked(capped as u16)
}
```

### D3: `needs_system` extension — maintain dirtiness counter and emit escalation transitions

`crates/worldwake-systems/src/needs.rs` hosts `needs_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>` (SystemFn contract defined at `sim/src/system_dispatch.rs:11-23`). The existing system already increments / resets `hunger_critical_ticks`, `thirst_critical_ticks`, `fatigue_critical_ticks`, `bladder_critical_ticks` against `DriveThresholds::critical`.

Extend the existing path:

1. **Counter maintenance**: After the existing need-update pass, compute `above = needs.dirtiness >= thresholds.dirtiness.critical()` and update `exposure.dirtiness_critical_ticks` with the same increment-on-above / reset-on-below rule already applied to the other four needs. Keep the write inside the same `WorldTxn` so the state delta is appended coherently with the rest of the tick's needs mutation.
2. **Escalation transitions**: For each `HomeostaticNeedId::ALL`, determine:
   - `prev_ticks = previous_exposure.ticks_at_critical(need)`
   - `next_ticks = updated_exposure.ticks_at_critical(need)`
   - `params = profile.params_for(need)`
   - `was_escalating = prev_ticks > params.start_after_ticks`
   - `is_escalating = next_ticks > params.start_after_ticks`
   If `is_escalating && !was_escalating`, emit an escalation-begin event; if `!is_escalating && was_escalating`, emit an escalation-end event. Emission uses the existing `WorldTxn` emission pattern already exercised by `needs_system` (see starvation-wound / death emission): open a dedicated hidden `WorldTxn` for the transition with `CauseRef::SystemTick(tick)`, `actor_id = Some(agent)`, `target_ids = [agent]`, tags `{ EventTag::System, EventTag::Escalation }`, and an empty state-delta vector (the counter mutation is already carried by the needs-update transaction of the same tick). The canonical carrier is `action_name` encoding, using `escalation_begin:{need}:{multiplier}` / `escalation_end:{need}:{duration}`. Section 4 raw-event inspection must be able to recover `(agent, need, transition_kind, multiplier_or_duration)` from that event record without introducing a new typed `StateDelta` variant.

No new SystemFn is added. No entry in `SystemId` or `SystemManifest::canonical()` is added. This keeps per-tick system count unchanged and preserves the causal ordering comment in `sim/src/system_manifest.rs` without modification.

### D4: Motive scoring integration in `worldwake-ai`

In `crates/worldwake-ai/src/ranking.rs`:

1. **Belief-view accessors** (in `crates/worldwake-sim/src/belief_view.rs` + `per_agent_belief_view.rs`):
   ```rust
   fn deprivation_exposure(&self, agent: EntityId) -> Option<DeprivationExposure>;
   fn drive_escalation_profile(&self, agent: EntityId) -> Option<DriveEscalationProfile>;
   ```
   Implement both on `BeliefView`, `PerAgentBeliefView`, and test doubles. The `drive_escalation_profile` accessor returns `Some(default)` for agents that lack an explicit component (universal component, consistent with the `UtilityProfile` pattern).

2. **`RankingContext`** extension (`ranking.rs:342`):
   ```rust
   struct RankingContext<'a> {
       // ... existing fields ...
       exposure: Option<DeprivationExposure>,
       escalation_profile: Option<DriveEscalationProfile>,
   }
   ```
   `RankingContext::new` populates both via the existing `GoalBeliefView` argument at the same call sites that already read `view.homeostatic_needs(agent)` and `view.drive_thresholds(agent)`.

3. **Multiplier application at the two `score_product` call sites**:
   - `drive_score` (ranking.rs:1167): accepts an additional `need: HomeostaticNeedId` discriminant (or an inline match on the closure, whichever minimizes call-site churn). After computing `base = score_product(weight, pressure)`, apply the multiplier:
     ```rust
     let ticks = context.exposure.map(|e| e.ticks_at_critical(need)).unwrap_or(0);
     let params = context.escalation_profile
         .as_ref()
         .map(|p| p.params_for(need))
         .unwrap_or_default();
     let multiplier = escalation_multiplier(ticks, params);
     base.saturating_mul(u32::from(multiplier.value())) / 1000
     ```
   - `relevant_self_consume_factors` (ranking.rs:1234): for each `DriveFactor` it emits, also compute the multiplier for `factor.drive`'s corresponding `HomeostaticNeedId` (map `RankedDriveKind::Hunger → HomeostaticNeedId::Hunger`, etc.) and attach it to the factor so the later `score_product`-then-`saturating_mul` application happens in one place.

4. **Decision trace**: extend `RankedDriveMotiveInput` with `pub escalation_multiplier: MultiplierPermille`. `drive_provenance_from_inputs` populates it. Section 7 trace output gains a `multiplier=` field on drive motive rows: `Hunger(pressure=186, weight=750, multiplier=1500, score=209250, recovery_relevant=true)`.

5. **`score_product` stays pure**: leave `score_product(weight, pressure) -> u32` untouched. The multiplier is applied at the caller so the decision trace keeps `weight × pressure` separate from the escalation factor.

### D5: Event tag

New variant in `crates/worldwake-core/src/event_tag.rs`:

```rust
pub enum EventTag {
    // ... existing variants ...
    Escalation,
}
```

Add to the in-order test fixture `ALL_EVENT_TAGS` and bump the `assert_eq!(ALL_EVENT_TAGS.len(), 27)` assertion. `EventTag::Escalation` is a unit variant, consistent with the existing schema.

Emission happens from `needs_system` as specified in D3.

### D6: Scenario integration

`crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct AgentDef {
    // ... existing fields ...
    #[serde(default)]
    pub drive_escalation_profile: Option<DriveEscalationProfile>,
}
```

`DriveEscalationProfile` has no `EntityId` references, so the component type is directly RON-deserializable without a `*Def` wrapper (following the `DriveThresholds` pattern). In `scenario/mod.rs::spawn_agent()`, call:

```rust
world_txn.set_component_drive_escalation_profile(
    agent_id,
    def.drive_escalation_profile.clone().unwrap_or_default(),
);
```

Universal per CLAUDE.md §5: always applied, `unwrap_or_default()`.

### D7: Golden coverage

Three new goldens in `crates/worldwake-ai/tests/golden_drive_escalation.rs`:

1. **`dirtiness_wash_cycle_under_priority_override`** — 2-agent scenario with `dirtiness_weight=625`, `hunger_weight=750`, `wilderness_relief_dirtiness_penalty=200`, wash at Spring Basin (2 hops from food source), water co-located with the wash facility. Assert: each agent performs ≥ 4 wash cycles over 800 ticks, and each agent's max consecutive dirtiness-critical run is < 250 ticks.

2. **`escalation_respects_belief_only_planning`** — agent with no belief about any wash-capable facility. Force dirtiness above critical for 400 ticks via wilderness relief. Assert: agent never plans Wash (because no belief supports it — FND-14 guard), yet escalation multiplier grows to its cap without errors. Escalation does not synthesize beliefs.

3. **`escalation_fades_after_relief`** — agent with critical dirtiness performs wash. Assert: within 1 tick of dirtiness falling below critical, `DeprivationExposure::ticks_at_critical(HomeostaticNeedId::Dirtiness) == 0` and an `EventTag::Escalation` event with `action_name` indicating escalation-end is present in the event log for the transition tick.

### D8: Retrofit contested authored survival bound

After S116 lands, tighten `survival-contested.ron`'s canonical authored survival-health contract from **400 to 300** by changing `survival_health_contract.max_authored_critical_run_ticks`. Rationale: the current empirical maximum consecutive critical run across the four agents in survival-contested is 372 ticks (Agent D), with the others at 313/315/319. Motive-score escalation is expected to break runs earlier than the current worst case; a target of 300 represents a ≥ 72-tick improvement over today's peak run while leaving headroom for the affordance-level water-possession bottleneck (`wash_preconditions` requires `TargetDirectlyPossessedByActor(0)` Water) that S116 does not solve. Further tightening toward 200 requires a follow-up spec that changes acquire-water-for-wash precedence or the wash precondition itself.

Update the rationale comment beside that scenario-authored bound to reference S116 and the remaining water-possession follow-up. Mirror the tightening on the scattered survival contract only if its empirical envelope supports it; otherwise leave scattered unchanged.

### D9: Unit coverage for motive math

Add three unit tests near `drive_score` in `ranking.rs`:

1. Multiplier 1000 permille (i.e., exposure = 0 or below `start_after_ticks`) returns the pre-S116 score unchanged (regression guard against silent change to the multiplier-free path).
2. Multiplier 2000 permille (exposure sufficient for 2× growth) doubles the score.
3. Multiplier saturates at `max_multiplier` regardless of further counter growth.

These run independently of the goldens and pin the scoring arithmetic.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Escalation state is derived entirely from the agent's own `HomeostaticNeeds`, `DriveThresholds`, `DeprivationExposure`, and `DriveEscalationProfile` — all agent-owned components. `needs_system` reads/writes these components via `WorldTxn`. `ranking.rs` reads them through `GoalBeliefView::homeostatic_needs`, `drive_thresholds`, `deprivation_exposure`, and `drive_escalation_profile` for the planning agent only. No cross-agent reads, no global-state queries. FND-7 and FND-14 preserved.

2. **Positive-feedback analysis**: Escalation is part of a **negative** (homeostatic) feedback loop: need rises → exposure counter grows → multiplier grows → relief action prioritized → relief performed → need falls → counter resets → multiplier returns to 1×. There is no positive-feedback loop introduced. The existing positive-feedback loop (wilderness relief raises dirtiness, which raises further wilderness-relief pressure) is unchanged — escalation interrupts it by elevating wash above food pursuit once dirtiness has accumulated.

3. **Concrete dampeners**: The dampener is the **physical relief action itself** (wash for dirtiness; eat, drink, sleep, toilet for the other needs). This is a world process — the agent travels to a relief-capable facility, possesses the required inputs, and commits the multi-tick action, which reduces the need by the normal mechanism. No numerical clamp. `DriveEscalationParams.max_multiplier` is a defensive cap, not a dampener: in healthy operation, the counter resets well before the cap fires.

4. **Stored state vs. derived read-model**:
   - **Stored state (authoritative)**:
     - `DeprivationExposure` (existing component, extended with `dirtiness_critical_ticks`). Maintained by `needs_system`.
     - `DriveEscalationProfile` (new universal agent component). Set by scenario / `Default`.
   - **Derived (per-tick computation)**: `escalation_multiplier(ticks, params)` — a pure function of stored counter and stored profile. Never persisted. Recomputed each ranking pass.
   - **Event log**: `EventTag::Escalation` begin/end transitions — authoritative append-only history.

## SystemFn Integration

No new SystemFn. `needs_system` (existing, `SystemId::Needs`, first in `SystemManifest::canonical()`) is extended as described in D3. Existing causal ordering — `Needs` before all economic, political, perception, and planning-facing systems — already satisfies "counter reflects this tick's need values before the agent's next ranking pass". The existing ordering comment in `sim/src/system_manifest.rs` remains unchanged.

## Component Registration

| Component | Classification | `AgentDef` field | `spawn_agent()` path |
|-----------|----------------|------------------|----------------------|
| `DriveEscalationProfile` | Universal | `drive_escalation_profile: Option<DriveEscalationProfile>` | Always applied, `unwrap_or_default()` |
| `DeprivationExposure` | (already registered) | (not in AgentDef — runtime state) | Initialized empty by `needs_system` first tick, same as today |

`DriveEscalationProfile` is directly RON-deserializable (no `EntityId` refs) — no `*Def` wrapper needed. `Default` impl returns engine-wide defaults as specified in D2. Save/load round-trip coverage mirrors `DriveThresholds` and `UtilityProfile`.

## Cross-System Interactions (FND-26)

All interactions are state-mediated:

- `needs_system` → writes `HomeostaticNeeds` (existing), writes `DeprivationExposure` (existing for 4 needs, extended to 5), emits `EventTag::Escalation` transitions → `ranking.rs` (AI) reads `DeprivationExposure` + `DriveEscalationProfile` via `GoalBeliefView`.
- `ranking.rs` → computes multiplier at read time, produces ranked goals → planning search consumes them (unchanged).
- Relief action handlers → write `HomeostaticNeeds` (need reduction) → next tick's `needs_system` observes sub-critical need → resets the relevant `*_critical_ticks` counter → emits `EventTag::Escalation` end event.

No system calls another directly. No cross-crate imperative dispatch. No new SystemFn.

## Risks and Open Questions

1. **Interaction with planner margin-based commitment (S74)**: The multiplier can change mid-plan as the counter increments. Mitigation: the counter updates at most once per tick, at `SystemId::Needs`, which runs before the agent's ranking pass — margin-based commitment already tolerates per-tick motive changes.
2. **Interaction with Obligation Satiation (S96)**: Satiation dampens obligation motive; escalation amplifies homeostatic-need motive. The two apply to disjoint goal categories and compose cleanly.
3. **Default parameter calibration against existing goldens**: The default (`start_after_ticks=100`, `growth_per_tick=10`, `max_multiplier=3000` multiplier-permille = 3×) must not destabilize `survival-baseline.ron` or `survival-scattered.ron`. **Acceptance criterion**: default parameters must leave per-agent wash-count, eat-count, drink-count, and sleep-count distributions on those two scenarios within ±10% of the current golden fixtures, and must not introduce any new `MAX_CRITICAL_RUN_TICKS` violations. If empirical re-runs breach this band, tighten the default before landing; document the calibration choice in the ticket.
4. **Escalation on `Fatigue`**: Fatigue relief is sleep, which is already a commonly-chosen goal. Unlikely to change fatigue behavior meaningfully. Verify as part of the calibration re-run above.
5. **Wash water-possession bottleneck**: Noted in Motivating Evidence. Escalation raises Wash's ranked score but does not guarantee the agent holds water at the right moment. The D8 target of 300 accepts this limit. A follow-up spec may change `wash_preconditions` (e.g., accept co-located water without possession, or add an `AcquireCommodity(Water, purpose=Wash)` sub-goal precedence rule under active dirtiness escalation).

## Verification Plan

1. `cargo test -p worldwake-ai --test golden_drive_escalation` — 3 new goldens pass.
2. `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored` — retrofit `survival_health_contract.max_authored_critical_run_ticks = 300` passes.
3. `cargo test -p worldwake-ai --test golden_survival_scattered` — still passes within the ±10% calibration acceptance band.
4. `cargo test -p worldwake-ai --test golden_survival_baseline` — still passes within the ±10% calibration acceptance band.
5. `cargo test -p worldwake-core` — `DeprivationExposure` dirtiness round-trip + `DriveEscalationProfile` component-table + bincode round-trip coverage pass.
6. `cargo test -p worldwake-ai ranking` — three new unit tests (D9) pass: multiplier=1000 regression, multiplier=2000 doubles, saturation at cap.
7. `cargo clippy --workspace --all-targets -- -D warnings` — clean.
8. Observer re-run of `survival-contested.ron` with `/scenario-analysis` — expected: 0 sustained-critical anomalies (or substantially fewer / shorter ones); `MAINTENANCE_STARVATION` (if S117 has landed) should also be absent.

## Outcome

Completed on 2026-04-18.

- Landed the drive-escalation substrate across the live architecture: `DriveEscalationProfile`, extended `DeprivationExposure`, escalation transition event tagging from `needs_system`, planner-facing belief accessors, motive-score multiplier integration in AI ranking, and decision-trace multiplier surfacing.
- Landed the authored scenario and golden proof chain through the archived S116 ticket family, including `golden_drive_escalation.rs`, the `drive-escalation-wash-priority.ron` scenario, and the contested authored-bound tightening from 400 to 300 in `survival-contested.ron`.
- Preserved the canonical `needs_system` carrier instead of introducing a separate `drive_escalation_system`, and completed the survival-health retrofit/follow-up chain through archived S119 and S121 work before closing the contested calibration loop.

### Deviations

- The final implementation differed from the original draft in 3 important ways:
  - escalation transitions remained on the existing `needs_system` path rather than introducing a dedicated new SystemFn
  - the contested retrofit ended at a scenario-authored `survival_health_contract.max_authored_critical_run_ticks = 300`, not the earlier draft target of 200
  - long-run survival proof carriage moved onto the later authored-contract substrate (`S119`/`S121`), so the final contested tightening lived in `survival-contested.ron` instead of a file-local golden constant

### Verification Result

- Passed `cargo test -p worldwake-ai --test golden_drive_escalation`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
- Passed `cargo test -p worldwake-ai --test golden_survival_contested -- --ignored`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
