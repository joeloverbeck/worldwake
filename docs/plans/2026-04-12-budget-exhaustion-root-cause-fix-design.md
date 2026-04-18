# Budget Exhaustion Root Cause Fix — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix GOAP planner budget/frontier exhaustion by giving agents recipe knowledge and adding fail-fast detection when all goal-relevant operators fail.

**Architecture:** Two independent fixes. Fix 1: add `KnownRecipes` to agent seed functions and scenario files so Harvest affordances are generated. Fix 2: detect at root expansion when only Travel candidates survive and cap the search budget. Fix 3: investigate TreatWounds belief routing.

**Tech Stack:** Rust, golden snapshot tests, RON scenario files.

---

### Task 1: Add Recipe Knowledge to Golden Test Agent Seed Functions

**Files:**
- Modify: `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs:332-349` (Merchant Vara)
- Modify: `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs:483-500` (Guard Theron)
- Modify: `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs:502-532` (Kael)
- Modify: `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs:895-911` (generic `seed_agent_at`)

**Step 1: Add recipe resolution to `seed_merchant_vara_cli_agent`**

Change `seed_merchant_vara_cli_agent` to accept the harness (for recipe lookup) and populate recipes:

```rust
fn seed_merchant_vara_cli_agent(
    h: &mut GoldenHarness,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    let harvest_water = h
        .recipes
        .recipe_by_name("Harvest Water")
        .expect("pathology harness should include Harvest Water")
        .0;
    let harvest_grain = h
        .recipes
        .recipe_by_name("Harvest Grain")
        .expect("pathology harness should include Harvest Grain")
        .0;
    let harvest_apples = h
        .recipes
        .recipe_by_name("Harvest Apples")
        .expect("pathology harness should include Harvest Apples")
        .0;
    let bake_bread = h
        .recipes
        .recipe_by_name("Bake Bread")
        .expect("pathology harness should include Bake Bread")
        .0;
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Merchant Vara",
        place,
        needs,
        MetabolismProfile::default(),
        merchant_vara_utility_profile(),
        KnownRecipes::with([harvest_water, harvest_grain, harvest_apples, bake_bread]),
    );
    configure_merchant_vara_cli_components(h, agent);
    agent
}
```

**Step 2: Add recipe resolution to `seed_guard_theron_cli_agent`**

```rust
fn seed_guard_theron_cli_agent(
    h: &mut GoldenHarness,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    let harvest_water = h
        .recipes
        .recipe_by_name("Harvest Water")
        .expect("pathology harness should include Harvest Water")
        .0;
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Guard Theron",
        place,
        needs,
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([harvest_water]),
    );
    configure_guard_theron_cli_components(h, agent);
    agent
}
```

**Step 3: Add recipe resolution to `seed_kael_cli_agent`**

```rust
fn seed_kael_cli_agent(
    h: &mut GoldenHarness,
    place: EntityId,
    needs: HomeostaticNeeds,
) -> EntityId {
    let harvest_water = h
        .recipes
        .recipe_by_name("Harvest Water")
        .expect("pathology harness should include Harvest Water")
        .0;
    let harvest_grain = h
        .recipes
        .recipe_by_name("Harvest Grain")
        .expect("pathology harness should include Harvest Grain")
        .0;
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Kael",
        place,
        needs,
        MetabolismProfile::default(),
        /* keep existing UtilityProfile */
        UtilityProfile { ... },  // copy from current code
        KnownRecipes::with([harvest_water, harvest_grain]),
    );
    // keep existing perception/cognitive/etc setup below
    ...
}
```

**Step 4: Compile check**

