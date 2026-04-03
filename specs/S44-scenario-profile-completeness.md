# S44: Scenario Profile Completeness

## Summary

16 agent profile components are registered in the ECS but not available in the CLI scenario system (`AgentDef` / `spawn_agent()`). Agents spawned from RON scenarios silently lack profiles that control perception, social transmission, reasoning, belief confidence, and other core behaviors. This means scenario-driven evaluation and testing cannot exercise significant portions of the simulation, and users cannot configure agent diversity for these dimensions.

This spec audits all agent-registered components, classifies them as universal or role-specific, makes them scenario-definable, enforces runtime presence for universal profiles, and documents the pattern to prevent recurrence.

## Source

Discovered during CLI evaluation scenario maintenance (2026-04-03). Cross-referencing `component_schema.rs` agent registrations against `AgentDef` and `spawn_agent()` revealed 16 profile-type components with no scenario path.

## Phase

Phase 5: Architectural Substrates

## Crates

- `worldwake-cli` (scenario types, spawn_agent, scenario validation)
- `worldwake-core` (no changes — profiles already registered)

## Dependencies

- None. Independent of S42, S43. Should be implemented before any further spec that adds agent profiles.

## FOUNDATIONS Alignment

- **Principle 19, Agent Symmetry**: Every agent uses the same rules. Universal profiles define what it means to be a functioning agent — absence is a creation bug, not a valid state.
- **Principle 22, Agent Diversity Through Concrete Variation**: Per-agent profile values in RON enable diversity. Without scenario-definable profiles, all agents get identical (default) behavior for 16 dimensions.
- **Principle 29, Debuggability**: Silent fallbacks when a profile is absent hide bugs. An agent that can't perceive because PerceptionProfile is missing produces confusing behavior that's hard to trace.
- **Principle 31, Validation and Falsification**: If the scenario system can't configure a profile, evaluations can't test it, and regressions in that profile's behavior go undetected.

## Design Goals

1. **Every agent profile registered on `EntityKind::Agent` must be reachable through the scenario system** — either always-applied with a default (universal) or optionally specified in RON (role-specific).
2. **Universal profiles are never absent on a live agent** — `spawn_agent()` guarantees their presence, and runtime access uses `expect()` not silent fallback.
3. **RON can override any profile's defaults** — enabling agent diversity (Principle 22) for both universal and role-specific profiles.
4. **The pattern is documented** so future specs that add profiles cannot repeat this gap.

## Non-Goals

- Changing golden test harness agent creation (they intentionally construct minimal agents for focused testing).
- Changing profile definitions or field types — this spec only makes existing profiles scenario-definable.
- Adding new profiles — this spec addresses the backlog of missing ones.

## Deliverables

### 1. Component Classification

Classify all agent-registered profile-type components. Components that are purely runtime-generated state (e.g., `ActiveGoal`, `IntentionFrame`, `InTransitOnEdge`) are excluded — they emerge from simulation, not configuration.

**Universal profiles** — every agent must have these to function as a reasoning, perceiving, socially-participating agent:

| Component | Current scenario status | Default when absent | Risk of absence |
|-----------|----------------------|--------------------|--------------------|
| `PerceptionProfile` | Missing | Agent cannot perceive | Blind agent, silent failure in golden tests |
| `TellProfile` | Missing | Agent cannot participate in social transmission | Socially isolated, no Tell candidates |
| `ReasoningProfile` | Missing | Falls back to global defaults | Per-agent reasoning diversity lost (S42) |
| `EpistemicDispositionProfile` | Missing | Hardcoded defaults in belief confidence | Belief confidence not agent-specific |
| `IntentionDispositionProfile` | Missing | Hardcoded defaults for commitment | Commitment behavior not agent-specific |
| `CommunicationProfile` | Missing | Falls back to defaults in Tell handler | Per-class acceptance diversity lost (S43) |
| `PreferenceProfile` | Missing | No consumption preferences | Agent can't prefer specific commodities |

**Role-specific profiles** — only relevant for agents in specific roles:

| Component | Role | Current scenario status |
|-----------|------|----------------------|
| `CombatProfile` | Combatants | In AgentDef ✓ |
| `MerchandiseProfile` | Merchants | In AgentDef ✓ |
| `TradeDispositionProfile` | Traders | In AgentDef ✓ |
| `TheftDispositionProfile` | Thieves | Missing |
| `JusticeDispositionProfile` | Law enforcers | Missing |
| `ViolationDispositionProfile` | Violation responders | Missing |
| `PatrolProfile` | Guards | Missing |
| `PatrolRoute` | Guards | Missing |
| `PursuitProfile` | Pursuers/guards | Missing |
| `FacilityQueueDispositionProfile` | Facility users | Missing |
| `CommodityValuationProfile` | Traders/merchants | Missing |
| `SubstitutePreferences` | Consumers | Missing |

