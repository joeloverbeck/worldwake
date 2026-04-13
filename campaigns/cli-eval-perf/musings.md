# Musings

## Baseline Profiling (200-tick sample)

| Phase | % of Total |
|---|---|
| AI planning (produce_inputs) | 85.4% |
| run_systems | 10.5% |
| process_inputs | 2.4% |
| progress_actions | 1.7% |

Within AI planning:
- **search_plan_with_trace_metadata**: 98.3% of planning time (~84% of total)
- Snapshot building: 1.5%
- Candidate ranking: <1%

Within run_systems:
- Perception: 79% of system time (~8% of total)
- Needs: 3.7%

Flat perf profile: BTreeMap operations dominate (find_key_index 13%, search_tree 5%, Enumerate::next 7%, memmove 3%). Consistent with heavy ECS queries during GOAP node expansion.

Agent e6g0 dominates with 67% of planning time. 3-4 candidates per call. Cost is per-expansion state lookups, not candidate count.

**Priority targets**: (1) reduce GOAP search cost per expansion, (2) perception system cost.

## exp-001: Gate expansion trace allocation behind expansion_summaries
**Category**: trace-overhead (UCB1 score: infinity)
**Hypothesis**: `expansion_candidates` Vec is allocated and populated on EVERY expansion regardless of whether `expansion_summaries` is Some. Each trace struct contains a String + Vec heap alloc. With 50-500+ candidates * 224 expansions per search, this is massive wasted allocation when tracing is off. Making it conditional should eliminate the dominant allocation overhead in the hot path.

**Profiling evidence**: Line 415 in search/mod.rs passes `Some(&mut expansion_candidates)` unconditionally. The struct `ExpansionCandidateTrace` contains `action_name: String` and `authoritative_targets: Vec<EntityId>` — two heap allocations per candidate. Similarly, lines 452 and 458 pass Some unconditionally for filter traces. The perf flat profile shows BTreeMap and allocation operations dominating.

**Result**: NEAR_MISS (335060 -> 329916, 1.54% improvement, within 3% noise threshold)
**Learning**: The trace allocation gating produces a measurable but modest improvement (~5s). This confirms trace overhead exists but is NOT the dominant cost. The BTreeMap operations in perf profile are from actual ECS queries during node expansion, not just trace allocation. The real bottleneck is the per-expansion state queries (BTreeMap::get calls on PlanningState components). Need to target the search algorithm itself — fewer expansions, cheaper per-expansion work, or better pruning.

**Key insight**: PlanningState uses SharedMap (Rc<BTreeMap>) with COW semantics, so clone is O(1). The BTreeMap cost is from QUERIES, not cloning. Debug mode makes BTreeMap::get expensive due to no inlining.

## exp-002: Combine trace gating with additional trace-computation elimination
**Category**: trace-overhead (UCB1 score: infinity for other categories, but combining near-miss)
**Hypothesis**: exp-001 near-miss (1.54%) plus additional gating of: (1) travel pruning trace allocation, (2) landmark_heuristic computation moved inside expansion_summaries guard, (3) planning_facts_from_state call moved inside guard, (4) preferred_candidates / non_terminal_after_beam moved inside guard. Cumulative effect should exceed 3% threshold.

**Result**: NEAR_MISS (335060 -> 329072, 1.79% improvement, within 3% threshold)
**Learning**: Additional trace-computation gating adds only ~0.25% over exp-001. The trace-overhead category is capped at ~2% total improvement. The extra computations (landmark_heuristic, planning_facts_from_state) are cheap individually. The fundamental cost driver is NOT trace infrastructure — it's the actual search work (affordance generation, successor building, state queries). Must pivot to a different category to break the 3% barrier.

## Deep Profiling: Per-tick cost growth

Phase breakdown over 1440 ticks (debug mode) revealed:
- **Perception system** goes from 200us (tick 0) to 45,000us (tick 1400) — 220x increase
- All other phases (AI planning, process_inputs, progress_actions) remain relatively stable
- The initial 200-tick profiling was misleading — perception was only 8% early but becomes 90%+ of cost at tick 1200+

