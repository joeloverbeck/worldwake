# Refresh Generated Docs and Roadmap Truth (Step 7: Same Pass)

When scenario, golden, or family workflow files change, refresh everything the roadmap reads from in one pass.

## Regenerate companion docs

When scenario, golden, or family workflow files change, refresh the generated companions that feed the roadmap:

```bash
cargo run -p worldwake-cli --bin scenario-coverage -- --write
python3 scripts/golden_inventory.py --write --check-docs
```

If these fail because of local annotation drift or generated-doc drift you just introduced, fix that and rerun. Do not leave stale generated artifacts behind.
If these fail because of a pre-existing unrelated blocker elsewhere in the repo, isolate the blocker, apply the smallest truthful fix needed to complete the required refresh, and report that side-fix explicitly instead of stopping with stale generated docs.

## Schema fallout sweep

If you changed the scenario authoring schema or spawn path while landing the row, do a bounded fallout sweep before treating the refresh as complete:

- search for synthetic `ScenarioDef { ... }` initializers that now need the new field(s)
- search `scenarios/*.ron`, especially broad fixtures like `scenarios/cli-evaluation.ron`, for old field shapes that no longer match the live authored schema
- update `crates/worldwake-cli/src/bin/scenario_coverage.rs` when the new authored substrate needs structural detection support
- rerun the generated-doc refresh only after those schema consumers parse and compile again

## Roadmap sections to edit

Then update `docs/scenario-roadmap.md` as needed. Common sections that need edits:

- `Gameplay Feature Catalog`
- `Status Summary`
- `Ordered Roadmap`
- `Planned Entry Summaries`
- `Landed Scenarios`
- `Auxiliary and Non-Roadmap Scenarios`
- `Maintenance Workflow` or detection appendix when the generator rule itself changed
- any row-ordinal or range prose that changed because the row status moved at all, not just `Landed` transitions (for example a `Remaining planned rows` sentence that still says `Rows 9-17` after row 9 became `In Progress`)

## Outcome classification

Update the roadmap to match the true result:

- mark the row `Landed` only if scenario, golden, and generated companion all agree
- if the outcome is narrower, rewrite the row rather than overclaiming
- if an auxiliary scenario now became a true roadmap row, move or rewrite the auxiliary caveat accordingly
- if the generated companion shows additional active features beyond the requested row, classify each one explicitly as either newly landed here or merely structurally active because of shared substrate; do not let those sibling rows inherit `Landed` status by implication
- for capstone/coexistence rows, separate `full structural coexistence`, `representative causal branch`, and `structural-only support mechanics`; do not rewrite the feature catalog as if every structurally active feature now has a new standalone behavior landing

When the row ends `In Progress`, prefer a concrete partial-progress writeup over vague prose. A good pattern is:

- `Scenario-owned progress`: feature rows or seams now truthfully proved here
- `Structurally active only`: sibling rows activated by shared authored substrate but not behaviorally proven
- `Blocked`: the exact remaining feature row or seam plus its owning ticket

For multi-feature rows, do not collapse these into a single status sentence. Write the row prose so a later reader can tell exactly which part moved forward and which part still blocks `Landed`.

## CI ownership refresh

Also make the CI ownership truthful in the same pass:

- update an existing family workflow matrix entry when the new scenario belongs to that family
- create the new family workflow when no correct family exists yet
- use explicit workflow matrix `test_target` values when the scenario name and Rust test binary do not share the family-derived name
- keep the regular `ci.yml` lanes relying on ignored-by-default behavior rather than inlining these long-running tests there

## Rename/promotion fallout

Common rename/promotion fallout to expect during refresh:

- `docs/generated/scenario-coverage.md`
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-scenario-details/*`, including deletion of the old generated detail page when the scenario identifier changed
