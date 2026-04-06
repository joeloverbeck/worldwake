# Simulation Remediation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the 5 issues identified in `reports/simulation-remediation.md`: add water to scenario, fix planner prerequisite_places for SellCommodity, fix planner fallback when top goal is unsatisfiable, and add 2 golden tests.

**Architecture:** TK-1 is a scenario-only change. TK-3 adds one match arm to `prerequisite_places()` in the goal model. TK-2 modifies candidate admission in `build_candidate_plans` to backfill from lower-ranked candidates when top candidates are frontier-exhausted. GT-1 and GT-2 are golden tests asserting the corrected behaviors.

**Tech Stack:** Rust, RON scenario files, GoldenHarness test framework

**Worktree:** All paths relative to `/home/joeloverbeck/projects/worldwake/.claude/worktrees/simulation-observer/`

---

### Task 1: Add Water Resource Source to Scenario (TK-1)

**Files:**
- Modify: `scenarios/cli-evaluation.ron`

**Step 1: Add water resource source and items**

In `scenarios/cli-evaluation.ron`:

1. Update the comment header (after line 7) — add:
```
// Updated 2026-04-06: TK-1 — Added Water resource source at Thornwall Village and
//   Water items at Eldergrove Forest to fix universal dehydration (simulation-remediation).
```

2. In the `items` section, after the existing Water entry (line 349), add:
```ron
// Water at Eldergrove Forest for Forager Lina
(commodity: Water, quantity: 5, location: "Eldergrove Forest"),
```

3. In the `resource_sources` section (line 369), add:
```ron
(commodity: Water, location: "Thornwall Village", regeneration_ticks_per_unit: 3, capacity: 15),
```

**Step 2: Validate scenario loads**

Run: `cargo run -p worldwake-cli --bin worldwake-cli -- scenarios/cli-evaluation.ron --exec quit 2>&1`
Expected: Clean exit, no errors.

**Step 3: Commit**

```bash
git add scenarios/cli-evaluation.ron
git commit -m "TK-1: add water resource source to cli-evaluation scenario"
```

---

### Task 2: Add prerequisite_places for SellCommodity (TK-3)

**Files:**
- Modify: `crates/worldwake-ai/src/goal_model.rs:1371`
- Test: existing tests in `crates/worldwake-ai/src/goal_model.rs` (test module at bottom)

**Step 1: Write a unit test for the new match arm**

In `crates/worldwake-ai/src/goal_model.rs`, in the test module (after the last `prerequisite_places_*` test, around line 6061+), add:

```rust
#[test]
fn prerequisite_places_sell_commodity_returns_home_facility_when_remote() {
    let (mut world, mut event_log) = test_world();
    let agent = test_agent(&mut world, &mut event_log, TEST_PLACE_A);
    let facility_place = TEST_PLACE_B;

    // Create a facility entity at place B
    let facility = {
        let mut txn = new_test_txn(&mut world);
        let (facility, _, _) = txn
            .create_merchant_facility(facility_place, agent, LoadUnits(500), Some(LoadUnits(300)))
            .unwrap();
        txn.set_component_merchandise_profile(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        )
        .unwrap();
        commit_test_txn(txn, &mut event_log);
        facility
    };

    let view = test_belief_view(&world, agent);
    let goal = GoalKind::SellCommodity {
        commodity: CommodityKind::Bread,
    };
    let budget = ExecutionBudget::default();
    let places = goal.prerequisite_places(&view, agent, &budget);

    // Agent at place A, facility at place B → should return place B
    assert_eq!(places, vec![facility_place]);
}

#[test]
fn prerequisite_places_sell_commodity_empty_when_at_home() {
    let (mut world, mut event_log) = test_world();
    let agent = test_agent(&mut world, &mut event_log, TEST_PLACE_A);

    // Create facility at same place as agent (place A)
    let facility = {
        let mut txn = new_test_txn(&mut world);
        let (facility, _, _) = txn
            .create_merchant_facility(TEST_PLACE_A, agent, LoadUnits(500), Some(LoadUnits(300)))
            .unwrap();
        txn.set_component_merchandise_profile(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(facility),
            },
        )
        .unwrap();
        commit_test_txn(txn, &mut event_log);
        facility
    };

    let view = test_belief_view(&world, agent);
    let goal = GoalKind::SellCommodity {
        commodity: CommodityKind::Bread,
    };
    let budget = ExecutionBudget::default();
    let places = goal.prerequisite_places(&view, agent, &budget);

    // Agent already at home facility → no prerequisite places
    assert!(places.is_empty());
}
```

