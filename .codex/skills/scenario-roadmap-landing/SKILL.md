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

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), [docs/scenario-roadmap.md](../../../docs/scenario-roadmap.md), [docs/golden-e2e-testing.md](../../../docs/golden-e2e-testing.md), [tickets/README.md](../../../tickets/README.md), and [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md) before editing. Then load `references/naming-contract.md` for file/workflow naming conventions and the auxiliary-promotion pattern.

## Top Rules

- Resolve the exact roadmap row first. Do not start coding from the user phrase alone.
- Treat roadmap landing as a three-layer contract: structural activation, behavioral proof, and causal validity.
- If earlier landed rows already own part of the requested mechanic family, subtract that overlap first and restate the residual row-owned seam before authoring scenario or golden code.
- If the requested row depends on prerequisite substrate that was already landed earlier, prefer authoring that substrate directly in the new scenario instead of re-proving the earlier row, unless the current row truthfully owns re-exercising it.
- If a planned extension names a concrete behavior or invocation surface that live mechanics cannot lawfully support, use the repo's 1-3-1 rule before reinterpreting the proof target: state the single contradiction, three viable paths, and one recommendation, then wait for confirmation.
- For capstone or coexistence rows such as `final-integration`, do not re-prove every earlier row's behavior in one oversized golden. Subtract earlier behavior owners, define the residual contract as full structural activation plus a representative causal pressure branch, and classify any structurally active mechanics that still lack standalone behavior proof.
- A 1440-tick survival pass is necessary but not sufficient.
- When the outcome is partial progress, distinguish explicitly between `what was implemented` and `why the row is still not Landed`.
- Scenario-backed proof must assert the mechanic's intended behavior, not just the presence of a goal or action name.
- Treat externally requested actions, payload overrides, and other human-driven helper paths as auxiliary evidence unless the roadmap row explicitly owns that invocation model.
- Scenario-backed roadmap goldens are CI-owned long-running suites. They must be wired through `.github/workflows/golden-<family>.yml`, marked `#[ignore]` for ordinary local/workspace lanes, and not treated as regular-lane coverage.
- Keep Cargo commands sequential during this workflow. Targeted tests and long-running goldens contend on Cargo package and artifact locks, so do not run Cargo build/test/check commands in parallel.
- Do not update `.github/workflows/golden-<family>.yml` until the roadmap-owned golden has a truthful retained seam: either the row is actually landing, or a narrower in-repo partial seam is intentionally being kept as the canonical owner.
- If the live architecture cannot yet support the truthful golden, create or update ticket(s) in `tickets/` instead of weakening the scenario, weakening the golden, or falsely marking the roadmap row landed.
- Keep the roadmap and generated docs truthful in the same pass. Do not leave doc drift behind.

## Expected Outputs

Depending on live feasibility, this workflow should end with one of these outcomes:

1. **Full landing**
   - `scenarios/<scenario>.ron`
   - `crates/worldwake-ai/tests/golden_<scenario_snake>.rs`
   - `.github/workflows/golden-<family>.yml` updated, created, or verified already correct so the suite runs in CI
   - any required production code changes
   - generated doc refreshes
   - `docs/scenario-roadmap.md` updated to `Landed`
2. **Truthful partial progress with blockers**
   - authored scenario and/or partial proof changes only if they match a truthful narrower seam
   - if the row owns multiple feature rows, classify each owned feature explicitly as `landed here`, `structurally active only`, or `blocked`; do not let one proven feature silently land the whole row
   - if the row is a capstone/coexistence row, classify the proof as `full structural coexistence`, `representative causal branch`, and `structural-only support mechanics` instead of claiming standalone behavior landings for every active feature
   - one or more new or updated tickets in `tickets/` for the architectural gap
   - `docs/scenario-roadmap.md` updated to reflect `Drafting` / `In Progress` / auxiliary status rather than a false landing
   - if the attempted scenario/golden does not remain a truthful retained seam, remove the draft artifacts and leave only roadmap/ticket state
3. **Reassessment only**
   - if the requested row is already landed or the row name is wrong, answer with the exact live status and correct path/name before changing code