Sub-phase breakdown within perception:
- **observe_passive_local_entities** is the sole bottleneck (3ms -> 667ms)
- Event witness loop, active action observation, and commit are all stable

Root cause: **Entity accumulation at Dusty Trail** — grows from 3 entities to 540 by tick 1400.
Each perception tick, every colocated agent iterates ALL entities, builds snapshots, detects mismatches.
O(agents_at_place * entities_at_place * per_entity_work) per tick. With 540 entities, this dominates.

## exp-003: Skip re-observation of unchanged entities
**Category**: perception-opt (UCB1 score: infinity)
**Hypothesis**: Track `last_modified_tick` per entity. During passive observation, skip entities where the agent already observed them after their last change. At tick 1400, ~530 of 540 entities at Dusty Trail are unchanged items sitting on the ground. This should reduce perception from 667ms to ~10-20ms at late ticks, for a massive overall improvement.

**Profiling evidence**: Dusty Trail entity count grows from 3 to 540 over 1440 ticks. observe_passive_local_entities grows from 3ms to 667ms. Belief stores are stable (12-16 claims/agent) — the cost is in snapshot building + mismatch detection, not belief management.

**FOUNDATIONS alignment**: Principle 12 — pure computation compression. Re-observing an unchanged entity yields identical beliefs. Stochastic observation check is for initial perception, not continuous re-confirmation.

**Result**: REJECT — multiple approaches tried:
1. Skip entity entirely from observed_snapshots → crash at tick 633 (behavioral divergence causes precondition failure in justice system)
2. Skip but refresh observed_tick → still crashes (claim dynamics not preserved)
3. Reuse prior belief as snapshot (clone + insert) → 17% regression (clone cost in debug mode + downstream processing still expensive)
4. Source-gated skip (only DirectObservation) → too conservative, barely any skipping, 2.6% regression from behavioral change

**Learning**: Perception optimization at this depth is NOT a simple experiment. The observation-claim-belief pipeline is tightly coupled:
- Skipping observation skips claim re-recording → stale acquired_tick → different confidence decay → different belief derivation → different planning → crash
- Reusing beliefs as "observations" avoids snapshot build but clone + mismatch detection + claim recording still runs → no net gain in debug mode
- The entity accumulation at Dusty Trail (540 entities) is fundamentally a content/design issue that needs architectural treatment

## CEILING REPORT
**Ceiling metric**: 335,060ms (baseline unchanged)
**Experiments since last accept**: 3 (all NEAR_MISS or REJECT)
**Categories attempted**: trace-overhead (2 near-misses, capped at ~2%), perception-opt (1 reject, requires spec)
**Architectural bottlenecks identified**:
1. **Perception quadratic growth**: `observe_passive_local_entities` scales O(entities_at_place * per_entity_snapshot_cost). With 540 entities at Dusty Trail by tick 1400, perception goes from 200us to 45,000us per tick. This is 90%+ of total cost at late ticks. Fix requires a spec for dirty-entity-tracking with careful handling of claim freshness dynamics.
2. **Entity accumulation**: Production creates items that pile up at places with no consumption or decay. Dusty Trail grows from 3 to 540 entities. This is a content/design issue — either production needs consumers/decay or perception needs bounded observation.
3. **Debug mode overhead**: BTreeMap operations lack inlining, making every query 5-10x more expensive than release. trace-overhead optimizations (gating allocations) save ~2% but can't overcome the fundamental debug-mode tax on BTreeMap-heavy code.

**Recommended next steps**:
- Write a spec for perception dirty-entity tracking (entity_change_ticks infrastructure already prototyped in world.rs/world_txn.rs)
- The spec must address claim freshness: how to maintain `acquired_tick` progression without full re-observation
- Consider observation budget per agent per tick (cap entities observed, prioritize by relevance/novelty)
- Consider item decay or consumption to prevent unbounded entity accumulation