**Note:** The exact test helper names (`test_world`, `test_agent`, `new_test_txn`, `test_belief_view`, `commit_test_txn`, `TEST_PLACE_A`, `TEST_PLACE_B`) must be checked against the existing test module's helpers. Adapt to whatever patterns the file already uses. The `create_merchant_facility` method exists on WorldTxn and is used in `golden_merchant_selling.rs:1247`.

**Step 2: Run tests to verify they fail**

Run: `cargo test -p worldwake-ai -- prerequisite_places_sell_commodity 2>&1`
Expected: FAIL — the tests call `prerequisite_places` on `SellCommodity` which currently hits the default `_ => Vec::new()` arm, so the "remote" test will fail (returns empty instead of `[facility_place]`).

**Step 3: Add the match arm**

In `crates/worldwake-ai/src/goal_model.rs`, line 1371, replace:
```rust
            _ => Vec::new(),
```
with:
```rust
            GoalKind::SellCommodity { .. } => state
                .merchandise_profile(actor)
                .and_then(|p| p.home_facility)
                .and_then(|facility| state.effective_place(facility))
                .filter(|home_place| state.effective_place(actor) != Some(*home_place))
                .into_iter()
                .collect(),
            _ => Vec::new(),
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p worldwake-ai -- prerequisite_places_sell_commodity 2>&1`
Expected: PASS — both tests pass.

**Step 5: Run full test suite**

Run: `cargo test -p worldwake-ai 2>&1`
Expected: All existing tests still pass. No regressions.

**Step 6: Commit**

```bash
git add crates/worldwake-ai/src/goal_model.rs
git commit -m "TK-3: add prerequisite_places for SellCommodity goal"
```

---

### Task 3: Planner Fallback via Candidate Backfill (TK-2)

**Files:**
- Modify: `crates/worldwake-ai/src/agent_tick/planning.rs:229-245`

**Step 1: Understand current code**

In `planning.rs` lines 229-245, the current logic:
```rust
let admitted_candidates: Vec<_> = ranked_candidates
    .iter()
    .filter(|c| {
        opportunity_admitted_by_exhaustion(
            exhaustion_cache,
            OpportunityKey { goal_key: c.grounded.key, anchor: c.grounded.anchor },
            current_tick,
        )
    })
    .collect();
let candidates_to_plan: Vec<_> = admitted_candidates
    .into_iter()
    .take(usize::from(cognitive.max_candidates_to_plan))
    .collect();
```

Problem: If all top-ranked candidates are frontier-exhausted, `admitted_candidates` is empty → agent idles.

**Step 2: Replace with backfill-aware admission**

Replace lines 229-245 with:
```rust
let cap = usize::from(cognitive.max_candidates_to_plan);
let candidates_to_plan: Vec<_> = ranked_candidates
    .iter()
    .filter(|c| {
        opportunity_admitted_by_exhaustion(
            exhaustion_cache,
            OpportunityKey {
                goal_key: c.grounded.key,
                anchor: c.grounded.anchor,
            },
            current_tick,
        )
    })
    .take(cap)
    .collect();
```

This is functionally equivalent to the current code but avoids the intermediate `admitted_candidates` vec. The key insight: the current code already works correctly IF `ranked_candidates` contains candidates beyond the exhausted top goals. The issue is that `max_candidates_to_plan` is typically small (3-5) and `filter` + `take` already skips exhausted entries to find non-exhausted ones further down the ranked list.

**Wait — re-examine the problem.** Let me re-read to make sure the current code isn't already doing this... The `filter` runs on ALL `ranked_candidates`, not just the first N. Then `take(N)` picks the first N that passed the filter. So the current code DOES backfill past exhausted candidates.

If this is already correct, the stuck behavior has a different root cause. Before implementing, verify by adding a diagnostic test:

**Step 3: Write a diagnostic test**

In `crates/worldwake-ai/tests/golden_ai_decisions.rs`, add:

```rust
// Scenario XX: Agent with unsatisfiable thirst still eats when hungry
// Systems: Needs, AI
// GoalKinds: ConsumeOwnedCommodity
// ActionDomains: Needs
// Principles: P20
// Proves: when top-priority need (thirst) has no viable plan, agent falls back
//   to next-best addressable need (hunger) instead of going idle.

#[test]
fn golden_fallback_to_addressable_need_when_top_need_unsatisfiable() {
    let mut h = GoldenHarness::new(Seed([99; 32]));

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Parched",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(400), pm(700), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    // Give food but NO water — thirst is unsatisfiable
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(5),
    );

    h.driver.enable_tracing();

    let mut ate = false;
    let mut slept = false;
    let mut max_idle = 0u32;
    let mut consecutive_idle = 0u32;

    for _ in 0..200 {
        h.step_once();

        if let Some(sink) = h.action_trace_sink() {
            let actions_this_tick: Vec<_> = sink
                .events_for(agent)
                .iter()
                .filter(|e| matches!(e.kind, ActionTraceKind::Committed { .. }))
                .map(|e| e.action_name.as_str())
                .collect();

            let had_action = !actions_this_tick.is_empty();
            if had_action {
                consecutive_idle = 0;
            } else {
                consecutive_idle += 1;
                max_idle = max_idle.max(consecutive_idle);
            }

            ate |= actions_this_tick.contains(&"eat");
            slept |= actions_this_tick.contains(&"sleep");
        }

        if ate || slept {
            break;
        }
    }

    assert!(
        ate || slept,
        "Agent with unsatisfiable thirst should still eat or sleep — got neither in 200 ticks"
    );
    assert!(
        max_idle < 100,
        "Agent should not be idle for 100+ consecutive ticks, was idle for {max_idle}"
    );
}
```

