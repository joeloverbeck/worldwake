---
name: scenario-roadmap-landing
description: "Implement a planned scenario from `docs/scenario-roadmap.md` as a true roadmap landing. Use when the user asks to implement or turn an Ordered Roadmap scenario such as `survival-drive-escalation` or `survival-tell` into a landed scenario with an authored `scenarios/*.ron`, a truthful `golden_*.rs` suite, architectural-gap tickets if needed, and synchronized roadmap/generated docs."
---

# Scenario Roadmap Landing

Use this skill when the user asks to implement one scenario row from [`docs/scenario-roadmap.md`](../../../docs/scenario-roadmap.md), especially requests like:

- `Implement the 'survival-tell' scenario from the Ordered Roadmap`
- `Turn the 'survival-drive-escalation' scenario from the Ordered Roadmap section into a true roadmap landing`

This workflow is stricter than ordinary feature implementation. The goal is not merely "add a scenario" or "get a golden green". The goal is to land one roadmap row truthfully:

1. the authored scenario structurally activates the intended gameplay feature(s)
2. the golden proves the intended mechanic behavior and survival-health contract
3. the golden proves the authored causal branch, not a rival lawful pass
4. the scenario-backed golden runs through the appropriate CI workflow under `.github/workflows/`, not the regular local/workspace lanes
5. `docs/scenario-roadmap.md` and generated companions tell the same true story

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), [docs/scenario-roadmap.md](../../../docs/scenario-roadmap.md), [docs/golden-e2e-testing.md](../../../docs/golden-e2e-testing.md), [tickets/README.md](../../../tickets/README.md), and [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) before editing.

## Top Rules

- Resolve the exact roadmap row first. Do not start coding from the user phrase alone.
- Treat roadmap landing as a three-layer contract: structural activation, behavioral proof, and causal validity.
- A 1440-tick survival pass is necessary but not sufficient.
- Scenario-backed proof must assert the mechanic's intended behavior, not just the presence of a goal or action name.
- Scenario-backed roadmap goldens are CI-owned long-running suites. They must be wired through `.github/workflows/golden-<family>.yml`, marked `#[ignore]` for ordinary local/workspace lanes, and not treated as regular-lane coverage.
- If the live architecture cannot yet support the truthful golden, create or update ticket(s) in `tickets/` instead of weakening the scenario, weakening the golden, or falsely marking the roadmap row landed.
- Keep the roadmap and generated docs truthful in the same pass. Do not leave doc drift behind.

## Expected Outputs

Depending on live feasibility, this workflow should end with one of these outcomes:

1. **Full landing**
   - `scenarios/<scenario>.ron`
   - `crates/worldwake-ai/tests/golden_<scenario_snake>.rs`
   - `.github/workflows/golden-<family>.yml` updated or created so the suite runs in CI
   - any required production code changes
   - generated doc refreshes
   - `docs/scenario-roadmap.md` updated to `Landed`
2. **Truthful partial progress with blockers**
   - authored scenario and/or partial proof changes only if they match a truthful narrower seam
   - one or more new or updated tickets in `tickets/` for the architectural gap
   - `docs/scenario-roadmap.md` updated to reflect `Drafting` / `In Progress` / auxiliary status rather than a false landing
3. **Reassessment only**
   - if the requested row is already landed or the row name is wrong, answer with the exact live status and correct path/name before changing code

## Naming Contract

For roadmap scenarios, follow the existing naming pattern unless live repo conventions have changed:

- scenario file: `scenarios/<roadmap-name>.ron`
- golden file: `crates/worldwake-ai/tests/golden_<roadmap_name_with_underscores>.rs`
- CI family workflow: `.github/workflows/golden-<family>.yml`

Examples:

- `survival-drive-escalation` -> `scenarios/survival-drive-escalation.ron` and `golden_survival_drive_escalation.rs`
- `survival-tell` -> `scenarios/survival-tell.ron` and `golden_survival_tell.rs`

Before assuming a new file is required, check whether an existing auxiliary or partial scenario/golden already owns part of the contract.
Also check whether the scenario belongs in an existing family matrix workflow or needs a new family workflow. Follow the existing repo convention shown by `golden-survival.yml` and `golden-drive-escalation.yml`: one matrix workflow per long-running scenario family.

## Workflow

### 0. Resolve the exact roadmap row and current live status

Start from `docs/scenario-roadmap.md`:

1. Find the exact row in `Ordered Roadmap`.
2. Read any existing detailed planned-entry summary and any auxiliary/non-roadmap section that already overlaps it.
3. Identify:
   - requested scenario name
   - intended new feature scope
   - current status (`Planned`, `Drafting`, `In Progress`, `Landed`, auxiliary only)
   - dependencies on earlier landed rows
4. If the row name is slightly wrong, resolve it by exact live roadmap text instead of treating the task as blocked.
5. If the row is already landed, report that directly and stop unless the user asked for a reassessment or repair.

Before coding, state a short checkpoint in your own working notes:

