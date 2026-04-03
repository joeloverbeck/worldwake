# S44: Scenario Profile Completeness

## Summary

16 agent profile components are registered in the ECS but not available in the CLI scenario system (`AgentDef` / `spawn_agent()`). Additionally, 3 already-defaulted profiles (`DriveThresholds`, `MetabolismProfile`, `CarryCapacity`) are applied uniformly with no scenario override path, undermining agent diversity. Agents spawned from RON scenarios silently lack profiles that control perception, social transmission, reasoning, belief confidence, and other core behaviors — and share identical urgency thresholds and depletion rates.

This spec audits all agent-registered components, classifies them as universal or role-specific, makes them scenario-definable, adds missing `Default` impls, enforces runtime presence for universal profiles, and documents the pattern to prevent recurrence.

## Source

Discovered during CLI evaluation scenario maintenance (2026-04-03). Cross-referencing `component_schema.rs` agent registrations against `AgentDef` and `spawn_agent()` revealed 16 profile-type components with no scenario path, plus 3 already-applied profiles with no override path.

## Phase

Phase 5: Architectural Substrates

## Crates

- `worldwake-core` (add `Default` impls for `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `PreferenceProfile`)
- `worldwake-cli` (scenario types, spawn_agent, scenario validation)
- `worldwake-ai` (runtime enforcement — convert universal profile access sites from silent fallback to `expect()`)
- `worldwake-systems` (runtime enforcement — convert universal profile access sites from silent fallback to `expect()`)

## Dependencies

- None. Independent of S42, S43. Should be implemented before any further spec that adds agent profiles.

## FOUNDATIONS Alignment

- **Principle 19, Agent Symmetry**: Every agent uses the same rules. Universal profiles define what it means to be a functioning agent — absence is a creation bug, not a valid state.
- **Principle 22, Agent Diversity Through Concrete Variation**: Per-agent profile values in RON enable diversity. Without scenario-definable profiles, all agents get identical (default) behavior for 19 dimensions. DriveThresholds (28 values controlling urgency) and MetabolismProfile (16 fields controlling depletion rates) are especially impactful — "homogeneous populations collapse into herd behavior" (P22).
- **Principle 29, Debuggability**: Silent fallbacks when a profile is absent hide bugs. An agent that can't perceive because PerceptionProfile is missing produces confusing behavior that's hard to trace.
- **Principle 31, Validation and Falsification**: If the scenario system can't configure a profile, evaluations can't test it, and regressions in that profile's behavior go undetected.

## Design Goals

1. **Every agent profile registered on `EntityKind::Agent` must be reachable through the scenario system** — either always-applied with a default (universal) or optionally specified in RON (role-specific).
2. **Universal profiles are never absent on a live agent** — `spawn_agent()` guarantees their presence, and runtime access uses `expect()` not silent fallback.
3. **RON can override any profile's defaults** — enabling agent diversity (Principle 22) for both universal and role-specific profiles.
4. **Already-defaulted profiles become overridable** — DriveThresholds, MetabolismProfile, and CarryCapacity get `Option` fields in AgentDef so scenario authors can vary them per agent.
5. **The pattern is documented** so future specs that add profiles cannot repeat this gap.

## Non-Goals

- Changing golden test harness agent creation (they intentionally construct minimal agents for focused testing).
- Changing profile definitions or field types — this spec only makes existing profiles scenario-definable (except adding missing `Default` impls).
- Adding new profiles — this spec addresses the backlog of missing ones.

## Deliverables

### 1. Component Classification

Classify all agent-registered profile-type components. Components that are purely runtime-generated state (e.g., `ActiveGoal`, `IntentionFrame`, `InTransitOnEdge`) are excluded — they emerge from simulation, not configuration.

**Universal profiles** — every agent must have these to function as a reasoning, perceiving, socially-participating agent:

| Component | Current scenario status | Default impl exists | Risk of absence |
|-----------|----------------------|--------------------|--------------------|
| `PerceptionProfile` | Missing | Yes | Blind agent, silent failure in golden tests |
| `TellProfile` | Missing | Yes | Socially isolated, no Tell candidates. Fields (post-S43): `max_tell_candidates`, `max_relay_chain_len`, `conversation_memory_capacity`, `conversation_memory_retention_ticks`. |
| `ReasoningProfile` | Missing | Yes | Per-agent reasoning diversity lost (S42) |
| `EpistemicDispositionProfile` | Missing | **No — must add** | Belief confidence not agent-specific |
| `IntentionDispositionProfile` | Missing | **No — must add** | Commitment behavior not agent-specific |
| `CommunicationProfile` | Missing | Yes | Per-class acceptance diversity lost (S43) |
| `PreferenceProfile` | Missing | **No — must add** | Agent can't prefer specific commodities |

**Already-defaulted profiles** — currently always applied with no scenario override:

| Component | Current scenario status | Diversity impact |
|-----------|----------------------|-----------------|
| `DriveThresholds` | Defaulted, not overridable | **Critical** — 28 threshold values control AI urgency (timid vs brave agents). All agents share identical urgency thresholds. |
| `MetabolismProfile` | Defaulted, not overridable | **High** — 16 fields control depletion rates, tolerance, recovery. All agents have identical metabolism. |
| `CarryCapacity` | Defaulted, not overridable | Medium — all agents carry exactly 20 LoadUnits. |

**Role-specific profiles** — only relevant for agents in specific roles:

| Component | Role | Current scenario status | Needs Def wrapper |
|-----------|------|----------------------|-------------------|
| `CombatProfile` | Combatants | In AgentDef ✓ | No |
| `MerchandiseProfile` | Merchants | In AgentDef ✓ | Yes (existing MerchandiseProfileDef) |
| `TradeDispositionProfile` | Traders | In AgentDef ✓ | No |
| `TheftDispositionProfile` | Thieves | Missing | No |
| `JusticeDispositionProfile` | Law enforcers | Missing | No |
| `ViolationDispositionProfile` | Violation responders | Missing | No |
| `PatrolProfile` | Guards | Missing | No |
| `PatrolRoute` | Guards | Missing | **Yes** — contains `Vec<EntityId>` for places |
| `PursuitProfile` | Pursuers/guards | Missing | No |
| `FacilityQueueDispositionProfile` | Facility users | Missing | No |
| `CommodityValuationProfile` | Traders/merchants | Missing | No |
| `SubstitutePreferences` | Consumers | Missing | No — `BTreeMap<TradeCategory, Vec<CommodityKind>>` is directly serializable |

Note on role-specific profiles: none have `Default` impls, and none need them — they contain `NonZeroU32` fields that can't be zero-initialized. Role-specific profiles remain `Option`-based at all access sites (`if let Some(...)` pattern). Only universal profiles get runtime enforcement via `expect()`.

**Runtime-only state** — excluded from scenario definition (emerge from simulation):

| Component | Reason |
|-----------|--------|
| `AgentData` | Set by `create_agent()` |
| `HomeostaticNeeds` | Already in AgentDef ✓ |
| `UtilityProfile` | Already in AgentDef ✓ |
| `DeprivationExposure` | Always defaulted, no diversity value (tracks accumulated deprivation) |
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

### 2. Add missing Default impls (`worldwake-core`)

Three universal profiles lack `Default` impls. Add them so `spawn_agent()` can use `unwrap_or_default()`:

**`EpistemicDispositionProfile`** (`crates/worldwake-core/src/epistemic.rs`):
- `stale_evidence_barrier_threshold`: reasonable default (e.g., `Permille(500)`)
- `witness_query_duration_ticks`: reasonable default (e.g., `NonZeroU32::new(3).unwrap()`)
- `ask_memory_retention_ticks`: reasonable default (e.g., `50`)

**`IntentionDispositionProfile`** (`crates/worldwake-core/src/intention_disposition.rs`):
- `domain_patience`: empty `BTreeMap` (use default patience for all domains)
- `default_patience_ticks`: reasonable default (e.g., `NonZeroU32::new(20).unwrap()`)
- `commitment_switch_margin`: reasonable default (e.g., `Permille(200)`)

**`PreferenceProfile`** (`crates/worldwake-core/src/experience.rs`):
- `route_caution_weight`: reasonable default (e.g., `Permille(500)`)
- `source_trust_weight`: reasonable default (e.g., `Permille(500)`)
- `route_memory_capacity`: reasonable default (e.g., `20`)
- `source_memory_capacity`: reasonable default (e.g., `20`)
- `memory_retention_ticks`: reasonable default (e.g., `200`)

The exact values should be calibrated during implementation by checking existing golden test setups for these profiles to determine the baseline values that existing behavior assumes.

### 3. AgentDef extensions (`worldwake-cli/src/scenario/types.rs`)

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

**Already-defaulted profiles** (now overridable — `#[serde(default)]` optional fields):

```rust
#[serde(default)]
pub drive_thresholds: Option<DriveThresholds>,
#[serde(default)]
pub metabolism_profile: Option<MetabolismProfile>,
#[serde(default)]
pub carry_capacity: Option<CarryCapacity>,
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

**`PatrolRouteDef`** — new scenario Def type (like `MerchandiseProfileDef`):

```rust
#[derive(Clone, Debug, Deserialize)]
pub struct PatrolRouteDef {
    pub assigned_places: Vec<String>,
}
```

During spawning, resolve place names to `EntityId`s and construct `PatrolRoute { assigned_places: resolved_ids, current_index: 0 }`.

### 4. spawn_agent() updates (`worldwake-cli/src/scenario/mod.rs`)

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

**Already-defaulted profiles** — change existing unconditional default to override-or-default:

```rust
// Already-defaulted profiles — now overridable
let thresholds = agent_def.drive_thresholds.unwrap_or_default();
txn.set_component_drive_thresholds(agent_id, thresholds)?;

let metabolism = agent_def.metabolism_profile.unwrap_or_default();
txn.set_component_metabolism_profile(agent_id, metabolism)?;

let carry = agent_def.carry_capacity.unwrap_or(DEFAULT_AGENT_CARRY_CAPACITY);
txn.set_component_carry_capacity(agent_id, carry)?;
```

**Role-specific profiles** — conditional, same pattern as existing CombatProfile:

```rust
if let Some(ref profile) = agent_def.theft_disposition {
    txn.set_component_theft_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.justice_disposition {
    txn.set_component_justice_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.violation_disposition {
    txn.set_component_violation_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.patrol_profile {
    txn.set_component_patrol_profile(agent_id, profile.clone())?;
}
if let Some(ref route_def) = agent_def.patrol_route {
    let assigned_places = route_def.assigned_places.iter()
        .map(|name| resolve_name(names, name, &format!("agent '{}' patrol route", agent_def.name)))
        .collect::<Result<Vec<_>, _>>()?;
    txn.set_component_patrol_route(agent_id, PatrolRoute { assigned_places, current_index: 0 })?;
}
if let Some(ref profile) = agent_def.pursuit_profile {
    txn.set_component_pursuit_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.facility_queue_disposition {
    txn.set_component_facility_queue_disposition_profile(agent_id, profile.clone())?;
}
if let Some(ref profile) = agent_def.commodity_valuation {
    txn.set_component_commodity_valuation_profile(agent_id, profile.clone())?;
}
if let Some(ref prefs) = agent_def.substitute_preferences {
    txn.set_component_substitute_preferences(agent_id, prefs.clone())?;
}
```

### 5. Runtime enforcement for universal profiles

For each universal profile, audit the primary access sites and replace silent fallbacks with `expect()`:

- `PerceptionProfile`: accessed in perception systems, Tell handler, candidate generation. Replace `if let Some(profile) = ...` with `.expect("agent must have PerceptionProfile")` where the caller already verified the entity is an agent.
- `TellProfile`: accessed in `emit_social_candidates()` and Tell handler. Replace `if let Some(profile) = ctx.view.tell_profile(...)` patterns.
- `ReasoningProfile`: accessed in planning. Replace `unwrap_or_default()` with `expect()`.
- `CommunicationProfile`: accessed in Tell handler. Replace `unwrap_or_default()` with `expect()`.
- `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `PreferenceProfile`: audit and convert similarly.

**Important**: Only convert access sites where the entity is known to be an agent. Some code paths query profiles on arbitrary entities (which may be places, items, etc.) — those must remain `Option`-based. Role-specific profiles keep their `if let Some(...)` access pattern everywhere — they are genuinely optional.

### 6. Documentation updates

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

**General principle** (also in `docs/spec-drafting-rules.md`):

```markdown
Any new ECS component that affects agent behavior must be exercisable through the
scenario system. If a component changes what an agent can do, perceive, decide, or
communicate, a scenario author must be able to configure it. Silent absence of
behavioral components is a bug, not a feature.
```

### 7. Update CLI evaluation scenario

After all profiles are scenario-definable, update `scenarios/cli-evaluation.ron` to exercise agent diversity:

- Give agents varied `PerceptionProfile` fidelity values.
- Give Merchant Vara a `CommunicationProfile` with non-default gossip acceptance.
- Give Guard Theron a `PatrolProfile` and `PursuitProfile`.
- Give at least one agent a `TheftDispositionProfile` to enable theft-related affordances.
- Give agents varied `DriveThresholds` (e.g., Guard Theron with higher danger tolerance, Forager Lina with lower hunger thresholds).

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

No cross-system coupling changes. Modifications span `worldwake-core` (Default impls), `worldwake-cli` (scenario loading), `worldwake-ai` and `worldwake-systems` (access site enforcement). Profile components are read by systems as before — this spec ensures they exist on scenario-spawned agents and converts access patterns from silent fallback to expect where appropriate.

## Migration Path

1. Add missing `Default` impls for `EpistemicDispositionProfile`, `IntentionDispositionProfile`, `PreferenceProfile` in `worldwake-core` (Deliverable 2).
2. Add `PatrolRouteDef` and all missing fields to `AgentDef` in `types.rs` (Deliverable 3).
3. Update `spawn_agent()` — universal profiles unconditional, already-defaulted profiles overridable, role-specific profiles conditional (Deliverable 4).
4. Audit and convert universal profile access sites from silent fallback to `expect()` in `worldwake-ai` and `worldwake-systems` (Deliverable 5).
5. Update `docs/spec-drafting-rules.md` and `CLAUDE.md` (Deliverable 6).
6. Update `scenarios/cli-evaluation.ron` with diverse profiles (Deliverable 7 — scenario skill's job, done separately).
7. Update existing `types.rs` tests to cover new fields.

## Verification

- `cargo test -p worldwake-core` — new Default impls compile and have reasonable values.
- `cargo test -p worldwake-cli` — scenario deserialization tests pass with new fields.
- `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit` — scenario loads.
- `cargo test --workspace` — no regressions from `expect()` conversions.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- Inspect `spawn_agent()` — every universal profile has an unconditional `set_component_*` call; DriveThresholds, MetabolismProfile, CarryCapacity now use override-or-default pattern.
- Inspect `AgentDef` — every missing profile from Deliverable 1 has a field.
- `docs/spec-drafting-rules.md` contains the profile completeness checklist.
- `CLAUDE.md` contains the scenario profile completeness invariant.
