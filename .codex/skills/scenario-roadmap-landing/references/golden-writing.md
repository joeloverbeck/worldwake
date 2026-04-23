# Writing the Golden (Step 4: Around the Mechanic, Not the Label)

Use [docs/golden-e2e-testing.md](../../../../docs/golden-e2e-testing.md) as the canonical rulebook.

## What the golden must prove

Every roadmap golden must prove both:

1. the survival-health contract authored in the scenario
2. the scenario-specific mechanic contract at the earliest honest causal surface

The key rule is: prove what the mechanic intends.

Examples:

- `tell` landing: prove belief transfer, listener-side behavioral consequence, and accepted vs excluded rival info paths; do not stop at "a tell goal existed"
- `trade` landing: prove sellers tend lots, buyers attempt purchases, successful trades commit when preconditions hold, and rejected/failed trades fail for lawful reasons; do not stop at "a trade goal existed"
- `drive escalation` landing: prove the critical-need escalation branch changes behavior in the intended way while preserving the survival envelope; do not stop at "wash happened eventually"

## Assertion rules

- Prefer the earliest semantic proof surface that proves the contract.
- Use authoritative world state for durable outcomes.
- Use action traces for lifecycle and same-tick ordering.
- Use decision traces for candidate-generation, suppression, ranking, or omission claims.
- Use event-log assertions only when record/public visibility is itself the contract.
- Read survival-health bounds from the scenario file rather than restating local constants.
- Do not silently strengthen a row-scoped invariant into a per-agent symmetry claim unless the roadmap row explicitly owns that stronger promise. One agent recurring through the mechanic may be sufficient when the row is about scenario-level coexistence rather than actor symmetry.
- When a scenario uses supporting actors, state explicitly which agents own the survival-health envelope and which agents are supporting causal actors only. Do not let supporting witnesses silently inherit full survival-contract ownership unless the roadmap row truly promises that wider envelope.

## Per-golden proof-surface classification

For every scenario-backed golden, explicitly identify:

1. intended invariant or branch
2. earliest proof surface
3. lawful competing branches
4. which branches are intentionally excluded from setup
5. which remaining rival branches are accepted vs invalid

## Deterministic replay and generated-doc parser

Prefer adding a deterministic replay companion unless the scenario already has a truthful reason not to.

Keep the repo's generated-doc parser contract in mind when authoring the golden comment block. New scenario-backed goldens should use the existing numbered `Scenario <N>:` header style expected by `scripts/golden_inventory.py`, or the generated scenario-detail docs may not materialize correctly.

## CI ownership

CI ownership rules for these scenario-backed goldens:

- mark each long-running scenario test `#[ignore = "..."]` so ordinary `cargo test --workspace` lanes skip it
- keep the ignore message truthful and CI-oriented, matching the existing family pattern
- wire the suite into the family matrix workflow under `.github/workflows/`
- if the scenario belongs to an existing family such as `survival` or `drive-escalation`, append it to that matrix instead of creating a redundant workflow
- if the scenario starts a genuinely new family, create `.github/workflows/golden-<family>.yml` following the existing family-per-matrix convention
- if the result is truthful partial progress but the roadmap-named scenario/golden remains the real owning proof surface for that row, wire it into CI now rather than leaving roadmap-owned proof orphaned from automation
- if the result is only exploratory or auxiliary evidence and is not the truthful roadmap-row owner, keep it out of roadmap-family CI and describe it that way explicitly

The owning workflow should run the ignored test explicitly in CI, typically in the same shape as the existing workflows:

```bash
cargo test --release -p worldwake-ai --test golden_<family>_<scenario> -- --ignored --test-threads=1
```

Do not treat local non-ignored execution as the canonical lane for these roadmap scenarios.
