# S58GOLGAP-001: Autonomous threat-warning notice golden closeout

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes
**Deps**: None (S51 artifact issuance goals fully implemented; Scenarios 107, 109, 112 already cover manual notice, vacancy notice, and autonomous bounty)

## Problem

Scenario 107 proves manual `post_notice` followed by downstream route change. Scenario 112 proves autonomous institutional bounty posting. But no golden test proves the full autonomous-notice-to-downstream-reroute chain: an AI agent autonomously decides to post a `ThreatWarning` notice, and a second agent later perceives that posted artifact and reroutes away from the warned branch. This is the remaining cross-system contract distinguishing S51's autonomous notice issuance from the already-covered manual notice and autonomous bounty surfaces.

## Assumption Reassessment (2026-04-05)

1. Scenario 107 exists at `crates/worldwake-ai/tests/golden_integration.rs:6636` — proves manual post_notice + route change. Confirmed.
2. Scenario 109 exists at `crates/worldwake-ai/tests/golden_offices.rs:3268` — proves vacancy notice political uptake. Confirmed.
3. Scenario 112 exists at `crates/worldwake-ai/tests/golden_integration.rs:6674` — proves autonomous institutional bounty posting. Confirmed.
4. `GoalKind::PostNotice { topic: NoticeTopic, posting: EntityId }` exists at `crates/worldwake-core/src/goal.rs:71`. Confirmed.
5. `NoticeTopic::ThreatWarning { place: EntityId }` exists at `crates/worldwake-core/src/social_artifact.rs:79`. Confirmed.
6. `notice_posting_weight` on UtilityProfile controls autonomous notice emission — agents with zero weight never post. Confirmed from S51 implementation.
7. `post_notice` action exists at `crates/worldwake-systems/src/artifact_actions.rs`. Confirmed.
8. Route-threat / planning substrate exists — Scenario 107 already proves this mechanism works for manually-posted notices.
9. `PerceptionProfile` required on agents that need to observe posted notice artifacts.
10. `golden_integration.rs` exists at `crates/worldwake-ai/tests/golden_integration.rs` — target file. Confirmed.

### Reassessment Correction (2026-04-05, implementation)

- ticket says: this is a pure golden-closeout ticket with no production changes
- live code has: autonomous `ThreatWarning` issuance is still same-place only. `emit_notice_posting_candidates()` and `post_notice_motive()` currently require `posting.posting_place == warned_place`, while the route-threat consumer already supports a notice posted at one place warning about another.
- correction applied: broaden this ticket to include the bounded AI production fix needed to make autonomous remote threat warnings lawful before the golden can honestly prove downstream reroute
- why safe: this is the smallest shared change that matches the live spec/gap contract without distorting the scenario harness or weakening the intended proof surface

## Architecture Check

1. This is a mixed production-plus-golden ticket. The bounded production change is in the AI notice-emission path: autonomous `ThreatWarning` posting must lawfully support `posting_place != warned_place` when the issuer locally knows danger at the warned place. The golden test then exercises the full cross-crate contract: danger belief → candidate generation (AI) → PostNotice goal selection (AI) → post_notice action (systems) → artifact entity creation (core) → second agent perception (systems) → route planning change (AI). All interaction through state per Principle 26.
2. The test must ensure the notice comes from autonomous AI selection — no external request injection or human control shortcut.
3. No backward-compatibility shims.

## Verification Layers

1. Issuer selects PostNotice through live AI pipeline → decision trace (PostNotice candidate emitted and selected)
2. `post_notice` commits without external request injection → action trace (post_notice committed, no human control source)
3. SocialArtifact entity created with ThreatWarning topic → authoritative world state
4. Downstream agent perceives notice artifact locally → belief store assertion (believed_artifact with ThreatWarning)
5. Downstream agent reroutes away from warned branch → decision trace or plan trace (different route selected than the shorter warned path)
6. Multi-layer ticket: each assertion mapped to specific proof surface above.
7. Focused AI proof covers the bounded production change: autonomous notice candidate generation and ranking can target a warned place distinct from the posting place when supported by live local beliefs.

## What to Change

### 1. Broaden autonomous `ThreatWarning` issuance to support remote warned places

In `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs`:

- remove the stale same-place restriction that requires `posting.posting_place == warned_place`
- derive the warned place from the strongest live local danger belief substrate rather than forcing the issuer to warn only about its current place
- keep posting local: the issuer still posts at its current lawful posting place, but the `NoticeTopic::ThreatWarning { place }` may name a different believed dangerous place
- add focused proofs that the autonomous notice family now emits and ranks for `posting_place != warned_place`, while still staying zero-motive when the warned-place danger substrate is absent

### 2. Add golden scenario: autonomous threat-warning notice reroutes later travel

In `crates/worldwake-ai/tests/golden_integration.rs`:

**Setup**:
- 3 places: SafeHub, DangerousRoute, AlternativeRoute — topology where SafeHub connects to a destination via both DangerousRoute (shorter, e.g. 2 ticks) and AlternativeRoute (longer, e.g. 5 ticks).
- 1 AI issuer at SafeHub/Market. Has: PerceptionProfile, UtilityProfile with `notice_posting_weight > 0`, and live local belief about danger at DangerousRoute/WarnedRoad (for example a believed hostile or conflict there). Issuer is AI-controlled — no human control source.
- 1 AI traveler at SafeHub with a route-sensitive goal (e.g., AcquireCommodity at the destination). Has: PerceptionProfile, UtilityProfile with route-caution sensitivity. Would normally prefer DangerousRoute (shorter) without the warning.
- Notice posting place at SafeHub (where the traveler will perceive it).

**Execution**: Tick until issuer posts notice AND traveler makes route choice.

**Assertions**:
- Issuer generated PostNotice goal autonomously (decision trace — no external request).
- `post_notice` action committed by issuer (action trace).
- SocialArtifact entity created with `NoticeTopic::ThreatWarning { place: DangerousRoute }` (authoritative world state).
- Traveler perceived notice artifact at SafeHub (belief store — `believed_artifact` with ThreatWarning).
- Traveler's next plan uses AlternativeRoute instead of DangerousRoute (decision trace / plan trace — route choice diverges from shorter path).

### 3. Add deterministic replay companion

Same scenario with identical seed — assert identical world hash and event-log hash.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Manual notice posting tests (already covered by Scenario 107)
- Vacancy notice political uptake (already covered by Scenario 109)
- Autonomous bounty posting (already covered by Scenario 112)
- Changes unrelated to the bounded autonomous remote-warning emission contract
- Notice expiration or withdrawal scenarios

## Acceptance Criteria

### Tests That Must Pass

1. Golden: issuer autonomously selects PostNotice and commits post_notice without external request
2. Golden: downstream traveler perceives posted notice and reroutes away from warned branch
3. Deterministic replay companion produces identical outcome
4. Focused AI: autonomous notice emission and ranking support `posting_place != warned_place` when local beliefs support the warning
5. Existing suite: `cargo test --workspace`

### Invariants

1. Notice posting comes from autonomous AI selection — no human control source on issuer (Principle 1)
2. Both agents act on local beliefs — issuer posts from local danger perception, traveler reroutes from local artifact perception (Principle 7)
3. Neither agent uses authoritative world state for planning — belief-only (Principle 14)
4. The notice is a first-class social artifact whose existence reshapes downstream behavior (Principle 25)
5. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs` — Autonomous threat-warning notice golden scenario + replay companion
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused remote-warning candidate-generation proof
3. `crates/worldwake-ai/src/ranking.rs` — focused remote-warning ranking proof

### Commands

1. `cargo test -p worldwake-ai autonomous_notice`
2. `cargo test -p worldwake-ai --test golden_integration`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Broadened the ticket from a pure golden-closeout slice into a bounded AI production-plus-golden slice after reassessment showed that autonomous `ThreatWarning` producers still enforced `posting_place == warned_place` while the existing route-threat consumer already supported notices posted locally about a different place.
  - Added shared remote-threat signal helpers in `crates/worldwake-ai/src/route_threat.rs`, then reused that substrate in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs` so autonomous `PostNotice` can be posted locally about a different believed dangerous place.
  - Added focused AI proofs for remote warned-place candidate emission and motive.
  - Added Scenario 113 plus deterministic replay coverage in `crates/worldwake-ai/tests/golden_integration.rs`, proving the full chain: autonomous `PostNotice` selection, committed `post_notice` at Market about Warned Road, local notice perception, and later traveler reroute to the safe branch.
  - Refreshed generated golden coverage docs in `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md`.
- **Deviations from original plan**:
  - The original ticket claimed a golden-only closeout, but live code required a bounded production fix because autonomous notice producers and the downstream route-threat consumer disagreed on the lawful `ThreatWarning` shape.
  - The final golden setup had to suppress a lawful competing `ShareBelief` branch with `social_weight: pm(0)` on the issuer so the scenario proved autonomous notice posting rather than a different information-path outcome.
  - The remote-warning branch also needed explicit remembered combat-activity belief seeding because plain place/entity belief snapshots did not carry enough threat substrate for the intended autonomous notice candidate.
- **Verification**:
  - `cargo test -p worldwake-ai posting_candidates_emit_threat_warning_notice_for_remote_warned_place_from_belief -- --nocapture`
  - `cargo test -p worldwake-ai post_notice_goal_has_non_zero_motive_for_remote_warned_place_from_belief -- --nocapture`
  - `cargo test -p worldwake-ai golden_s58_autonomous_notice_reroutes_later_travel -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
