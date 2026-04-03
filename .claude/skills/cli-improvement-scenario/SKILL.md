---
name: cli-improvement-scenario
description: "Update the CLI evaluation scenario (scenarios/cli-evaluation.ron) when new simulation features land. Invoke after implementing specs that add new action types, systems, or components."
user-invocable: true
---

# CLI Evaluation Scenario Maintenance

Update the dedicated evaluation scenario to exercise new simulation features, keeping the CLI evaluation pipeline comprehensive as the simulation evolves.

## Invocation

```
/cli-improvement:scenario
```

No arguments. Reads the current scenario and recent changes to determine what needs updating.

## When to Invoke

After implementing a spec that adds new simulation capabilities:
- New action types (e.g., new trade mechanisms, combat actions, social actions)
- New component types (e.g., new profiles, new state components)
- New systems (e.g., new needs, new production recipes)
- New entity kinds or place features

This skill is NOT part of the evaluate-implement loop. It's a maintenance operation run when the simulation grows.

## Process

Follow these steps in order.

### Step 1: Read Current Scenario

First, validate the scenario loads: `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit 2>&1`. If it fails with a parse error (missing field, type mismatch), fix the schema drift before proceeding with feature analysis. This is the most common maintenance trigger.

Then read `scenarios/cli-evaluation.ron` to understand what's currently exercised.

Take inventory:
- Which place tags are used
- Which agent profiles are present (needs, combat, utility, merchandise, trade)
- Which commodities exist
- Which facilities/workstations exist
- Which resource sources exist

Compare each inventory against the full set of variants in the corresponding enum (`CommodityKind::ALL`, `WorkstationTag::ALL`, `PlaceTag::ALL`) to identify unexercised variants. Not all variants need scenario coverage — apply "exercise, don't overload."

### Step 2: Read Latest Evaluation

Read the latest evaluation from `reports/cli-evaluation.md` (last ~100 lines) to understand:
- Are there commands that couldn't be fully exercised due to missing scenario elements?
- Are there recommendations about scenario gaps?

If `reports/cli-evaluation.md` does not exist, skip this step — there are no prior evaluation recommendations to consider.

### Step 3: Identify New Features

Check what's changed recently:

1. Read recent git commits: `git log --oneline -20`
2. Check active specs in `specs/` for newly implemented features
3. Check `crates/worldwake-core/src/` for new component types or profile types
4. Check `crates/worldwake-systems/src/` for new action registrations
5. Check `crates/worldwake-cli/src/scenario/types.rs` for any new scenario def fields
6. For each new component or feature, check whether it appears in `AgentDef` or other scenario def types. Components that are runtime-generated (e.g., experience records, belief state, active goals) don't need scenario entries — they emerge naturally from agent behavior during ticking. Only features with scenario-definable fields need scenario updates.
7. Compare the full set of `AgentDef` fields against what the current scenario RON actually uses. Fields present in `AgentDef` but absent from all agents in the RON are coverage gaps — these are the primary candidates for scenario updates.

### Step 4: Update the Scenario

If new features exist that aren't exercised by the current scenario, update `scenarios/cli-evaluation.ron`:

- **New agent profiles**: Add an agent with the new profile, or add the profile to an existing agent where it makes sense
- **New commodities**: Add items of the new type at appropriate places
- **New facilities/workstations**: Add the facility at an appropriate place
- **New resource sources**: Add a source at an appropriate place
- **New place tags**: Add a place with the new tag if travel/location features use it
- **New travel features**: Add edges that exercise new travel mechanics

Preserve existing agents and places where possible — changing established entities could invalidate previous evaluation comparisons.

### Step 5: Add Change Comment

Add a comment at the top of the RON file documenting what was added and why:

```ron
// CLI evaluation scenario — exercises all CLI features.
// Updated YYYY-MM-DD: Added <description of changes> for <spec/feature>.
```

### Step 6: Validate

Launch the CLI with the updated scenario and immediately quit:

```bash
cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron <<< "quit"
```

If it fails, fix the RON errors and try again.

### Step 7: Summary

Report what was added to the scenario and why. Suggest running `/cli-improvement:evaluate` to measure how the new features affect CLI usability.

## Guardrails

- **Preserve stability**: Don't rename or remove existing agents/places unless absolutely necessary. Adding is safer than changing.
- **Exercise, don't overload**: Add enough to exercise new features, not everything conceivable. Keep the scenario manageable.
- **Validate always**: Never leave an invalid RON file. Always run the validation step.
- **No evaluation**: This skill only updates the scenario. Do not score or evaluate — that's the evaluate skill's job.
- **No CLI changes**: Do not modify `crates/worldwake-cli/` — that's the implement skill's job.
- **Document changes**: Always update the comment at the top of the RON file.