## Workflow

1. **Reassess roadmap row, mechanic contract, and live branch.** Load `references/reassessment.md`. Resolve the exact row in `docs/scenario-roadmap.md`, map the mechanic contract against the Gameplay Feature Catalog, inspect the live branch state, and explicitly subtract any overlapping seam already owned by earlier landed rows before authoring anything. Covers original Steps 0–2.
2. **Author the scenario and run one preflight before deeper proof work.** Load `references/scenario-authoring.md`. Create or revise `scenarios/<roadmap-name>.ron` as a real 1440-tick survival-contract scenario that activates the intended mechanic through authored state, then run one scenario-lint/spawn preflight so authored-surface failures are caught before golden debugging.
3. **Write the golden around the mechanic.** Load `references/golden-writing.md`. Prove both the survival-health contract and the scenario-specific mechanic contract at the earliest honest causal surface, reject auxiliary external-request paths as roadmap proof unless the row explicitly owns them, and treat workflow wiring as deferred until that proof seam is truthful enough to retain.
4. **Make the landing pass, or create ticket ownership for real blockers.** Load `references/implementation-or-tickets.md`. Implement required production / scenario / golden changes; if a genuine architectural contradiction blocks the landing, own the gap via tickets in `tickets/` instead of weakening the contract. Name the exact live decision-pipeline boundary when the blocker is mixed AI behavior such as `GoalKind` admission, suppression, ranking, or selection. Only update `.github/workflows/golden-<family>.yml` once the retained roadmap-owned seam is truthful. Covers original Steps 5–6.
5. **Refresh generated docs and roadmap truth in the same pass.** Load `references/refresh-and-roadmap.md`. Allocate a unique `Scenario <N>:` header before running the generated-doc refresh, then regenerate companion docs, run schema fallout sweeps, classify the outcome, update CI ownership, and handle rename/promotion fallout.
6. **Close out.** Load `references/closeout.md`. Run the closeout checklist and produce the report in the required format.

### Blocked-Row Closeout Checklist

When the requested row is blocked rather than landed:

1. remove any drafted scenario/golden artifacts that do not remain a truthful retained seam
2. revert premature `.github/workflows/golden-<family>.yml` wiring
3. create or update blocker ticket(s) in `tickets/` with the exact live contradiction
4. downgrade `docs/scenario-roadmap.md` to the truthful non-landed status
5. leave generated docs untouched unless a retained truthful seam still owns them

## Guardrails

- Do not claim a roadmap landing based only on structural activation.
- Do not claim a capstone/coexistence row re-landed every earlier feature behavior merely because every feature is structurally active. Earlier rows remain the behavior owners unless the new golden intentionally proves those branches again.
- Do not claim a roadmap landing based only on a 1440-tick survival pass.
- Do not prove a mechanic with action-name presence when the real contract is a world consequence or belief change.
- Do not let a later roadmap row silently re-prove a seam already landed by an earlier row; narrow the owned seam first.
- Do not count externally requested or payload-override proof paths as roadmap-row proof unless the row explicitly owns human-driven invocation.
- Do not leave a new roadmap scenario golden only as a regular test target without the matching `.github/workflows` family wiring.
- Do not add `.github/workflows` wiring for a drafted row that may still be deleted if the live proof collapses.
- Do not hide a blocker by broadening tolerances, weakening assertions, or inventing helper-only setup.
- Do not keep an overclaimed `survival_health_contract` or roadmap row just because it was the first draft; narrow the authored contract first when focused proof falsifies it.
- Do not update `docs/scenario-roadmap.md` as if the row landed when the golden still fails.
- Do not keep draft scenario/golden files in-repo when the attempted proof does not remain a truthful retained seam.
- Do not leave generated docs stale after changing scenario or golden metadata.
- Do not reuse an existing `Scenario <N>:` id. Check the current id inventory before editing a golden header, not only after `golden_inventory.py` fails.
- Do not trust old scenario-roadmap prose over live generator rules and live code when they conflict; rewrite the roadmap first.