**Step 4: Run the diagnostic test**

Run: `cargo test -p worldwake-ai golden_fallback_to_addressable_need 2>&1`

If it **passes**: The planner already handles fallback correctly, and the observer's stuck-agent finding was caused by the scenario (no addressable needs at all when dehydrated), not a planner bug. In that case, TK-2 is a scenario issue (already fixed by TK-1) and this test becomes GT-1 as-is.

If it **fails**: The planner genuinely doesn't fall back, and the `build_candidate_plans` logic needs investigation — the `filter` + `take` should be working, so check `continue_same_goal_after_found` (line 255) and `record_exhausted_goals` (line 431) for the actual blocking mechanism.

**Step 5: Investigate if test fails**

If the test fails, read `planning.rs:254-306` carefully. The `continue_same_goal_after_found` logic (line 255-259) breaks out of the planning loop once a DIFFERENT goal key is encountered after a plan was found. This could cause the planner to stop early. Also check `record_exhausted_goals` to see if exhaustion entries are created with `FrontierExhausted` vs `BudgetExhausted` — only `FrontierExhausted` suppresses future planning.

**Step 6: Fix if needed, or confirm test as GT-1**

If the test passes without code changes, commit it as GT-1:
```bash
git add crates/worldwake-ai/tests/golden_ai_decisions.rs
git commit -m "GT-1: golden test for planner fallback to addressable needs"
```

If it fails, implement the fix in `planning.rs`, verify the test passes, then commit both:
```bash
git add crates/worldwake-ai/src/agent_tick/planning.rs crates/worldwake-ai/tests/golden_ai_decisions.rs
git commit -m "TK-2 + GT-1: planner fallback when top need is unsatisfiable"
```

---

### Task 4: Strengthen Merchant Travel-to-Market Test (GT-2)

**Files:**
- Modify: `crates/worldwake-ai/tests/golden_merchant_selling.rs:1318-1322`

**Step 1: Tighten assertion**

In `golden_merchant_selling.rs`, replace lines 1318-1322:
```rust
    // The merchant should at least travel (restock-driven movement toward home market).
    assert!(
        saw_travel || arrived_at_home || saw_staff_market,
        "merchant at remote place with stock and demand memory should eventually move toward home_facility or start selling"
    );
```

with:
```rust
    assert!(
        saw_travel,
        "merchant at remote place should travel toward home_facility"
    );
    assert!(
        saw_staff_market,
        "merchant should staff_market after arriving at home_facility (travel={saw_travel}, arrived={arrived_at_home})"
    );
```

**Step 2: Run the test**

Run: `cargo test -p worldwake-ai move_cargo_then_sell_commodity_plan_shape 2>&1`
Expected: PASS — with the TK-3 fix (prerequisite_places for SellCommodity), the merchant should now chain travel + staff_market.

If it fails, the tick budget (120) may need increasing, or the planner may need additional tuning. Check the trace output.

**Step 3: Run full test suite**

Run: `cargo test -p worldwake-ai 2>&1`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add crates/worldwake-ai/tests/golden_merchant_selling.rs
git commit -m "GT-2: strengthen Scenario 84 to require staff_market completion"
```

---

### Task 5: Full Verification

**Step 1: Clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1`
Expected: No warnings or errors.

**Step 2: Full test suite**

Run: `cargo test --workspace 2>&1`
Expected: All tests pass.

**Step 3: Re-run observer**

Run: `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440 --output /tmp/verify-dump.md 2>&1`

Check `/tmp/verify-dump.md` for:
- Thirst averages below 500 (was 926-981)
- At least one `drink` action per AI agent
- No `UNADDRESSED_NEED` anomaly for thirst
- Max consecutive idle for Guard Theron < 100 (was 1024)
- `staff_market` appears in Merchant Vara's action list (was 0 committed)

**Step 4: Clean up temp file**

```bash
rm /tmp/verify-dump.md
```

**Step 5: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "Simulation remediation: verification cleanup"
```
