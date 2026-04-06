**Status**: COMPLETED

# S51: Social Artifact Issuance Goals

## Summary

Add AI goal generation for *creating* social artifacts (bounties, notices), not just consuming them. Currently agents can fulfill bounties and read notices but the planner never generates goals to post them. This spec adds `GoalKind::PostBounty` and `GoalKind::PostNotice` with candidate generation driven by institutional role, economic motivation, and situational awareness.

## Phase

Phase 6: Architectural Substrates II

## Crates

- `worldwake-core` (new GoalKind variants)
- `worldwake-ai` (candidate generation, planner ops, goal dispatch declarations)
- `worldwake-systems` (artifact action enrichment if needed)

## Dependencies

- S45 (unified social artifact model) — completed (`archive/specs/S45-unified-social-artifact-model.md`)
- S36 (declarative goal registration) — completed (`archive/specs/S36-declarative-goal-registration.md`)

## Design Goals

- Enable agents to autonomously post bounties when they have motive (e.g., institution wants someone eliminated, merchant wants cargo delivered)
- Enable agents to autonomously post notices when they hold information worth broadcasting (e.g., wanted notice for crime suspect, danger warning)
- Use existing `post_bounty` and `post_notice` action infrastructure — no new actions needed
- Candidate generation must be belief-driven, not omniscient

## Non-Goals

- Artifact maintenance (updating, revoking, contesting) — deferred
- Artifact copying or reposting — deferred
- Operational assignments (patrol orders, escort duties) — separate spec
- Debt/contract artifact types — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Maximal Emergence) | Bounty posting emerges from agent motivation, not authored triggers |
| P7 (Locality) | Agent must believe the motive conditions locally — no omniscient bounty generation |
| P14 (Belief ≠ Truth) | Posting decisions based on agent beliefs about threats, needs, resources |
| P18 (Records Are World State) | Posted artifacts are inspectable, persistent world entities |
| P20 (Resource-Bounded Practical Reasoning) | Goals name desired conditions ("threat eliminated", "information broadcast") |
| P25 (Social Artifacts) | Bounties and notices are first-class world artifacts with identity |

## Deliverables

### 1. New GoalKind Variants

`GoalKind` derives Copy (`goal.rs:9`). All nested types must be Copy. Both `BountyTarget` and `NoticeTopic` already derive Copy (`social_artifact.rs`).

```rust
GoalKind::PostBounty {
    target: BountyTarget,     // From social_artifact.rs — EliminateEntity or DeliverCommodity
    posting_place: EntityId,  // Where the agent believes it can post
}

GoalKind::PostNotice {
    topic: NoticeTopic,       // From social_artifact.rs — ThreatWarning, OfficeVacancy, etc.
    posting_place: EntityId,  // Where the agent believes it can post
}
```

**Design note**: Posting *motive* (why the agent wants to post) is used during candidate generation for ranking but is NOT stored in GoalKind. GoalKind fields affect `GoalKey` deduplication — two agents posting the same bounty target at the same place should deduplicate, regardless of whether the motive is institutional enforcement or personal vendetta. Motive context lives in the `GroundedGoal` ranking metadata emitted by candidate generation.

### 2. Candidate Generation

New emission functions in `crates/worldwake-ai/src/candidate_generation.rs`:

**`emit_bounty_posting_candidates()`** — driven by:
- Office holder with unresolved accusations and `bounty_posting_weight > 0`: emits `PostBounty { target: EliminateEntity { target: accused }, posting_place }`. Ranked by `bounty_posting_weight × accusation severity`.
- Agent with `enterprise_weight > 0` and unsatisfied delivery needs: emits `PostBounty { target: DeliverCommodity { ... }, posting_place }`. Ranked by `bounty_posting_weight × demand urgency`.
- Agent with high `danger_weight` and known hostile threat: emits `PostBounty { target: EliminateEntity { target: threat }, posting_place }`. Ranked by `bounty_posting_weight × believed danger`.