**Runtime-only state** — excluded from scenario definition (emerge from simulation):

| Component | Reason |
|-----------|--------|
| `AgentData` | Set by `create_agent()` |
| `HomeostaticNeeds` | Already in AgentDef ✓ |
| `DeprivationExposure` | Always defaulted by `spawn_agent()` ✓ |
| `DriveThresholds` | Always defaulted by `spawn_agent()` ✓ |
| `MetabolismProfile` | Always defaulted by `spawn_agent()` ✓ |
| `CarryCapacity` | Always defaulted by `spawn_agent()` ✓ |
| `UtilityProfile` | Already in AgentDef ✓ |
| `WoundList` | Created when wounded |
| `DeadAt` | Created on death |
| `CombatStance` | Created when entering combat |
| `RouteExperience` | Created from travel |
| `SourceReliability` | Created from observation |
| `AgentBeliefStore` | Created by AI runtime |
| `BlockedIntentMemory` | Created by planning |
| `KnownRecipes` | Created by production |
| `DemandMemory` | Created by trade |
| `ActiveGoal` | Created by decision runtime |
| `IntentionFrame` | Created by decision runtime |
| `FacilityQueueIntents` | Created by queue system |
| `InTransitOnEdge` | Created during travel |
| `ViolationMemory` | Created by violation system |

### 2. AgentDef extensions (`worldwake-cli/src/scenario/types.rs`)

Add fields to `AgentDef` for all missing profiles:

**Universal profiles** (always applied — use `#[serde(default)]` so RON can omit them):

```rust
#[serde(default)]
pub perception_profile: Option<PerceptionProfile>,
#[serde(default)]
pub tell_profile: Option<TellProfile>,
#[serde(default)]
pub reasoning_profile: Option<ReasoningProfile>,
#[serde(default)]
pub epistemic_disposition: Option<EpistemicDispositionProfile>,
#[serde(default)]
pub intention_disposition: Option<IntentionDispositionProfile>,
#[serde(default)]
pub communication_profile: Option<CommunicationProfile>,
#[serde(default)]
pub preference_profile: Option<PreferenceProfile>,
```

**Role-specific profiles** (optional — only applied if present in RON):

```rust
#[serde(default)]
pub theft_disposition: Option<TheftDispositionProfile>,
#[serde(default)]
pub justice_disposition: Option<JusticeDispositionProfile>,
#[serde(default)]
pub violation_disposition: Option<ViolationDispositionProfile>,
#[serde(default)]
pub patrol_profile: Option<PatrolProfile>,
#[serde(default)]
pub patrol_route: Option<PatrolRouteDef>,
#[serde(default)]
pub pursuit_profile: Option<PursuitProfile>,
#[serde(default)]
pub facility_queue_disposition: Option<FacilityQueueDispositionProfile>,
#[serde(default)]
pub commodity_valuation: Option<CommodityValuationProfile>,
#[serde(default)]
pub substitute_preferences: Option<SubstitutePreferences>,
```

Note: `PatrolRoute` contains `EntityId` references to places, so it needs a `PatrolRouteDef` with string names (like `MerchandiseProfileDef`), resolved during spawning.

### 3. spawn_agent() updates (`worldwake-cli/src/scenario/mod.rs`)

**Universal profiles** — always applied, RON value takes precedence over default:

```rust
// Universal profiles — every agent gets these
let perception = agent_def.perception_profile.unwrap_or_default();
txn.set_component_perception_profile(agent_id, perception)?;

let tell = agent_def.tell_profile.unwrap_or_default();
txn.set_component_tell_profile(agent_id, tell)?;

let reasoning = agent_def.reasoning_profile.unwrap_or_default();
txn.set_component_reasoning_profile(agent_id, reasoning)?;

let epistemic = agent_def.epistemic_disposition.unwrap_or_default();
txn.set_component_epistemic_disposition_profile(agent_id, epistemic)?;

let intention = agent_def.intention_disposition.unwrap_or_default();
txn.set_component_intention_disposition_profile(agent_id, intention)?;

let communication = agent_def.communication_profile.unwrap_or_default();
txn.set_component_communication_profile(agent_id, communication)?;

let preference = agent_def.preference_profile.unwrap_or_default();
txn.set_component_preference_profile(agent_id, preference)?;
```

**Role-specific profiles** — conditional, same pattern as existing CombatProfile:

```rust
if let Some(ref profile) = agent_def.theft_disposition {
    txn.set_component_theft_disposition_profile(agent_id, profile.clone())?;
}
// ... same for justice, violation, patrol, pursuit, facility_queue, commodity_valuation, substitute_preferences
```

### 4. Runtime enforcement for universal profiles

For each universal profile, audit the primary access sites and replace silent fallbacks with `expect()`:

