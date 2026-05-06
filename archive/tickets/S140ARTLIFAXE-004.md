# S140ARTLIFAXE-004: Unified ArtifactDef scenario authoring (NoticeDef rename + payload sum type)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No engine changes; CLI scenario-author surface only. `NoticeDef` renamed to `ArtifactDef`; new `ArtifactPayloadDef` sum type; `spawn_notice` renamed to `spawn_artifact` and dispatches by kind.
**Deps**: archive/tickets/S140ARTLIFAXE-001.md

## Problem

The current scenario-author surface has only `NoticeDef` (`crates/worldwake-cli/src/scenario/types.rs:144-154`); bounties and accusations are runtime-only. With the lifecycle axes landed in 001, the spec authorial surface should reflect the engine model: artifacts are one taxonomy (FND-25: "There is no special quest system. There are only world entities and records that people create..."). Per spec D7 and the reassess-spec finding that zero `.ron` scenarios author `notices:` today, unifying `NoticeDef` into `ArtifactDef` with an `ArtifactPayloadDef` sum type carries near-zero RON migration cost while removing the fictional separation between notice and (future) bounty authoring. Future artifact classes (warrants, contracts, scenario-authored accusations) inherit the unified surface for free.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-cli/src/scenario/types.rs:144-154` defines `NoticeDef` with fields `issuer, location, issuing_authority, expires_at, jurisdiction, topic` (where `topic: NoticeTopicDef`). `ScenarioDef.notices: Vec<NoticeDef>` lives at `scenario/types.rs:39`. `crates/worldwake-cli/src/scenario/mod.rs:955-987` defines `spawn_notice`. The existing inline test `test_spawn_notice_artifact_from_scenario:1687` exercises the spawn path. Verified at reassess: zero `.ron` scenarios populate `notices:` today (`grep "notices:" scenarios/*.ron` returned no matches). No `BountyDef` or `AccusationDef` exists.
2. Spec deliverable D7 names the rename + payload sum type. The spec confirmed that bounties and accusations are runtime-only and the unification has zero `.ron` migration cost.
3. **Cross-system shared abstraction boundary**: The boundary under audit is the scenario-author authority (declared `ScenarioDef` shape consumed by RON deserialization and scenario tests) and the spawn-side translation into authoritative components (`ArtifactHeader, ArtifactPostingContext, NoticeContent` post-001). Both sides live in `worldwake-cli/src/scenario`.
4. **Information-path refactor stance**: For the same fact "this scenario authors a posted artifact", the current path is `ScenarioDef.notices: Vec<NoticeDef>` → `spawn_notice`. The post-004 canonical path is `ScenarioDef.artifacts: Vec<ArtifactDef>` → `spawn_artifact` (dispatching by kind). There is no temporary mixed-state coexistence; `NoticeDef` and `spawn_notice` are removed in this ticket.
13. **Adjacent-contradiction classification**: If implementation discovers that the existing scenario-test in `scenario/mod.rs:1687` (`test_spawn_notice_artifact_from_scenario`) constructs `NoticeDef` literals that need migration to `ArtifactDef` with `ArtifactPayloadDef::Notice`, that's a required consequence of the rename, not a separate ticket.

## Architecture Check

1. The unified `ArtifactDef` reflects FND-25's engine model — there is one artifact taxonomy, not separate parallel surfaces per kind. DRY (CLAUDE.md): repeating the 5 axis fields per Def for each future artifact class would duplicate the axis schema.
2. `ArtifactPayloadDef` as a sum type preserves type safety: each kind keeps its kind-specific data (`NoticeTopicDef` for notices, `BountyTermsDef` for bounties when authored). This is more honest than a flattened `Option<NoticeTopicDef>` + `Option<BountyTermsDef>` pair, which would allow nonsensical combinations.
3. Per FND-28: no `NoticeDef` alias is retained at the engine layer. The optional axis-state fields on `ArtifactDef` are scenario-author boundary normalization (FND-13), not engine-layer back-compat.

## Verification Layers

1. RON deserialization roundtrip → focused unit test constructing an `ArtifactDef` with each `ArtifactPayloadDef` variant and verifying RON parse + spawn produces the expected authoritative components.
2. Default axis-state semantics (omit axis fields → historical `Active` shape) → unit test asserting that `ArtifactDef::default_axes()` (or equivalent) produces `Exists | Posted | Active | Credible | Actionable`.
3. Spawn dispatch → existing scenario-test `test_spawn_notice_artifact_from_scenario` (renamed) exercises the post-004 code path with `ArtifactPayloadDef::Notice`.
4. Single-layer scope: this is a CLI scenario-author surface change. No engine semantics change; the spawn path's emitted authoritative state is identical to the pre-004 path when defaults apply.

## What to Change

### 1. Rename `NoticeDef` to `ArtifactDef` and extend with `ArtifactPayloadDef` sum + axis fields

In `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct ArtifactDef {
    pub kind: ArtifactKindDef,
    pub issuer: String,
    pub location: String,
    pub issuing_authority: Option<String>,
    pub expires_at: Option<u64>,
    pub jurisdiction: Option<String>,
    pub payload: ArtifactPayloadDef,
    pub existence: Option<ArtifactExistenceDef>,
    pub visibility: Option<ArtifactVisibilityDef>,
    pub legal_effect: Option<ArtifactLegalEffectDef>,
    pub credibility: Option<ArtifactCredibilityDef>,
    pub actionability: Option<ArtifactActionabilityDef>,
}

pub enum ArtifactPayloadDef {
    Notice(NoticeTopicDef),
    Bounty(BountyTermsDef),
    // future: Accusation(AccusationDef) when accusations are landed as artifacts
}

pub enum ArtifactKindDef {
    Notice,
    Bounty,
}
```

Define the per-axis `*Def` wrapper types (`ArtifactExistenceDef`, etc.) mirroring the engine enums but using string entity names where the engine carries `EntityId`. `BountyTermsDef` is a new wrapper analogous to existing notice-side wrappers.

### 2. Rename `ScenarioDef.notices` to `ScenarioDef.artifacts`

`scenario/types.rs:39` field rename. Per FND-28, no alias is retained; the field is renamed in-place.

### 3. Rename `spawn_notice` to `spawn_artifact` and dispatch by kind

In `crates/worldwake-cli/src/scenario/mod.rs:955-987`, rename the function and add a kind-dispatch arm:

- `ArtifactKindDef::Notice` → existing notice-construction path (now reading `payload: ArtifactPayloadDef::Notice(...)`)
- `ArtifactKindDef::Bounty` → new bounty-construction path that constructs `ArtifactHeader + BountyTerms` from `payload: ArtifactPayloadDef::Bounty(...)` and the shared header fields.

The default axis-state mapping: when an axis field is `None`, use the historical-active mapping (`Exists | Posted | Active | Credible | Actionable`).

The call site at `scenario/mod.rs:324` updates from `spawn_notice(...)` to `spawn_artifact(...)`.

### 4. Update the existing scenario test

`test_spawn_notice_artifact_from_scenario:1687` is updated (and renamed if its name carries the `notice` term) to construct `ArtifactDef { kind: Notice, payload: ArtifactPayloadDef::Notice(...) }` and assert spawn behavior is unchanged.

### 5. Add a new test for bounty authoring

Add `test_spawn_bounty_artifact_from_scenario` exercising `ArtifactDef { kind: Bounty, payload: ArtifactPayloadDef::Bounty(...) }`. This test demonstrates the unification value (a future-bounty author has no engine work to do beyond filling in `BountyTermsDef`).

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — `NoticeDef` → `ArtifactDef`; add `ArtifactPayloadDef`, `ArtifactKindDef`, `BountyTermsDef`, axis-state `*Def` wrappers; rename `ScenarioDef.notices` → `ScenarioDef.artifacts`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — `spawn_notice` → `spawn_artifact` with kind dispatch; update call site at line 324; existing test at line 1687; new bounty-spawn test)
- Likely: any `scenarios/*.ron` files using `notices:` field — verified zero today, but re-grep at implementation time before declaring no migration needed: `grep "notices:" scenarios/*.ron` (expect 0 matches)

## Out of Scope

- Engine-side artifact lifecycle (covered by 001, 002).
- Planner observability (covered by 003).
- Observer rendering (covered by 005).
- E2E goldens (covered by 006).
- `Accusation` artifact authoring — explicit Non-Goal in spec; reserved for a future spec.
- `SaleListing` migration into `ArtifactDef` — explicit Non-Goal in spec.
- Per-class custom axes — explicit Non-Goal in spec; the 5 axes apply uniformly.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_notice_artifact_from_scenario -- --exact` — passes with `ArtifactDef::Notice` shape.
2. `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_bounty_artifact_from_scenario -- --exact` — new test passes.
3. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` — coverage check passes.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. The string `pub struct NoticeDef` does not appear in `crates/worldwake-cli/src/scenario/`.
2. `ScenarioDef` carries a `pub artifacts: Vec<ArtifactDef>` field; no `pub notices:` field.
3. Default axis-state mapping when axis fields are `None`: `Exists | Posted | Active | Credible | Actionable` per the Migration Map.
4. Zero `.ron` scenarios use the pre-004 `notices:` field name (verified at implementation time).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` (modify) — rename + adapt `test_spawn_notice_artifact_from_scenario`.
2. `crates/worldwake-cli/src/scenario/mod.rs` (modify) — add `test_spawn_bounty_artifact_from_scenario`.
3. `crates/worldwake-cli/src/scenario/mod.rs` (modify) — add `test_spawn_artifact_axis_defaults_match_migration_map` asserting default-axis mapping.
4. `crates/worldwake-cli/src/scenario/types.rs` (modify) — add RON deserialization tests for artifact authors, bounty payloads, and axis wrappers.

### Commands

1. `cargo test -p worldwake-cli --lib scenario`
2. `cargo run -p worldwake-cli --bin scenario-coverage -- --check`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-05-06.

1. Replaced the CLI scenario authoring field with `ScenarioDef.artifacts: Vec<ArtifactDef>` and removed the old `ScenarioDef.notices` / `NoticeDef` authoring surface.
2. Added `ArtifactKindDef`, `ArtifactPayloadDef`, `BountyTermsDef`, bounty target/reward wrappers, and per-axis `Artifact*Def` wrappers that author the lifecycle axes with string references at the scenario boundary.
3. Renamed the spawn path to `spawn_artifact` and dispatched shared artifact authoring into `NoticeContent` or `BountyTerms` while preserving the default lifecycle mapping: `Exists | Posted | Active | Credible | Actionable`.
4. Updated scenario coverage, lint/display/handler constructor fallout, and the affected AI golden fixture constructor to the `artifacts` field.

## Verification Result

1. Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_notice_artifact_from_scenario -- --exact`
2. Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_bounty_artifact_from_scenario -- --exact`
3. Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_artifact_axis_defaults_match_migration_map -- --exact`
4. Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserializes_artifact_authors -- --exact`
5. Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_deserializes_bounty_artifact_payload_and_axes -- --exact`
6. Passed `cargo test -p worldwake-cli --lib scenario`
7. Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`
8. Passed `scripts/verify.sh`, covering formatting, workspace tests, active-goal guard, both clippy gates, and scenario coverage.
