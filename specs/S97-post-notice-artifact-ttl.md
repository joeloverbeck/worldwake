# S97: PostNotice Artifact TTL Provisioning

## Summary

Makes the planner and goal dispatch provide profile-driven `expires_at` values for PostNotice (and PostBounty) artifacts, so that posted notices expire via the existing `artifact_lifecycle_system` instead of persisting indefinitely. Addresses the SocialArtifact pollution identified in the simulation observer report (500+ never-expiring artifacts at Dusty Trail).

## Phase and Status

Phase 7 adjunct. Status: Draft.

## Crates

- `worldwake-core` — new `ArtifactPostingProfile` component with TTL defaults
- `worldwake-sim` — `GoalBeliefView` accessor for the new profile
- `worldwake-ai` — candidate generation sets `expires_at` from profile
- `worldwake-cli` — `AgentDef` field and `spawn_agent()` registration

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
    pub threat_warning_ttl: u64,
    /// Default TTL for OfficeVacancy notices.
    pub office_vacancy_ttl: u64,
    /// Default TTL for bounty postings.
    pub bounty_ttl: u64,
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

Register on `EntityKind::Agent` in `component_schema.rs`. Universal component (every agent gets `Default` if not specified in scenario).

TTL fields are `u64` to match `Tick` arithmetic (`Tick` wraps `u64` and implements `Add<u64>`).

### D2: `GoalBeliefView` accessor

Add accessor to `GoalBeliefView` trait in `crates/worldwake-sim/src/belief_view.rs`:

```rust
fn artifact_posting_profile(&self, agent: EntityId) -> Option<ArtifactPostingProfile> {
    None
}
```

Implement in `RuntimeBeliefView` to read the component from the snapshot. Forward through the `impl_goal_belief_view!` macro.

This follows the established pattern for profile components read by the AI crate (same as `drive_thresholds`, `cognitive_profile`, `utility_profile`).

### D3: Runtime candidate generation sets `expires_at` from profile

Two runtime locations in `crates/worldwake-ai/src/candidate_generation.rs` construct `ArtifactPostingContext` with `expires_at: None`:

- **Line 642**: `PostBounty` posting in `emit_bounty_posting_candidates` — use `bounty_ttl`
- **Line 726**: `PostNotice` posting in `emit_notice_posting_candidates` — use `threat_warning_ttl`

Both functions receive `ctx: &GenerationContext<'_>` which has `view: &dyn GoalBeliefView`. Access the profile via `ctx.view.artifact_posting_profile(ctx.agent)` and compute:

```rust
let posting_profile = ctx.view.artifact_posting_profile(ctx.agent);
let expires_at = posting_profile.map(|p| ctx.current_tick + p.threat_warning_ttl);
```

### D4: CLI scenario support

Add `ArtifactPostingProfile` to the scenario system per the profile completeness invariant:

1. Add `artifact_posting_profile: Option<ArtifactPostingProfile>` field to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`. No `*Def` wrapper needed (no `EntityId` references in the profile).
2. Add `set_component` call in `spawn_agent()` in `crates/worldwake-cli/src/scenario/mod.rs`. Universal component pattern: `unwrap_or_default()`, always applied.

### D5: Test fixture updates

Multiple test files construct `ArtifactPostingContext` or `BelievedArtifactState` with `expires_at: None`. After D1-D3 are implemented, test fixtures that construct `ArtifactPostingContext` for PostNotice/PostBounty goals should use profile-derived TTL values for consistency.

**`ArtifactPostingContext` locations (test code — need TTL values):**
- `candidate_generation.rs`: lines 11034, 11180, 11245 (past `#[cfg(test)]` at line 4899)
- `goal_dispatch_decl.rs`: lines 803, 819, 828 (in `representative_goal_for`, `#[cfg(test)]` at line 664)
- `ranking.rs`: lines 2712, 2809, 2894, 2939, 2994, 3189, 3233, 3265 (`#[cfg(test)]` at line 1715)
- `feasibility.rs`: lines 963, 981 (`#[cfg(test)]` at line 267)
- `goal_policy.rs`: line 704 (`#[cfg(test)]` at line 121)

**`BelievedArtifactState` locations (no code changes needed):**
These represent what agents *observe* about existing artifacts. Once artifacts are posted with `expires_at` values (via D3), the belief/perception system will naturally propagate non-None values. No changes needed in:
- `candidate_generation.rs`: line 6198
- `ranking.rs`: line 2376
- `route_threat.rs`: line 295
- `exhaustion.rs`: lines 1058, 1104, 1157, 1207
- `goal_model.rs`: lines 9595, 9662, 9904, 9981, 10064
- `goal_dispatch_key.rs`: 1 occurrence in tests
- `search/tests.rs`: 5 occurrences in tests
- `plan_revalidation.rs`: line 1140 (`PostBountyActionPayload`)

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

- **Candidate generation** (worldwake-ai): Reads `ArtifactPostingProfile` via `GoalBeliefView` accessor to compute `expires_at` for `ArtifactPostingContext`. Pure state read.
- **Action handler** (worldwake-systems): Already uses `payload.expires_at` when creating artifacts — no change needed.
- **Artifact lifecycle system** (worldwake-systems): Already transitions artifacts with `expires_at` to `Expired` — no change needed.
- **Perception system** (worldwake-systems): Already skips expired artifacts in observation — no change needed (verify during implementation).

## Profile-Driven Parameters

All TTL values are in `ArtifactPostingProfile`:
- `threat_warning_ttl`: Ticks before ThreatWarning notices expire
- `office_vacancy_ttl`: Ticks before OfficeVacancy notices expire
- `bounty_ttl`: Ticks before bounty postings expire

Scenario authors configure per-agent posting behavior in `AgentDef`. Guard Theron (institutional poster) might have `threat_warning_ttl: 72` (longer warnings). A panicky civilian might have `threat_warning_ttl: 24` (short-lived warnings).