**`emit_notice_posting_candidates()`** — driven by:
- Office holder with unresolved crime cases and `notice_posting_weight > 0`: emits `PostNotice { topic: Institutional { claim: Accusation { ... } }, posting_place }`. Ranked by `notice_posting_weight × case severity`.
- Agent with recent danger observation and `notice_posting_weight > 0`: emits `PostNotice { topic: ThreatWarning { place }, posting_place }`. Ranked by `notice_posting_weight × believed threat level`.

Posting place is determined from the agent's believed known places — the nearest place where posting is believed to be lawful.

### 3. Goal Dispatch

Register both new variants in `GoalDispatchDeclaration` (`crates/worldwake-ai/src/goal_dispatch_decl.rs`):
- **Relevant ops**: `PlannerOpKind::PostBounty`, `PlannerOpKind::PostNotice` (new ops wrapping existing actions)
- **Feasibility**: agent has coin for bounty reward reserve (for PostBounty), or posting place is known and reachable
- **Invalidation**: target already eliminated (bounty no longer needed), crime already resolved (notice no longer needed), threat gone (warning no longer relevant)
- **Replan on failure**: If `post_bounty` action fails (e.g., insufficient funds, not co-located), `handle_plan_failure` triggers standard replanning — agent may re-attempt after acquiring funds or traveling to posting place

### 4. Planner Integration

New `PlannerOpKind::PostBounty` and `PlannerOpKind::PostNotice` in `crates/worldwake-ai/src/planner_ops.rs`:
- Planner semantics: Travel(posting_place) → PostBounty/PostNotice action
- Hypothetical transition: artifact entity created, reward reserved (for bounty)
- Classification in `classify_action_def()`: map `post_bounty` action def → `PlannerOpKind::PostBounty`, `post_notice` → `PlannerOpKind::PostNotice`
- Both actions already have `with_payload_override_validator` registered (`artifact_actions.rs:39,58`), so planner-synthesized payloads will be revalidated correctly

### 5. UtilityProfile Extension

Add new fields to `UtilityProfile` (`crates/worldwake-core/src/utility_profile.rs`):
```rust
pub bounty_posting_weight: Permille,  // Motivation to post bounties vs handle threats directly
pub notice_posting_weight: Permille,  // Motivation to post notices vs tell individuals
```