- discrepancy class: missing scenario, missing golden, stale roadmap, failing proof, or architectural blocker
- authoritative boundary: exact gameplay feature rows and survival contract this landing owns

### 1. Map the mechanic contract before authoring

Use `Gameplay Feature Catalog` in `docs/scenario-roadmap.md` as the editorial source of truth for which mechanics the row is meant to land.

For the requested row:

1. Enumerate the exact feature rows it is meant to activate or upgrade.
2. Name the exact backing systems, goal/action families, and authored substrate that must be present.
3. Check `crates/worldwake-cli/src/bin/scenario_coverage.rs` when needed to verify the live structural activation rule.
4. Check existing scenarios and goldens to see what is already proven, what is only structurally active, and what is only auxiliary evidence.

Do not collapse these categories:

- structurally active in `scenario-coverage`
- behaviorally proven in a golden
- truly landed in the roadmap

### 2. Reassess the live branch before editing

Inspect the current state across:

- `scenarios/*.ron`
- existing `golden_*.rs` files
- generated docs under `docs/generated/`
- any relevant production modules in `worldwake-core`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli`

Answer these questions before writing code:

1. Does a scenario for this row already exist?
2. Does a golden already exist in full or auxiliary form?
3. Which gameplay mechanics are already structurally active under the generator?
4. Which intended mechanic behaviors are not yet proven at the strongest honest surface?
5. Are there already architectural blockers that make a truthful pass impossible today?

If the requested row depends on prior landed survival substrate, verify that substrate remains truthful under live code rather than assuming the roadmap prose is still sufficient.

### 3. Author the scenario as a survival-contract scenario

When the row is not already satisfied, create or revise the scenario in `scenarios/`.

Requirements:

- it must be a real survival scenario designed to run for 1440 ticks
- it must preserve the prior landed survival loop while adding one new architectural stressor
- it must activate the intended gameplay feature(s) through authored state, not test-only helpers
- it must define a truthful `survival_health_contract` when it is intended to be a roadmap landing

Scenario design rules:

- Copy the closest landed scenario only as a starting point; do not cargo-cult its envelope unchanged.
- Prefer minimal authored setup that still forces the mechanic under test to matter.
- Explicitly isolate rival lawful branches only when they would obscure the owned contract.
- If a competing branch is part of the architecture contract, keep it and prove the branching behavior instead.
- For cumulative mechanics, name the concrete threshold/cadence/capacity math in the scenario-owning ticket or notes before trusting the setup.

For social or belief-transport rows, verify the information path explicitly. A scenario is invalid if the intended behavior only works through omniscient setup assumptions.

### 4. Write the golden around the mechanic, not the label

Use [docs/golden-e2e-testing.md](../../../docs/golden-e2e-testing.md) as the canonical rulebook.

Every roadmap golden must prove both:

1. the survival-health contract authored in the scenario
2. the scenario-specific mechanic contract at the earliest honest causal surface

The key rule is: prove what the mechanic intends.

Examples:

- `tell` landing: prove belief transfer, listener-side behavioral consequence, and accepted vs excluded rival info paths; do not stop at "a tell goal existed"
- `trade` landing: prove sellers tend lots, buyers attempt purchases, successful trades commit when preconditions hold, and rejected/failed trades fail for lawful reasons; do not stop at "a trade goal existed"
- `drive escalation` landing: prove the critical-need escalation branch changes behavior in the intended way while preserving the survival envelope; do not stop at "wash happened eventually"

Assertion rules:

- Prefer the earliest semantic proof surface that proves the contract.
- Use authoritative world state for durable outcomes.
- Use action traces for lifecycle and same-tick ordering.
- Use decision traces for candidate-generation, suppression, ranking, or omission claims.
- Use event-log assertions only when record/public visibility is itself the contract.
- Read survival-health bounds from the scenario file rather than restating local constants.

For every scenario-backed golden, explicitly identify:

1. intended invariant or branch
2. earliest proof surface
3. lawful competing branches
4. which branches are intentionally excluded from setup
5. which remaining rival branches are accepted vs invalid

Prefer adding a deterministic replay companion unless the scenario already has a truthful reason not to.

CI ownership rules for these scenario-backed goldens:

- mark each long-running scenario test `#[ignore = "..."]` so ordinary `cargo test --workspace` lanes skip it
- keep the ignore message truthful and CI-oriented, matching the existing family pattern
- wire the suite into the family matrix workflow under `.github/workflows/`
- if the scenario belongs to an existing family such as `survival` or `drive-escalation`, append it to that matrix instead of creating a redundant workflow
- if the scenario starts a genuinely new family, create `.github/workflows/golden-<family>.yml` following the existing family-per-matrix convention

The owning workflow should run the ignored test explicitly in CI, typically in the same shape as the existing workflows:

```bash
cargo test --release -p worldwake-ai --test golden_<family>_<scenario> -- --ignored --test-threads=1
```

Do not treat local non-ignored execution as the canonical lane for these roadmap scenarios.

### 5. Make the landing pass if the architecture allows it

Treat this as an implementation workflow, not a design memo. If code changes are required to make the scenario and golden truthful, implement them.