Run: `cargo check -p worldwake-ai --tests`
Expected: Compiles (assertions will fail at runtime but that's Task 4)

**Step 5: Commit**

```bash
git add crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs
git commit -m "Add role-appropriate KnownRecipes to golden budget exhaustion test agents"
```

---

### Task 2: Add Recipe Knowledge to Scenario Files

**Files:**
- Modify: `scenarios/cli-evaluation.ron` (Kael, Merchant Vara, Guard Theron)
- Modify: `crates/worldwake-cli/tests/fixtures/cli_integration.ron` (Kael, Merchant Vara)

**Step 1: Add `known_recipes` to agents in `cli-evaluation.ron`**

For Kael (around line 63):
```ron
known_recipes: ["Harvest Water", "Harvest Grain"],
```

For Merchant Vara (around line 122):
```ron
known_recipes: ["Harvest Water", "Harvest Grain", "Harvest Apples", "Bake Bread"],
```

For Guard Theron (around line 287):
```ron
known_recipes: ["Harvest Water"],
```

Forager Lina already has `known_recipes: ["Harvest Apples"]` — leave as-is.

**Step 2: Add `known_recipes` to agents in `cli_integration.ron` fixture**

For Kael (line 17):
```ron
known_recipes: ["Harvest Water"],
```

For Merchant Vara (line 22):
```ron
known_recipes: ["Harvest Water", "Harvest Grain", "Harvest Apples", "Bake Bread"],
```

**Step 3: Verify scenario loading still works**

Run: `cargo test -p worldwake-cli -- scenario`
Expected: All scenario-loading tests pass.

**Step 4: Commit**

```bash
git add scenarios/cli-evaluation.ron crates/worldwake-cli/tests/fixtures/cli_integration.ron
git commit -m "Add role-appropriate known_recipes to scenario agent definitions"
```

---

### Task 3: Investigate TreatWounds Belief Routing

**Files:**
- Read: `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — TreatWounds setup functions
- Read: `crates/worldwake-ai/src/search/strategic.rs` — `acquisition_places_for_commodity`

**Step 1: Check if TreatWounds test agents have beliefs about Medicine at Hearthstone**

Read the setup functions for TreatWounds scenarios (`setup_merchant_vara_treat_wounds_snapshot` and `setup_kael_treats_vara_snapshot`). Look for:
- `seed_belief_from_world` calls that include Hearthstone Inn entities
- `infer_beliefs_about` calls for Medicine
- Whether agents at Dusty Trail have ever perceived Hearthstone Inn (they probably haven't — different location)

**Step 2: Document findings**

If beliefs are absent: TreatWounds budget exhaustion is **correct behavior** (FND-14/15 — agent doesn't know where Medicine is). The golden test should remain as a budget/frontier exhaustion contract. No code change needed.

If beliefs are present: the strategic planner has a routing bug. File a follow-up ticket.

**Step 3: Commit investigation notes**

No code changes expected — just documenting the finding.

---

### Task 4: Add Futile Root Expansion Detection to Search

**Files:**
- Modify: `crates/worldwake-ai/src/search/mod.rs` (~line 411, after tactical filter)
- Test: Existing golden budget exhaustion tests serve as verification

**Step 1: Add futile search detection after root expansion**

After line 411 in `search/mod.rs` (after `apply_tactical_candidate_filter_with_expansion_trace`), add:

```rust
// Detect futile root expansion: all goal-relevant operators failed,
// only Travel candidates remain. Cap the effective budget to avoid
// cycling through 224-300 Travel-only expansions.
if depth == 0 && !candidates.is_empty() {
    let all_travel = candidates.iter().all(|c| {
        semantics_table
            .get(&c.def_id)
            .is_some_and(|s| s.op_kind == crate::PlannerOpKind::Travel)
    });
    if all_travel && !root_omissions.is_empty() {
        let goal_relevant_ops = goal.key.kind.relevant_op_kinds();
        let all_relevant_omitted = goal_relevant_ops.iter().all(|op_kind| {
            *op_kind == crate::PlannerOpKind::Travel
                || root_omissions
                    .iter()
                    .any(|omission| omission.op_kind == *op_kind)
        });
        if all_relevant_omitted {
            effective_budget = cognitive.max_node_expansions / 4;
        }
    }
}
```

This requires declaring `effective_budget` before the loop and using it in the budget check at line 343:

```rust
let mut effective_budget = cognitive.max_node_expansions;
// ... in the loop:
if expansions >= effective_budget {
    // ... existing budget exhaustion logic
}
```

**Step 2: Compile check**

Run: `cargo check -p worldwake-ai`
Expected: Compiles.

**Step 3: Run clippy**

Run: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
Expected: No warnings.

**Step 4: Commit**

```bash
git add crates/worldwake-ai/src/search/mod.rs
git commit -m "Add futile root expansion detection to cap search budget on Travel-only roots"
```

---

### Task 5: Run Golden Tests and Update Assertions

**Files:**
- Modify: `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs:2184-2302`

**Step 1: Run the golden budget exhaustion tests to see new behavior**

Run: `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --nocapture 2>&1`
Expected: Tests fail with different expansion counts or plan-found results. Record the actual values.

**Step 2: Update test assertions based on actual outcomes**

For each test, update `assert_exact_exhaustion` to match the new behavior:
- Water scenarios with recipe knowledge: may now find plans (change to `assert!(matches!(result, PlanSearchResult::Found(_)))`)
- Apple scenario: may find plan via travel to Eldergrove + Harvest
- TreatWounds scenarios: likely still exhausted (agents don't know where Medicine is), but with reduced budget from fail-fast detection
- Update expansion counts to match actual values

**Step 3: Run full test suite**

Run: `cargo test -p worldwake-ai`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs
git commit -m "Update golden budget exhaustion assertions after recipe knowledge and fail-fast fixes"
```

---

### Task 6: Full Verification

**Step 1: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: Clean.

**Step 2: Run full workspace tests**

Run: `cargo test --workspace`
Expected: All pass. No regressions.

**Step 3: Final commit if any fixups needed**

---

### Task 7: Regenerate Residual Candidate Report (Optional)

**Step 1: If desired, regenerate the report to show improved candidate generation**

The ignored test `generate_residual_candidate_report()` can be run manually to produce an updated `reports/s94-residual-candidte-inventory.md` showing the new candidate landscape.

Run: `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots generate_residual_candidate_report -- --nocapture --ignored`

This is optional — only if the user wants to see the updated report.