Per `docs/spec-drafting-rules.md` section 5 (Agent Profile Scenario Contract):
- UtilityProfile is a universal profile — update `Default` impl with `bounty_posting_weight: Permille::new_unchecked(0)` and `notice_posting_weight: Permille::new_unchecked(0)` (default: agents don't autonomously post unless configured)
- Update `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` — existing `utility_profile` field already covers UtilityProfile; new fields are automatically available in RON since UtilityProfile is directly deserialized
- Update CLI evaluation scenario `scenarios/cli-evaluation.ron` — add non-zero `bounty_posting_weight` / `notice_posting_weight` to at least one agent

### 6. CLI Display

Update `format_goal_kind()` in `crates/worldwake-cli/src/display.rs` to handle `PostBounty` and `PostNotice` variants with human-readable output.

## Cross-System Interactions (Principle 26)

- **Justice system** writes accusation records → read by candidate generation for wanted-notice and enforcement-bounty motivation
- **Perception** updates beliefs about threats/crimes → drives danger-warning notice and threat-elimination bounty motivation
- **Trade system** creates unfulfilled demand → drives economic delivery-bounty motivation
- **Social artifact system** handles the actual posting via existing action handlers — no new actions introduced

All interaction through state. No cross-system direct calls.

## Section H — Causal Hooks

### H.1 Information path
Agent learns about threat/crime/demand through existing perception. Posting decision is belief-driven — agent must believe the motive conditions locally. No omniscient bounty generation. Information path: perception → belief store → candidate generation → goal selection → planner → post action.

### H.2 Positive feedback
Bounty posting → bounty fulfillment → reward payment → reduced resources for future bounties. This is self-dampening through resource consumption. Notice posting → notice reading by others → behavior change → reduced need for notices (threat resolved, vacancy filled).

### H.3 Dampeners
| Loop | Dampener |
|------|----------|
| Bounty posting spiral | Bounty requires real reward reserve (coin must exist). Treasury depletion limits posting rate. |
| Notice spam | Notice posting takes time (action duration). Agent has competing goals. `notice_posting_weight` default 0 limits who posts. |
| Institutional overposting | Office holder vacancy stops institutional posting. Limited accusations to resolve. |

### H.4 Stored vs derived
| Item | Classification |
|------|---------------|
| Posted `SocialArtifact` entities | **Stored authoritative state** |
| Candidate-generation motive context | **Derived** — transient, recomputed per tick |
| `bounty_posting_weight` / `notice_posting_weight` | **Stored** — agent profile parameters |

### H.5 Contention
Two agents can independently post bounties for the same target — this is not contention (both artifacts are valid world entities with separate identity). The contention occurs at *claim* time (S45 race-mode), not at posting time. No new contention mechanism needed.

### H.6 Partial failures
| Failure | Aftermath |
|---------|-----------|
| PostBounty: insufficient funds for reward | Precondition fails. No artifact created. Agent replans — may acquire funds first. |
| PostBounty: not co-located with posting place | Precondition fails. Planner should have included Travel op. If travel failed, standard replan. |
| PostNotice: not co-located with posting place | Same as above. |
| Goal invalidated mid-plan: target eliminated | Goal invalidation triggers. Agent selects new goal. |

### H.7 Belief staleness
Agent may post a bounty for a threat that has already been eliminated (but the agent doesn't know). The bounty is still a valid world entity — another agent may try to fulfill it and discover the target is gone. The bounty then expires or is withdrawn. This is correct behavior: agents act on beliefs, not truth (P14).

### H.8 Temporal resolution
Posting actions use standard action duration (1-2 ticks). Artifact is created at commit time. No special temporal considerations beyond standard action scheduling.

### H.9-H.12 (N/A)
- H.9 (derived views): No new derived views.
- H.10 (not covered above): Agents correct stale posting motivation through standard perception updates.
- H.11 (scheduling): Standard tick resolution. No simultaneity concerns for posting.
- H.12 (boundaries): No boundary/off-map interfaces.

### H.13 Invariants and regression
- Agents with `bounty_posting_weight == 0` must never generate PostBounty candidates
- Agents with `notice_posting_weight == 0` must never generate PostNotice candidates
- Posted artifacts must have real reward sources (conservation)
- Posting must be belief-driven — candidate generation reads beliefs, not world state

### H.14 Save/load
No new persistent state beyond what S45 already handles. GoalKind variants are transient (active goals are replanned on load). UtilityProfile fields are part of the existing save/load path.

## Verification

### Golden test: Institutional bounty posting

**Setup**: One office holder with justice disposition, unresolved accusation, `bounty_posting_weight > 0`, and coin for reward. One posting place.

**Execution**: Tick until office holder posts bounty.

**Assertions**:
- PostBounty goal generated (decision trace)
- `post_bounty` action committed (action trace)
- SocialArtifact entity created with correct BountyTerms (authoritative world state)
- Reward reserved from office holder's funds (conservation)

### Golden test: Danger-warning notice posting

**Setup**: One agent with recent danger observation, `notice_posting_weight > 0`, co-located with posting place.

**Execution**: Tick until agent posts notice.

**Assertions**:
- PostNotice goal generated (decision trace)
- `post_notice` action committed (action trace)
- SocialArtifact entity created with ThreatWarning topic (authoritative world state)

## Outcome

Completed: 2026-04-05

Implemented via `S51ARTISS-001` through `S51ARTISS-005`. The delivered S51 slice now includes the shared posting-goal substrate (`GoalKind::PostBounty` / `GoalKind::PostNotice`, posting utility weights, and lawful posting terms), live planner support, belief-driven candidate generation, selection activation, bounded AI admission-path completion for payload-override posting steps, and autonomous institutional bounty posting proof in `crates/worldwake-ai/tests/golden_integration.rs` as Scenario 112.

The main scope correction during implementation was that S51 could not honestly land all originally sketched posting families at once. The first live autonomous closeout is the institutional accusation-backed bounty path, while the remaining autonomous threat-warning notice golden gap is tracked separately in `specs/S58-golden-gaps-S51.md`. Existing manual and focused notice coverage remains intact.

Verification:
- `cargo test -p worldwake-core`
- `cargo test -p worldwake-ai`
- `cargo test -p worldwake-ai --test golden_integration`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
