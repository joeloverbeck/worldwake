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

First, validate the scenario loads: `cargo run -p worldwake-cli --bin worldwake-cli -- scenarios/cli-evaluation.ron --exec quit 2>&1`. If it fails with a parse error (missing field, type mismatch), fix the schema drift before proceeding with feature analysis. This is the most common maintenance trigger.

To determine correct values for missing fields: read the struct's `Default` impl in the source. Use defaults unless the agent's existing profile values suggest a deliberate personality divergence — in that case, choose a value consistent with the agent's characterization.

**Silent schema drift warning**: RON deserialization silently ignores unknown field names (no `deny_unknown_fields`). A renamed field will not cause a parse error — the old field is silently dropped and the agent gets no value for the new field. Step 3.1 (AgentDef-vs-RON comparison) is the primary defense against this. When Step 3 identifies recent renames (e.g., from commit messages or `types.rs` diffs), manually verify the RON uses the current field names.

Then read `scenarios/cli-evaluation.ron` to understand what's currently exercised.

Take inventory:
- Which place tags are used
- Which agent profiles are present (all optional `AgentDef` fields — compare against the struct in `types.rs`)
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

Check what's changed recently. Substeps 1-3 are the highest-value checks (primary defense against silent schema drift) — run them first.

1. Compare the full set of `AgentDef` fields against what the current scenario RON actually uses. Fields present in `AgentDef` but absent from all agents in the RON are coverage gaps — these are the primary candidates for scenario updates. To perform this comparison: read the `AgentDef` struct definition in `types.rs`, list all `pub` fields (excluding `name`, `location`, `control`), then grep or scan the RON for each field name. Fields that appear in `AgentDef` but not in any agent's RON block are coverage gaps. Also check for fields in the RON that do NOT appear in `AgentDef` — these indicate stale renamed fields (silent schema drift).
2. Check git diff of `types.rs` against the version used when the RON was last updated. Field renames or removals in `AgentDef` that aren't reflected in the RON indicate silent schema drift.
3. For each profile type that IS present in the RON, check whether its internal fields changed. New or renamed fields inside a profile struct (e.g., `CognitiveProfile` gaining a field) won't show up as an AgentDef-level gap — the AgentDef field exists, but the RON block is incomplete. The quickest approach: if Step 1 (scenario validation) produced a parse error naming a specific struct, `git diff` that struct's source file against the last RON update commit to see exactly what changed. Otherwise, spot-check profile structs that appear in recent commits.
4. If substeps 1-3 fully account for the schema drift found in Step 1 and no AgentDef-level fields were added, substeps 5-10 can be scanned quickly (git log + skim) rather than performed exhaustively.
5. Read recent git commits: `git log --oneline -20`
6. Check active specs in `specs/` for newly implemented features
7. Check `crates/worldwake-core/src/` for new component types or profile types
8. Check `crates/worldwake-systems/src/` for new action registrations
9. Check `crates/worldwake-cli/src/scenario/types.rs` for any new scenario def fields beyond what substep 1 already found
10. For each new component or feature, check whether it appears in `AgentDef` or other scenario def types. Components that are runtime-generated (e.g., experience records, belief state, active goals) don't need scenario entries — they emerge naturally from agent behavior during ticking. Only features with scenario-definable fields need scenario updates.

### Step 4: Update the Scenario

If new features exist that aren't exercised by the current scenario, update `scenarios/cli-evaluation.ron`:

- **New agent profiles**: Add an agent with the new profile, or add the profile to an existing agent where it makes sense
- **New commodities**: Add items of the new type at appropriate places
- **New facilities/workstations**: Add the facility at an appropriate place
- **New resource sources**: Add a source at an appropriate place
- **New place tags**: Add a place with the new tag if travel/location features use it
- **New travel features**: Add edges that exercise new travel mechanics
- **Universal profile overrides**: If a universal profile (always applied with defaults) has meaningful scenario-overridable fields (e.g., `last_seen_memory.capacity`), add an explicit override on an agent where it's thematically relevant

When adding a new profile for coverage, choose at least some values that diverge from defaults — this exercises non-default tuning and makes the profile more distinctive. Values should be thematically consistent with the agent's established personality.

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
cargo run -p worldwake-cli --bin worldwake-cli -- scenarios/cli-evaluation.ron --exec quit
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
