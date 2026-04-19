# SCEROAD-001: Build `scenario-coverage` binary + generated companion

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — new tooling binary under `crates/worldwake-cli/src/bin/`; reads scenario files via the existing `ScenarioDef` deserializer and emits markdown. No simulation or world-state mutations.
**Deps**: None

## Problem

Goldens are now backed by `scenarios/*.ron` observer runs (see design doc `docs/plans/2026-04-19-scenario-roadmap-doc-design.md`, §Brainstorm Context). Without a machine-readable coverage snapshot, designers must read every RON by hand to judge which gameplay features are truly active — and profiles like `TellProfile` are easy to misclassify as "covered" when every gating field is zero. The editorial roadmap (SCEROAD-002) needs an evidence companion that CI can diff to keep doc and scenarios in lockstep.

## Assumption Reassessment (2026-04-19)

1. `crates/worldwake-cli/src/scenario/types.rs` defines `ScenarioDef`, `AgentDef`, `SurvivalHealthContractDef`, `PlaceDef.visibility_profile`, `ResourceSourceDef.capacity`, etc. Confirmed. All profile types referenced by the spec's §3 catalog and §7 detection rules are importable from `worldwake_core` (`UtilityProfile`, `TellProfile`, `MetabolismProfile`, `CommunicationProfile`, `PerceptionProfile`, `MerchandiseProfile`, `TheftDispositionProfile`, `JusticeDispositionProfile`, `ViolationDispositionProfile`, `PatrolProfile`, `PursuitProfile`, `CombatProfile`, `ArtifactPostingProfile`, `ObligationSatiationProfile`, `DriveEscalationProfile`, `DiversificationProfile`, `PreferenceProfile`, `ContentionDispositionProfile`, `CommodityValuationProfile`, `SubstitutePreferences`, `DisposalProfile`, `LastSeenMemory`, `ExpectationStore`, `EpistemicDispositionProfile`, `IntentionDispositionProfile`). Verified via grep against `crates/worldwake-core/src/`.
2. `scenarios/` contains exactly 5 files today: `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`, `drive-escalation-wash-priority.ron`, `cli-evaluation.ron`. Only the first three declare `survival_health_contract`; `cli-evaluation.ron` and `drive-escalation-wash-priority.ron` intentionally omit it — matches design doc §5.
3. Shared abstraction boundary under audit: the `ScenarioDef` deserializer in `crates/worldwake-cli/src/scenario/types.rs`. This ticket consumes it read-only; it does not add a parallel schema. `ron = "0.8"`, `serde`, `clap = "4"` are already workspace dependencies of `worldwake-cli` per `crates/worldwake-cli/Cargo.toml` — no new crate additions required.

Additional notes:
- No existing `crates/worldwake-cli/src/bin/scenario_coverage.rs` or `docs/generated/scenario-coverage.md` — both are new.
- Existing `crates/worldwake-cli/src/bin/observer.rs` demonstrates the `clap::Parser`-driven binary pattern used in this crate; follow that shape.
- Binary command name is written `scenario-coverage` (hyphen) in the design doc's CLI examples while the source file is `scenario_coverage.rs` (underscore). This requires a `[[bin]]` entry in `crates/worldwake-cli/Cargo.toml` mapping `name = "scenario-coverage"` to `path = "src/bin/scenario_coverage.rs"`, OR naming the source file with a hyphen. Either is acceptable; pick one and stay consistent across this ticket, SCEROAD-002, and SCEROAD-003.

## Architecture Check

1. **Single source of truth for feature detection.** The `const FEATURES: &[FeatureDef]` table inside the binary is the evidence companion to §3 of the roadmap doc. Detection logic routes through that one table — no scattered per-feature classifiers. This makes the doc/catalog lockstep auditable.
2. **Tooling boundary (FOUNDATIONS Principle 28).** The binary reads `ScenarioDef` via the canonical deserializer and writes markdown. It never mutates simulation state, never introduces a parallel schema, and never reaches into authoritative components.
3. **Forward-compatible via serde defaults.** When new profile fields land, deserialization keeps working; unrecognized top-level fields surface as a warning row in the generated file — prompting a `FEATURES` catalog update without silently hiding drift.
4. **Determinism.** Aggregation uses `BTreeMap`/`BTreeSet` throughout; markdown emission iterates scenarios in filesystem-sorted order and features in `FEATURES` declaration order. Consistent with the project rule (`CLAUDE.md` → Critical Invariants → determinism) even though this binary is tooling.
5. **No backwards-compat shim.** There is no prior generator to maintain parity with.

## Verification Layers

1. Feature detection correctness per scenario → unit tests in `scenario_coverage.rs` exercising each scenario file and asserting Active/PresentInactive/Absent classification for representative features.
2. Generated output determinism → targeted unit test that runs the generator twice on fixed input and asserts byte-identical output; plus `--check` round-trip that re-runs generation and diffs against the committed file.
3. Drift detection → targeted unit test that supplies a `ScenarioDef` with a field name not in any `FeatureDef` and asserts the warning row is emitted.
4. Single-layer ticket: all verification is local to the binary itself; it has no authoritative-layer or planner surface to map.

## What to Change

### 1. New binary `crates/worldwake-cli/src/bin/scenario_coverage.rs`

Structure (following `observer.rs` conventions):

