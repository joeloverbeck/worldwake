# S97: PostNotice Artifact TTL Provisioning

## Summary

Makes the planner and goal dispatch provide profile-driven `expires_at` values for PostNotice (and PostBounty) artifacts, so that posted notices expire via the existing `artifact_lifecycle_system` instead of persisting indefinitely. Addresses the SocialArtifact pollution identified in the simulation observer report (500+ never-expiring artifacts at Dusty Trail).

## Phase and Status

Phase 7 adjunct. Status: Draft.

## Crates

- `worldwake-core` — new `ArtifactPostingProfile` component with TTL defaults
- `worldwake-ai` — goal dispatch and candidate generation set `expires_at` from profile

## Dependencies

- None. The expiry infrastructure already exists: `ArtifactHeader.expires_at`, `artifact_lifecycle_system` in worldwake-systems, `ArtifactState::Expired` transition.

## Design Goals

- Bound SocialArtifact entity count at any location over long runs.
- Profile-driven TTL so that institutional postings (via issuing_authority) can have different lifetimes than personal postings (FND-22).
- Zero infrastructure work — use the existing `expires_at` field and `artifact_lifecycle_system`.

## Non-Goals

- Changing artifact lifecycle states or transitions (already correct per FND-25A).
- Perception throttling (deferred; artifact count reduction should reduce perception load).
- Modifying PostNotice action handler (already uses `payload.expires_at`).

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-11 (Feedback Dampening) | Closes the undamped artifact accumulation loop: artifacts now expire, bounding the population. |
| FND-22 (Agent Diversity) | TTL is per-agent profile. Cautious guards post longer-lived warnings; hasty civilians post short-lived ones. |
| FND-25A (Artifact Lifecycle) | Completes the lifecycle: artifacts now transition Active → Expired via declared `expires_at`. |
| FND-8 (Every Action Has Cost) | Posting a notice that persists forever had no ongoing cost. Finite TTL means agents must re-post if the threat persists — each re-posting has duration and occupancy cost. |

## Deliverables

### D1: `ArtifactPostingProfile` component

New component in `worldwake-core`:

```rust
/// Per-agent defaults for artifact TTL when posting notices and bounties.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactPostingProfile {
    /// Default TTL (in ticks) for ThreatWarning notices posted by this agent.
    /// The posted artifact's `expires_at` is set to `current_tick + threat_warning_ttl`.
    pub threat_warning_ttl: u32,
    /// Default TTL for OfficeVacancy notices.
    pub office_vacancy_ttl: u32,
    /// Default TTL for bounty postings.
    pub bounty_ttl: u32,
}
```

**Default impl** (universal component):
```rust
impl Default for ArtifactPostingProfile {
    fn default() -> Self {
        Self {
            threat_warning_ttl: 48,
            office_vacancy_ttl: 96,
            bounty_ttl: 144,
        }
    }
}
```

### D2: Goal dispatch sets `expires_at` from profile

Modify `goal_dispatch_decl.rs` PostNotice and PostBounty dispatch paths to compute `expires_at` from the agent's `ArtifactPostingProfile`:

In `GoalDispatchKey::PostNoticeWarning`:
```rust
GoalDispatchKey::PostNoticeWarning => {
    let ttl = posting_profile.threat_warning_ttl;
    GoalKind::PostNotice {
        posting: ArtifactPostingContext {
            posting_place: destination,
            issuing_authority: None,
            expires_at: Some(Tick(current_tick.0 + ttl)),
            jurisdiction: None,
        },
        topic: NoticeTopic::ThreatWarning { place: destination },
    }
}
```

Similarly for `PostNoticeOther` (using `office_vacancy_ttl`) and `PostBounty` (using `bounty_ttl`).

### D3: Candidate generation sets `expires_at` from profile

All PostNotice candidate generation paths in `candidate_generation.rs` that currently hardcode `expires_at: None` must be updated to compute TTL from the agent's `ArtifactPostingProfile`. The profile is available through the planning snapshot's component access.

Affected locations (grep for `expires_at: None` in candidate_generation.rs):
- Lines 642, 726, 6198, 11034, 11180, 11245

