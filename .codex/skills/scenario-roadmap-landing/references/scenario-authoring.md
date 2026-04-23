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
- For trade rows, validate the authored economy math early: merchant stock volume, buyer purchasing power, self-care supply, and carry-capacity/load limits. Do this before spending repeated long reruns on golden debugging.

## Special cases

For social or belief-transport rows, verify the information path explicitly. A scenario is invalid if the intended behavior only works through omniscient setup assumptions.

When a survival contract legitimately needs an uneven bound across need families, encode that as scenario-authored per-need overrides instead of inventing a stronger global cap and then forcing the codebase to satisfy it.

Before deeper proof/debugging, do a scenario-lint preflight on the authored file or its owning golden once. The expected boundary is the canonical scenario loader/spawn path (`load_scenario_file` + `spawn_scenario`) without `--ignore-lints`; a narrow golden run that reaches scenario spawn is an acceptable preflight once the owning test exists. Treat scenario-spawn lint failures as part of the authoring loop, not as late proof surprises.
