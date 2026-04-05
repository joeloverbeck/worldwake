# S58: Golden Gaps — Artifact Issuance Goals

**Status**: COMPLETED

## Summary

Post-implementation golden gap analysis for S51 (Social Artifact Issuance Goals). The live suite now proves autonomous institutional bounty posting through Scenario 112, manual threat-warning notice downstream route change through Scenario 107, and notice-driven political uptake through Scenario 109. One materially different S51 emergence chain remains unproved at golden E2E level:

1. autonomous high-danger `ThreatWarning` notice posting that then changes another agent's downstream route choice through the posted artifact path

This is not a subtype checklist gap. It is the remaining cross-system contract that still distinguishes S51's live autonomous notice issuance from the already-covered manual notice and autonomous bounty surfaces.

## Scenario: Autonomous Threat-Warning Notice Reroutes Later Travel

An AI agent under real local danger autonomously posts a `ThreatWarning` notice, a second agent later perceives the posted artifact locally, and that notice changes the second agent's next planned travel branch.

### Description

1. A local AI issuer at a threatened place has non-zero `notice_posting_weight` and enough live danger substrate to lawfully emit `GoalKind::PostNotice { topic: NoticeTopic::ThreatWarning { place }, .. }`.
2. The issuer selects the posting goal through the normal AI pipeline and commits `post_notice`.
3. A second agent with an ordinary acquisition or travel motive later arrives or resumes AI at the posting place, perceives the active notice artifact, and internalizes the warning through the existing artifact-belief path.
4. The second agent's next selected plan avoids the shorter warned route and chooses the safer lawful branch instead.

### GoalKinds Exercised

- `PostNotice`
- `AcquireCommodity(SelfConsume)` or another existing route-sensitive travel goal

### ActionDomains Exercised

- `Social` — `post_notice`
- `Travel` — route choice altered by the warning
- `Production` or `Needs` — whichever downstream consumer path is used to make the reroute matter

### Systems Exercised

- **AI candidate generation / ranking / admission**: autonomous high-danger notice emission
- **Social artifact actions**: `post_notice` commit and artifact creation
- **Perception / belief**: local notice discovery as `believed_artifact`
- **Route-threat / planning**: downstream route choice changes because of the perceived warning

### Setup Requirements

- One AI issuer already co-located with the posting place and under enough live danger to lawfully emit the current `ThreatWarning` notice family
- One downstream traveler whose route-sensitive goal would normally prefer the shorter warned branch without the notice
- Topology with at least two lawful routes so the warning changes an actual plan choice
- No author-side direct request for `post_notice`; the notice must come from autonomous AI selection

### What Emergence It Demonstrates

This proves the remaining autonomous issuance half of S51's current live contract: not just that notices can exist, but that an agent can autonomously decide to broadcast danger through a social artifact and have that new artifact reshape another agent's later planning through the existing belief and route-threat substrate.

### Foundation Principle Alignment

- **Principle 1**: the route adaptation emerges from a locally caused social artifact, not authored branching
- **Principle 7**: both the posting decision and the downstream consumer depend on local knowledge and local artifact discovery
- **Principle 14**: both agents act on beliefs rather than authoritative omniscience
- **Principle 25**: the notice is a first-class social artifact whose existence matters at runtime

### Why It Is Not A Duplicate

- **Scenario 107** proves manual `post_notice` followed by downstream route change, not autonomous notice issuance.
- **Scenario 109** proves manual vacancy notice uptake into politics, not danger-warning route adaptation.
- **Scenario 112** proves autonomous institutional bounty posting, not the notice side of S51's live issuance surface.

## Ticket Breakdown

### S58GOLGAP-001: Autonomous threat-warning notice golden closeout

- Add a golden scenario plus deterministic replay companion for autonomous high-danger `ThreatWarning` notice posting
- Assert:
  - the issuer selects `PostNotice` through the live AI path
  - `post_notice` commits without external request injection
  - the downstream agent perceives the posted notice artifact locally
  - the downstream plan reroutes away from the warned branch through the existing route-threat surface

**Files**: `crates/worldwake-ai/tests/golden_integration.rs`
**Effort**: Medium

## Tests

- [ ] autonomous threat-warning notice posts through the live AI pipeline
- [ ] downstream traveler reroutes because of the posted warning artifact
- [ ] deterministic replay companion for the autonomous notice scenario

## Acceptance Criteria

1. The golden proves autonomous `PostNotice` selection and committed notice creation without an external request shortcut
2. The same scenario proves a downstream planning consequence from the posted artifact, not just artifact existence
3. The scenario includes a deterministic replay companion
4. Assertions use the strongest honest surfaces available: decision traces for posting selection, action traces for notice commit, and plan/route proof for the downstream reroute

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Landed the bounded AI production fix needed for the spec’s intended contract: autonomous `ThreatWarning` producers no longer require `posting_place == warned_place`, and now support locally posting about a different believed dangerous place.
  - Added focused AI proof in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs` for remote warned-place notice emission and motive.
  - Added Scenario 113 plus deterministic replay coverage in `crates/worldwake-ai/tests/golden_integration.rs`, proving the full chain from autonomous notice selection and committed `post_notice` through downstream local perception and route reroute.
  - Refreshed generated golden coverage docs in `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md`.
- **Deviations from original plan**:
  - The original gap spec implied a golden-only closeout, but implementation showed the live producer contract was narrower than the existing downstream route-threat consumer, so the work had to absorb a bounded AI semantic-parity fix before the golden could honestly pass.
  - The final scenario isolated autonomous notice posting by suppressing a lawful competing `ShareBelief` branch in the harness and by seeding explicit remembered combat-activity belief to provide enough remote threat substrate.
- **Verification**:
  - `cargo test -p worldwake-ai posting_candidates_emit_threat_warning_notice_for_remote_warned_place_from_belief -- --nocapture`
  - `cargo test -p worldwake-ai post_notice_goal_has_non_zero_motive_for_remote_warned_place_from_belief -- --nocapture`
  - `cargo test -p worldwake-ai golden_s58_autonomous_notice_reroutes_later_travel -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
