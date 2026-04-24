# Scenario Authoring (Step 3: Author as a Survival-Contract Scenario)

When the row is not already satisfied, create or revise the scenario in `scenarios/`.

## Requirements

- it must be a real survival scenario designed to run for 1440 ticks
- it must preserve the prior landed survival loop while adding one new architectural stressor
- it must activate the intended gameplay feature(s) through authored state, not test-only helpers
- it must define a truthful `survival_health_contract` when it is intended to be a roadmap landing

## Scenario design rules

- Copy the closest landed scenario only as a starting point; do not cargo-cult its envelope unchanged.
- Prefer minimal authored setup that still forces the mechanic under test to matter.
- For cumulative survival rows, "minimal" does not mean stripping out already-landed topology, economy, or substrate that the row still depends on. If the new row sits on top of an earlier landed row such as `survival-trade`, preserve that earlier row's truthful branch unless you first narrow or rewrite the roadmap contract.
- Treat `survival_health_contract` as a truth surface, not a wish list. If focused proof shows the authored envelope is too strict or overclaims the live behavior, narrow the authored contract before forcing code or tests to fit it.
- Later roadmap rows may need already-landed feature rows to remain authored-active in the same scenario. Keep those cumulative rows live when they are part of the truthful survival envelope, and move any resulting per-need envelope changes into the authored `survival_health_contract` instead of hiding them in test-local constants.
- Explicitly isolate rival lawful branches only when they would obscure the owned contract.
- If a competing branch is part of the architecture contract, keep it and prove the branching behavior instead.
- For cumulative mechanics, name the concrete threshold/cadence/capacity math in the scenario-owning ticket or notes before trusting the setup.
- Validate authored load and capacity math early whenever the row authors starting inventory, camp supplies, containers, merchant stock, or other carried/contained resources. This includes trade rows, but is not limited to them: check merchant stock volume, buyer purchasing power, self-care supply, camp supply containers, and actor carry-capacity/load limits before spending repeated long reruns on golden debugging.
- For capstone or full-coexistence rows, it is acceptable to use a non-autonomous support actor or support place to carry structural-only activation substrate. The roadmap and golden must state which actor owns the survival/behavior envelope and which profiles or world fields are structural-only support.
- Keep structural-only activators away from the critical proof path when their mechanics could interfere with the retained branch. For example, place concealment, hostile state, queues, offices, or broad social profiles can be authored on support places/actors when the row only needs structural activation from them.
- After adding a cluster of structural-only activators, rerun the narrow scenario spawn or owning golden preflight before layering on more substrate. If a support activator changes the selected branch, either move it off the critical path, narrow the proof contract truthfully, or split the blocker into a ticket.
- When adding support actors, minimize their agency and profiles before accepting their presence in the scenario. Try `ControlSource::None` or absent/zeroed support profiles when the retained mechanic allows it; if the proof requires an AI support actor, document the support role and expect generated coverage to show any unrelated structurally active mechanics.
- Remove abandoned support substrate before the final proof and generated-doc refresh. Places, edges, actors, profiles, or stock that were introduced for a failed proof attempt should not remain unless they carry the retained roadmap seam or an explicitly classified structural-only support role.
- If truthful authored substrate references another authored entity class, verify that the scenario loader can resolve that reference through the canonical spawn path. When the live mechanic requires the reference, fix the scenario authoring boundary with a focused spawn/schema regression instead of weakening the authored setup to avoid the reference.

## Special cases

For social or belief-transport rows, verify the information path explicitly. A scenario is invalid if the intended behavior only works through omniscient setup assumptions.

If a social or belief-transport proof starts from a concrete roadmap claim such as remote testimony, non-colocated witnessing, or a named communication surface, failed proof of that path is a decision point. Use the 1-3-1 rule before replacing it with a different information path such as same-place observation, same-place testimony, or zero-fidelity listener setup.

When roadmap prose blends a profile/dampener with an action branch it influences, split the contract before authoring. Name the feature mechanic, the supporting duty/action substrate, and the exact causal seam each one proves so the golden does not claim that one mechanic directly "discharges" another unless the live code really does that.

When a survival contract legitimately needs an uneven bound across need families, encode that as scenario-authored per-need overrides instead of inventing a stronger global cap and then forcing the codebase to satisfy it.

Before deeper proof/debugging, do a scenario-lint preflight on the authored file or its owning golden once. The expected boundary is the canonical scenario loader/spawn path (`load_scenario_file` + `spawn_scenario`) without `--ignore-lints`; a narrow golden run that reaches scenario spawn is an acceptable preflight once the owning test exists. When adding new authoring substrate, prefer a focused spawn/schema unit test that exercises the new field directly. `scenario-coverage` currently scans all `scenarios/*.ron` and does not provide a single-file `--scenario` preflight, so do not rely on that flag unless the live binary adds it.

Treat scenario-spawn lint failures as part of the authoring loop, not as late proof surprises. If a lint reports unreachable authored drive/profile behavior, either author a minimal reachable profile value that remains truthful to the row or remove the profile if it is not part of the scenario contract.
