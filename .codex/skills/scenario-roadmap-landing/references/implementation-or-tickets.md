# Landing Implementation or Ticket Ownership (Steps 5-6)

Try to make the landing pass. If genuine architectural blockers appear, create ticket ownership instead of faking green.

## 5. Make the landing pass if the architecture allows it

Treat this as an implementation workflow, not a design memo. If code changes are required to make the scenario and golden truthful, implement them.

Common work includes:

- production code changes to make the mechanic actually work under survival pressure
- scenario authoring-surface changes when the live mechanic exists but cannot yet be authored truthfully through `worldwake-cli` scenario files (for example `crates/worldwake-cli/src/scenario/types.rs`, `crates/worldwake-cli/src/scenario/mod.rs`, `crates/worldwake-cli/src/bin/scenario_coverage.rs`, and the affected scenario/fixture tests)
- a narrow scenario-spawn or schema-focused unit test when new authoring substrate is added, so authored-surface regressions do not depend only on rerunning the full 1440-tick roadmap golden
- scenario loader/name-resolution ordering fixes when a truthful authored field must refer to an entity spawned later in the canonical scenario pipeline; keep the authored mechanic and repair the loader boundary rather than replacing the field with a weaker setup
- scenario revisions when the authored substrate is insufficient
- golden revisions when assertions are too weak or prove the wrong branch
- CI workflow updates or verification so the new scenario runs in the correct family workflow and stays out of regular lanes
- helper extraction or trace usage needed to assert the correct boundary

When a scenario landing exposes a production contradiction in candidate generation, ranking, action commit, handoff state, or another lower layer, add the narrowest focused test at that layer before relying on the 1440-tick roadmap golden as proof. The long golden should prove row integration, not be the only regression surface for a localized production contract.

When a proof attempt falsifies the planned information path or invocation surface, pause before changing the meaning of the retained proof. State the single contradiction, three viable options, and one recommendation under the repo's 1-3-1 rule, then wait for confirmation. Examples include remote testimony that only works as same-place testimony, a non-colocated witness branch that only works when colocated, or an externally requested action that only proves an auxiliary helper path.

Run the narrowest truthful verification first, then expand.

Discover exact test selectors before writing final proof commands when you are
using existing tests, ambiguous filters, or module-qualified names:

```bash
cargo test -p worldwake-ai -- --list
```

For newly authored tests with obvious exact names, it is fine to run the new
test directly. In all cases, use the narrowest real commands that match the
changed files and owned behavior.

Keep Cargo commands sequential.
For long-running roadmap scenarios, local execution is for targeted/manual proof only; the canonical automation path is the CI workflow in `.github/workflows/`.

## 6. If a real architectural blocker appears, create ticket ownership instead of faking green

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