### D4: Goal ranking and feasibility TTL propagation

Update PostNotice-related code in `ranking.rs`, `feasibility.rs`, `route_threat.rs`, `goal_policy.rs`, `exhaustion.rs`, and `plan_revalidation.rs` that constructs `ArtifactPostingContext` with `expires_at: None` to propagate the profile-derived TTL.

### D5: `GoalDispatchContext` and `PlanningSnapshot` access

Ensure `ArtifactPostingProfile` is accessible in:
- `GoalDispatchContext` (for goal dispatch TTL computation)
- `PlanningSnapshot` (for candidate generation TTL computation)

### D6: Golden test — artifact expiry bounds entity count

File: `crates/worldwake-ai/tests/golden_planner_pathology.rs` (or `golden_integration.rs`)

**Setup**: Agent with `notice_posting_weight: 900` at a location with a persistent threat belief. `ArtifactPostingProfile { threat_warning_ttl: 12, .. }`. Run for 100 ticks.

**Assertion**: Agent posts multiple ThreatWarning notices. All posted notices have `expires_at` set. After `threat_warning_ttl` ticks, earlier notices transition to `ArtifactState::Expired`. Total active (non-expired) notice count at the location never exceeds a bounded ceiling (e.g., `ceil(100 / threat_warning_ttl) + 1` at most).

**Emergence justification**: Tests the interaction between goal-driven posting and artifact lifecycle — the posting system creates artifacts, the lifecycle system bounds them, and their interplay determines the artifact population trajectory.

**Why not a duplicate**: Existing artifact lifecycle tests verify the expiry mechanism in isolation. This test verifies that the planner actually provides `expires_at` values and that the end-to-end loop (post → expire → re-post) works.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: TTL is agent-local configuration. The artifact's `expires_at` is a public field on the posted entity — any perceiving agent can observe it. Expiry is processed by `artifact_lifecycle_system` which reads `ArtifactHeader` state.

2. **Positive-feedback analysis**: Existing undamped loop: post artifact → artifact persists → more artifacts → more perception load. This spec introduces expiry as a dampener.

3. **Concrete dampeners**: Artifact TTL (`threat_warning_ttl` etc.) — the posted notice decays over time like a physical posting weathering away. The agent must re-post to maintain the warning, and re-posting has duration/occupancy cost per FND-8 (combined with S96 satiation, re-posting also faces satiation decay).

4. **Stored state vs. derived**: `ArtifactPostingProfile` is authoritative per-agent configuration. `ArtifactHeader.expires_at` is authoritative per-artifact state. The expiry transition is a derived consequence of `current_tick >= expires_at`.

## SystemFn Integration

No new SystemFn. `artifact_lifecycle_system` already handles `expires_at` transitions. The only change is that PostNotice artifacts will now have non-None `expires_at` values.

## Component Registration

- `ArtifactPostingProfile`: Register on `EntityKind::Agent` in `component_schema.rs`. Universal component (every agent gets `Default` if not specified in scenario).

## Cross-System Interactions

- **Goal dispatch / candidate generation** (worldwake-ai): Reads `ArtifactPostingProfile` to compute `expires_at` for `ArtifactPostingContext`. Pure state read.
- **Action handler** (worldwake-systems): Already uses `payload.expires_at` when creating artifacts — no change needed.
- **Artifact lifecycle system** (worldwake-systems): Already transitions artifacts with `expires_at` to `Expired` — no change needed.
- **Perception system** (worldwake-systems): Already skips expired artifacts in observation — no change needed (verify during implementation).

## Profile-Driven Parameters

All TTL values are in `ArtifactPostingProfile`:
- `threat_warning_ttl`: Ticks before ThreatWarning notices expire
- `office_vacancy_ttl`: Ticks before OfficeVacancy notices expire
- `bounty_ttl`: Ticks before bounty postings expire

Scenario authors configure per-agent posting behavior in `AgentDef`. Guard Theron (institutional poster) might have `threat_warning_ttl: 72` (longer warnings). A panicky civilian might have `threat_warning_ttl: 24` (short-lived warnings).
