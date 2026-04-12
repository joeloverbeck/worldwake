# S96: Obligation Satiation

## Summary

Introduces a profile-driven satiation mechanism for obligation-class goals (PostNotice, PostBounty) so that repeated execution within a time window decays the drive score, preventing infinite obligation spam loops. Includes a golden test proving that obligation satiation allows survival needs to override saturated obligations.

## Phase and Status

Phase 7 adjunct. Status: Draft.

## Crates

- `worldwake-core` — new `ObligationSatiationProfile` component
- `worldwake-ai` — satiation tracking and score dampening in ranking, golden test

## Dependencies

- None. Uses existing goal ranking infrastructure in `ranking.rs`.

## Design Goals

- Dampen the PostNotice positive feedback loop identified in the simulation observer report (Guard Theron: 487 PostNotice executions while starving to death).
- Ensure obligation goals cannot permanently starve survival needs.
- Profile-driven parameters so that agents vary in dutifulness (FND-22).
- No behavioral change for agents with zero `notice_posting_weight` (default is 0).

## Non-Goals

- Changing how `threat_warning_signal_for_place` computes threat intensity — the signal itself is correct.
- Modifying PostNotice action mechanics, duration, or preconditions.
- Perception throttling or artifact TTL (separate specs).

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-1 (Maximal Emergence) | Obligation spam is an engine limitation, not emergence. Satiation restores emergent behavior by letting survival compete. |
| FND-8 (Every Action Has Cost) | Repeated posting now has cumulative attention/fatigue cost expressed as satiation decay. |
| FND-11 (Feedback Dampening) | Direct fix: the undamped obligation loop (post → retrigger → post) now has a concrete dampener (satiation decay per execution within a time window). |
| FND-22 (Agent Diversity) | Satiation parameters are per-agent profile fields. Dutiful agents satiate slowly; impulsive agents satiate quickly. |
| FND-26 (Systems Interact Through State) | Satiation state is a component read by ranking, not a cross-system call. |

## Deliverables

### D1: `ObligationSatiationProfile` component

New component in `worldwake-core`:

```rust
/// Per-agent parameters controlling how obligation-class goals
/// (PostNotice, PostBounty) decay in priority after repeated execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationSatiationProfile {
    /// Number of recent executions within `window_ticks` before satiation
    /// begins decaying the drive score. Below this threshold, no decay.
    pub satiation_threshold: u32,
    /// Time window (in ticks) over which executions are counted.
    /// Executions older than `current_tick - window_ticks` do not count.
    pub window_ticks: u32,
    /// Per-execution decay factor applied to the drive score after
    /// threshold is reached. The effective weight multiplier is:
    /// `max(floor, 1000 - (executions_over_threshold * decay_per_execution))`
    pub decay_per_execution: Permille,
    /// Minimum floor for the satiation multiplier (prevents obligation
    /// goals from reaching zero — even saturated agents still post
    /// occasionally). Expressed as permille of original score.
    pub satiation_floor: Permille,
}
```

**Default impl** (universal component):
```rust
impl Default for ObligationSatiationProfile {
    fn default() -> Self {
        Self {
            satiation_threshold: 2,
            window_ticks: 48,
            decay_per_execution: Permille::new_unchecked(200),
            satiation_floor: Permille::new_unchecked(50),
        }
    }
}
```

With these defaults: after 2 PostNotice executions within 48 ticks, each additional execution reduces the effective score multiplier by 200 permille, bottoming at 50 permille (5% of original). An agent with score 808200 would see it drop to ~40410 after 7 executions within the window — well below typical survival need scores (~200000-500000 at critical levels).

### D2: `ObligationExecutionTracker` runtime component

New runtime-generated component in `worldwake-core`:

```rust
/// Tracks recent obligation-class action completions for satiation.
/// Runtime-generated state — exempt from scenario contract (like ActiveGoal).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObligationExecutionTracker {
    /// Ticks at which obligation actions (PostNotice, PostBounty) completed.
    /// Maintained as a bounded ring: entries older than the agent's
    /// `window_ticks` are pruned each tick.
    pub completion_ticks: Vec<Tick>,
}
```

### D3: Satiation-dampened scoring in `post_notice_motive`

Modify `ranking.rs::post_notice_motive` to apply satiation decay:

```rust
fn post_notice_motive(
    context: &RankingContext<'_>,
    posting: ArtifactPostingContext,
    topic: NoticeTopic,
) -> u32 {
    // ... existing checks (topic, authority, place, thresholds) ...
    let raw_score = score_product(context.utility.notice_posting_weight, threat_signal);
    apply_obligation_satiation(context, raw_score)
}
```

