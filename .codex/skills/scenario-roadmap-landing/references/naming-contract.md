# Naming Contract

For roadmap scenarios, follow the existing naming pattern unless live repo conventions have changed:

- scenario file: `scenarios/<roadmap-name>.ron`
- golden file: `crates/worldwake-ai/tests/golden_<roadmap_name_with_underscores>.rs`
- CI family workflow: `.github/workflows/golden-<family>.yml`

Examples:

- `survival-drive-escalation` -> `scenarios/survival-drive-escalation.ron` and `golden_survival_drive_escalation.rs`
- `survival-tell` -> `scenarios/survival-tell.ron` and `golden_survival_tell.rs`

Before assuming a new file is required, check whether an existing auxiliary or partial scenario/golden already owns part of the contract.
If the row is currently owned only by an auxiliary scenario/golden pair, prefer promoting that owner into the roadmap naming shape instead of cloning it into parallel files.
Also check whether the scenario belongs in an existing family matrix workflow or needs a new family workflow. Follow the existing repo convention shown by `golden-survival.yml` and `golden-drive-escalation.yml`: one matrix workflow per long-running scenario family.
If a roadmap-named scenario/golden exists but only a narrower seam is currently proven, keep the roadmap file names when they are still the truthful row owner, but narrow the test names, scenario comments, and roadmap prose to the actually proven seam instead of claiming the full row landed.

## Auxiliary-promotion pattern

1. Resolve the existing auxiliary owner.
2. Decide whether truthful promotion is possible in-place.
3. Prefer rename/promote over duplicate-copy when the auxiliary file already owns the mechanic.
4. Add the missing authored substrate that makes the row structurally and behaviorally truthful:
   - `survival_health_contract` when the row is a survival-roadmap landing
   - any non-default authored profile or world-state gate that `scenario_coverage` requires
5. Refresh roadmap/generated docs and remove the old auxiliary caveat in the same pass.
