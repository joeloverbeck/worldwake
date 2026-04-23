# Reassessment: Roadmap Row, Mechanic Contract, Live Branch

Three reassessment passes run before any authoring: resolve the row, map the mechanic contract, and inspect the live branch.

## 0. Resolve the exact roadmap row and current live status

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

## 1. Map the mechanic contract before authoring

Use `Gameplay Feature Catalog` in `docs/scenario-roadmap.md` as the editorial source of truth for which mechanics the row is meant to land.

For the requested row:

1. Enumerate the exact feature rows it is meant to activate or upgrade.
2. Name the exact backing systems, goal/action families, and authored substrate that must be present.
3. Verify that the row's feature label and activation prose still match the live mechanic contract. If the roadmap wording overstates the shipped seam, narrow the roadmap/generator wording first instead of forcing implementation to satisfy stale editorial text.
4. Check `crates/worldwake-cli/src/bin/scenario_coverage.rs` when needed to verify the live structural activation rule.
5. Check existing scenarios and goldens to see what is already proven, what is only structurally active, and what is only auxiliary evidence.
6. Check whether the row's required substrate will also make later roadmap rows structurally active under the generator. If it does, record that upfront and keep those rows separate from the requested landing unless the new golden actually proves them.

Do not collapse these categories:

- structurally active in `scenario-coverage`
- behaviorally proven in a golden
- truly landed in the roadmap

## 2. Reassess the live branch before editing

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