- `PerceptionProfile`: accessed in perception systems, Tell handler, candidate generation. Replace `if let Some(profile) = ...` with `.expect("agent must have PerceptionProfile")` where the caller already verified the entity is an agent.
- `TellProfile`: accessed in `emit_social_candidates()` and Tell handler. Replace `if let Some(profile) = ctx.view.tell_profile(...)` patterns.
- `ReasoningProfile`: accessed in planning. Replace fallbacks to global defaults.
- Others: audit and convert similarly.

**Important**: Only convert access sites where the entity is known to be an agent. Some code paths query profiles on arbitrary entities (which may be places, items, etc.) — those must remain `Option`-based.

### 5. Documentation updates

**`docs/spec-drafting-rules.md`** — add a new section:

```markdown
## N. Agent Profile Scenario Contract

Every spec that adds a new ECS component registered on `EntityKind::Agent` that
affects agent behavior must:

1. Classify the component as **universal** (every agent needs it) or
   **role-specific** (only relevant agents).
2. Add the component to `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`.
3. Add the `set_component_*` call in `spawn_agent()` in `scenario/mod.rs`:
   - Universal: `unwrap_or_default()` — always applied.
   - Role-specific: conditional `if let Some(...)` — applied only if in RON.
4. Universal profiles must have a `Default` impl.
5. Runtime access to universal profiles on known agents uses `expect()`, not
   silent fallback.

Components that are purely runtime-generated state (ActiveGoal, IntentionFrame,
WoundList, etc.) are exempt — they emerge from simulation, not configuration.
```

**`CLAUDE.md`** — add to the "Critical Invariants" section:

```markdown
- **Scenario profile completeness** — every agent profile component registered on
  `EntityKind::Agent` must be scenario-definable via `AgentDef` + `spawn_agent()`.
  Universal profiles are always applied (with defaults). See `docs/spec-drafting-rules.md`
  for the checklist.
```

**General principle** (also in `docs/spec-drafting-rules.md` or `CLAUDE.md`):

```markdown
Any new ECS component that affects agent behavior must be exercisable through the
scenario system. If a component changes what an agent can do, perceive, decide, or
communicate, a scenario author must be able to configure it. Silent absence of
behavioral components is a bug, not a feature.
```

### 6. Update CLI evaluation scenario

After all profiles are scenario-definable, update `scenarios/cli-evaluation.ron` to exercise agent diversity:

- Give agents varied `PerceptionProfile` fidelity values.
- Give Merchant Vara a `CommunicationProfile` with non-default gossip acceptance.
- Give Guard Theron a `PatrolProfile` and `PursuitProfile`.
- Give at least one agent a `TheftDispositionProfile` to enable theft-related affordances.

This is the scenario skill's job after this spec is implemented.

## Section H — FOUNDATIONS Analysis

### H.1 Information-path analysis

No new information paths. This spec makes existing profiles configurable through the scenario system — it does not change how information flows.

### H.2 Positive-feedback analysis

No new feedback loops. Profile configuration is static (set at spawn time, not mutated by systems).

### H.3 Concrete dampeners

N/A — no amplifying loops introduced.

### H.4 Stored state vs. derived read-model list

| Item | Classification |
|------|---------------|
| Profile components on agents | **Stored authoritative state** — set at spawn, persisted in save/load |
| `AgentDef` fields | **Scenario definition** — read at load time, not persisted in save |
| Runtime enforcement (`expect`) | **Code invariant** — not state |

## Cross-System Interactions (Principle 12)

No cross-system changes. All modifications are in `worldwake-cli` (scenario loading). Profile components are read by systems in `worldwake-ai` and `worldwake-systems` as before — this spec just ensures they exist on scenario-spawned agents.

## Migration Path

1. Classify all agent-registered components (Deliverable 1).
2. Add missing fields to `AgentDef` in `types.rs` (Deliverable 2). Add `PatrolRouteDef` for place-reference resolution.
3. Update `spawn_agent()` to apply universal profiles unconditionally and role-specific profiles conditionally (Deliverable 3).
4. Audit and convert universal profile access sites from silent fallback to `expect()` (Deliverable 4).
5. Update `docs/spec-drafting-rules.md` and `CLAUDE.md` (Deliverable 5).
6. Update `scenarios/cli-evaluation.ron` with diverse profiles (Deliverable 6 — scenario skill's job, done separately).
7. Update existing `types.rs` tests to cover new fields.

## Verification

- `cargo test -p worldwake-cli` — scenario deserialization tests pass with new fields.
- `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit` — scenario loads.
- `cargo test --workspace` — no regressions from `expect()` conversions (all golden tests create agents with universal profiles, or if they don't, those tests must be updated).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- Inspect `spawn_agent()` — every universal profile has an unconditional `set_component_*` call.
- Inspect `AgentDef` — every missing profile from Deliverable 1 has a field.
- `docs/spec-drafting-rules.md` contains the profile completeness checklist.
- `CLAUDE.md` contains the scenario profile completeness invariant.