Common work includes:

- production code changes to make the mechanic actually work under survival pressure
- scenario revisions when the authored substrate is insufficient
- golden revisions when assertions are too weak or prove the wrong branch
- CI workflow updates so the new scenario runs in the correct family workflow and stays out of regular lanes
- helper extraction or trace usage needed to assert the correct boundary

Run the narrowest truthful verification first, then expand.

Always discover exact test selectors before writing final proof commands:

```bash
cargo test -p worldwake-ai -- --list
```

Then use the narrowest real commands that match the changed files and owned behavior.

Keep Cargo commands sequential.
For long-running roadmap scenarios, local execution is for targeted/manual proof only; the canonical automation path is the CI workflow in `.github/workflows/`.

### 6. If a real architectural blocker appears, create ticket ownership instead of faking green

If the truthful roadmap landing is blocked by a genuine missing substrate or architectural contradiction:

1. Stop weakening the scenario or golden.
2. Identify the exact contradiction and owning abstraction boundary.
3. Create or update one or more tickets in `tickets/` from `tickets/_TEMPLATE.md`.
4. Align those tickets with `docs/FOUNDATIONS.md`.
5. Name the exact production symbols, live mismatch, proof surfaces, and commands.
6. Keep the active roadmap row truthful: `Planned`, `Drafting`, or `In Progress`, not `Landed`.

Ticket rules for this workflow:

- the ticket should describe the production contradiction, not merely "golden fails"
- classify the gap as missing substrate, broken information path, planner/execution failure, invalid setup math, or other real architectural cause
- if the scenario/golden proved only a narrower honest slice, rewrite the row and ticket boundaries immediately rather than leaving the original broader claim in place

Do not create filler tickets. The ticket must own a real architectural gap whose resolution would make the roadmap landing possible.

### 7. Refresh generated docs and roadmap truth in the same pass

When scenario, golden, or family workflow files change, refresh the generated companions that feed the roadmap:

```bash
cargo run -p worldwake-cli --bin scenario-coverage -- --write
python3 scripts/golden_inventory.py --write --check-docs
```

If these fail because of local annotation drift or generated-doc drift you just introduced, fix that and rerun. Do not leave stale generated artifacts behind.

Then update `docs/scenario-roadmap.md` as needed. Common sections that need edits:

- `Gameplay Feature Catalog`
- `Status Summary`
- `Ordered Roadmap`
- `Planned Entry Summaries`
- `Landed Scenarios`
- `Auxiliary and Non-Roadmap Scenarios`
- `Maintenance Workflow` or detection appendix when the generator rule itself changed

Update the roadmap to match the true result:

- mark the row `Landed` only if scenario, golden, and generated companion all agree
- if the outcome is narrower, rewrite the row rather than overclaiming
- if an auxiliary scenario now became a true roadmap row, move or rewrite the auxiliary caveat accordingly

Also make the CI ownership truthful in the same pass:

- update an existing family workflow matrix entry when the new scenario belongs to that family
- create the new family workflow when no correct family exists yet
- keep the regular `ci.yml` lanes relying on ignored-by-default behavior rather than inlining these long-running tests there

## Guardrails

- Do not claim a roadmap landing based only on structural activation.
- Do not claim a roadmap landing based only on a 1440-tick survival pass.
- Do not prove a mechanic with action-name presence when the real contract is a world consequence or belief change.
- Do not leave a new roadmap scenario golden only as a regular test target without the matching `.github/workflows` family wiring.
- Do not hide a blocker by broadening tolerances, weakening assertions, or inventing helper-only setup.
- Do not update `docs/scenario-roadmap.md` as if the row landed when the golden still fails.
- Do not leave generated docs stale after changing scenario or golden metadata.
- Do not trust old scenario-roadmap prose over live generator rules and live code when they conflict; rewrite the roadmap first.

## Closeout Checklist

Before finishing, verify which of these are true:

- exact roadmap row resolved and reassessed
- scenario file exists or was updated truthfully
- golden file exists or was updated truthfully
- the scenario-backed golden is ignored in ordinary lanes and wired into the correct CI family workflow
- golden proves survival-health contract from authored scenario data
- golden proves the mechanic's intended branch at the strongest honest surface
- deterministic replay coverage added or consciously justified
- generated scenario coverage refreshed
- golden inventory/docs refreshed
- roadmap sections updated to match the live outcome
- blocker tickets created or updated when architecture prevented full landing
- final report states whether the row is now `Landed`, still `Drafting`/`In Progress`, or blocked behind named ticket(s)

## Report Format

Use a concise closeout shaped like this:

```markdown
# Scenario Roadmap Landing: <scenario-name>

## Reassessment
- <current row status, live overlap, and exact owned mechanics>

## Outcome
- <landed / in progress / blocked>
- <scenario, golden, production, and doc changes>

## Verification
- <exact commands actually run>
- <what each command proved>

## Follow-ups
- <tickets created or updated, if any>
```

If the row did not fully land, say that directly. Name the blocker and the owning ticket instead of implying success.