New helper:
```rust
fn apply_obligation_satiation(context: &RankingContext<'_>, raw_score: u32) -> u32 {
    let profile = context.satiation_profile; // &ObligationSatiationProfile
    let tracker = context.obligation_tracker;  // &ObligationExecutionTracker
    let current_tick = context.current_tick;
    let window_start = current_tick.0.saturating_sub(profile.window_ticks);
    let recent_count = tracker.completion_ticks.iter()
        .filter(|t| t.0 >= window_start)
        .count() as u32;
    if recent_count <= profile.satiation_threshold {
        return raw_score;
    }
    let over_threshold = recent_count - profile.satiation_threshold;
    let decay_total = over_threshold * u32::from(profile.decay_per_execution.value());
    let multiplier = 1000u32.saturating_sub(decay_total)
        .max(u32::from(profile.satiation_floor.value()));
    raw_score * multiplier / 1000
}
```

The same `apply_obligation_satiation` is also applied to `post_bounty_motive`.

### D4: Tracker update on obligation action commit

When a PostNotice or PostBounty action commits successfully, append the current tick to the agent's `ObligationExecutionTracker.completion_ticks`. This happens in the existing action commit path. The tracker is pruned of stale entries (older than `window_ticks`) during goal ranking to keep the Vec bounded.

### D5: `RankingContext` extension

Add `satiation_profile: &ObligationSatiationProfile` and `obligation_tracker: &ObligationExecutionTracker` to `RankingContext`. These are populated from the agent's components when constructing the ranking context.

### D6: Golden test — obligation does not starve survival needs

File: `crates/worldwake-ai/tests/golden_planner_pathology.rs`

**Setup**: One guard agent at a location with food and water available. Agent has an active hostile entity belief triggering ThreatWarning. Agent has `notice_posting_weight: 900` and `ObligationSatiationProfile::default()`. Set hunger and thirst to critical levels (>750 permille). Run for 200 ticks.

**Assertion**: Agent performs at least one `eat` and one `drink` action despite active PostNotice obligations. PostNotice executions must not exceed 80% of total actions. Agent must not die from NeedDeprivation.

**GoalKinds exercised**: `PostNotice(ThreatWarning)`, `AcquireCommodity(food)`, `AcquireCommodity(water)`.

**Emergence justification**: Tests the interaction between obligation satiation and survival need ranking — neither system alone produces this behavior; their interplay determines whether the agent lives or dies.

**Why not a duplicate**: `golden_integration.rs` tests PostNotice selection and commitment but does not test the interaction with competing survival needs under satiation. `golden_ai_decisions.rs::golden_fallback_to_addressable_need` tests fallback when top need is unsatisfiable, but not when a non-survival goal outranks survival.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Satiation state is agent-local. The agent's `ObligationExecutionTracker` records its own action completions. No information travels between agents for this mechanism.

2. **Positive-feedback analysis**: The existing undamped loop is: threat belief → PostNotice goal (high score) → execute PostNotice → threat belief persists → repeat. This spec introduces a dampener.

3. **Concrete dampeners**: `ObligationSatiationProfile` with `decay_per_execution` and `satiation_floor`. The dampener is attention fatigue — an agent who has repeatedly posted about a threat becomes habituated and turns attention to other needs. This is not a numeric clamp; it is a concrete behavioral process (habituation/satiation) with profile-driven parameters.

4. **Stored state vs. derived**: `ObligationExecutionTracker.completion_ticks` is authoritative stored state (records of when the agent acted). The satiation multiplier is a derived computation over this stored state, never stored itself.

## SystemFn Integration

No new SystemFn. The satiation logic runs inline within `rank_goals` (already a per-agent computation within the agent tick). `ObligationExecutionTracker` pruning happens during ranking context construction.

## Component Registration

- `ObligationSatiationProfile`: Register on `EntityKind::Agent` in `component_schema.rs`. Universal component (every agent gets `Default` if not specified in scenario).
- `ObligationExecutionTracker`: Register on `EntityKind::Agent`. Runtime-generated (exempt from scenario contract per spec-drafting-rules.md section 5).

## Cross-System Interactions

- **Action commit system** (worldwake-sim): Writes completion tick to `ObligationExecutionTracker` when PostNotice/PostBounty commits. State-mediated, not a direct call to ranking.
- **Goal ranking** (worldwake-ai): Reads `ObligationSatiationProfile` and `ObligationExecutionTracker` to compute dampened scores. Pure state read.

## Profile-Driven Parameters

All satiation parameters are in `ObligationSatiationProfile`:
- `satiation_threshold`: How many executions before decay begins
- `window_ticks`: Time window for counting recent executions
- `decay_per_execution`: Decay rate per execution over threshold
- `satiation_floor`: Minimum score multiplier (prevents total suppression)

Scenario authors configure per-agent satiation behavior in `AgentDef`. Guard Theron (highly dutiful) might have `satiation_threshold: 3, decay_per_execution: 150` (slow decay). A panicky civilian might have `satiation_threshold: 1, decay_per_execution: 300` (fast decay).