- `Args` via `clap::Parser`: `--write` (overwrite committed file), `--check` (exit non-zero on drift), default (print to stdout).
- `FeatureDef { name: &'static str, required_profiles: &'static [ProfileKey], gating_fields: &'static [GatingField], world_conditions: &'static [WorldCondition] }` with `ProfileKey`/`GatingField`/`WorldCondition` enums covering exactly the cases in design doc §3 and §7.
- `const FEATURES: &[FeatureDef] = &[ ... ]` enumerating all features from §3: Basic needs (Eat/Drink/Sleep/Relieve/Wash), Travel physiology, Drive escalation, Need-driven exploration, Activation-decay perception, Place concealment, Tell, Ask-about-person, Consult-record, Obligation satiation, Diversification, Experience preferences, Production, Merchant selling, Trade negotiation, Commodity valuation, Substitute preferences, Item decay, Disposal, Facility-queue contention, Offices/succession, Bounty posting, Notice posting, Theft, Justice, Violation investigation, Patrol, Pursuit, Combat, Escort, Bandit camps, Report/witness, Search, Stock/transport.
- Classification per-agent then aggregated per-scenario per design doc §9 core logic: `Active` iff ≥1 agent satisfies all gates; `PresentInactive` iff every agent with the profile has it zeroed/defaulted; `Absent` iff no agent has the profile.
- World-feature gates: `commodity_decay`, `PlaceDef.visibility_profile.base_concealment > 0`, `OfficeForceProfile` presence on any spawned office entity, explicit `ContentionDispositionProfile` presence.

### 2. Markdown emission matching design doc §8

- Header notice: `<!-- Generated by \`cargo run -p worldwake-cli --bin scenario-coverage -- --write\`. -->` + `<!-- Do not hand-edit. -->`.
- Feature × Scenario matrix with scenarios as columns (filesystem-sorted), features as rows (in `FEATURES` declaration order), cells `✅` / `⚠` / `—`, legend block beneath.
- Per-scenario detail: seed, agent count by control source, place count, survival contract summary (or "absent"), Active profiles list, Present-but-inactive list, Omitted profiles list, World features (commodity_decay status, visibility_profile places, facilities & resource sources counts, known_recipes union).
- Warning row at top if any scenario has unrecognized fields.

### 3. Cargo integration in `crates/worldwake-cli/Cargo.toml`

Add `[[bin]] name = "scenario-coverage" path = "src/bin/scenario_coverage.rs"` (if keeping the underscore file name and the hyphenated command name). Alternatively, rename the file — pick one convention.

### 4. First-run output at `docs/generated/scenario-coverage.md`

Run `cargo run -p worldwake-cli --bin scenario-coverage -- --write` once and commit the resulting file. This is the empirical snapshot for the 5 current scenarios and the reference CI will diff against.

### 5. Unit tests in `scenario_coverage.rs`

Per-scenario detection smoke tests (e.g., `survival-baseline` has UtilityProfile.enterprise_weight `PresentInactive` and `CombatProfile` `Absent`; `cli-evaluation` has `MerchandiseProfile` `Active`; a scenario with a Tell-zero profile is `PresentInactive` for Tell), determinism test (generate twice, compare), unknown-field warning test (synthetic `ScenarioDef` with surprise data).

## Files to Touch

- `crates/worldwake-cli/src/bin/scenario_coverage.rs` (new)
- `crates/worldwake-cli/Cargo.toml` (modify — add `[[bin]]` if using underscore file + hyphen command)
- `docs/generated/scenario-coverage.md` (new — first committed output)

## Out of Scope

- CI workflow integration — handled by SCEROAD-003.
- Hand-authored roadmap doc at `docs/scenario-roadmap.md` — handled by SCEROAD-002.
- Changes to `ScenarioDef` or any profile schema.
- Authoring new `.ron` scenarios or new goldens.
- Reporting coverage for features outside `AgentDef` + top-level `ScenarioDef` (e.g., runtime-only state) — design doc §3 scope is profile-struct-driven activation.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: detection for `scenarios/survival-baseline.ron` classifies `UtilityProfile.enterprise_weight` as `PresentInactive`, `CombatProfile` as `Absent`, and basic-needs features as `Active`.
2. Unit test: detection for `scenarios/cli-evaluation.ron` classifies `MerchandiseProfile`, `CombatProfile`, `TradeDispositionProfile`, `PatrolProfile` as `Active` (at least one agent activates each).
3. Unit test: `--check` mode exits 0 when `docs/generated/scenario-coverage.md` matches freshly generated output and non-zero when they diverge (use a tempfile-backed round-trip).
4. Unit test: generator is deterministic — two back-to-back runs produce byte-identical output.
5. Unit test: a `ScenarioDef` with a profile field absent from every `FeatureDef` surfaces a warning row in the header.
6. Existing suite: `cargo test -p worldwake-cli` green.

### Invariants

1. Every row of design doc §3 has a corresponding `FeatureDef` entry in `FEATURES` — count and name parity.
2. Generator uses `BTreeMap`/`BTreeSet` in authoritative output assembly; no `HashMap`/`HashSet` in paths that affect row order.
3. `--check` exits non-zero whenever committed and freshly-generated content differ by any byte.
4. Binary never mutates simulation state and performs no `World::*` mutating calls.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/scenario_coverage.rs` — in-file `#[cfg(test)] mod tests` covering per-scenario classification, determinism, `--check` round-trip, unknown-field warning.

### Commands

1. `cargo test -p worldwake-cli scenario_coverage` — targeted.
2. `cargo run -p worldwake-cli --bin scenario-coverage -- --write` — regenerates committed file; `git diff docs/generated/scenario-coverage.md` must be empty.
3. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` — must exit 0.
4. `./scripts/verify.sh` — full workspace verification (will not yet invoke `--check` until SCEROAD-003 lands).
