---
name: scenario-designer
description: "Designs and writes rich RON scenarios by scanning the codebase for available types and profiles, interpreting a theme or request, and producing a scenario that exercises multiple AI systems with realistic tensions."
user-invocable: true
arguments:
  - name: theme
    description: "Theme or specific request for the scenario (e.g., 'a frontier trading post threatened by bandits' or '5 agents, contested office, supply chain')"
    required: true
---

# Scenario Designer

Designs and writes rich RON scenarios for the Worldwake CLI. The skill scans the codebase to discover available types (commodities, workstations, profiles, goal kinds), interprets a user-provided theme or request, and produces a scenario that exercises multiple AI systems with realistic tensions.

Handles both vague themes ("a village under bandit threat") and specific requests ("4 agents, contested office, supply chain disruption"). Vague prompts get full creative design. Specific prompts get filled in with appropriate profiles.

## Invocation

```
/scenario-designer <theme>
```

**Arguments** (required, positional):
- `<theme>` — theme or specific request for the scenario

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path.

## Process

Follow these 3 phases in order. Do not skip any phase.

---

### Phase 1 — Codebase Discovery

Build a "capability palette" of what the scenario can use by scanning the codebase. Use up to 3 Explore agents in parallel for steps 1-5.

#### Step 1: Read Scenario Format

Read `crates/worldwake-cli/src/scenario/types.rs` to understand the full `ScenarioDef` structure:
- `PlaceDef` fields and `PlaceTag` variants
- `EdgeDef` fields
- `AgentDef` fields — all profile types and their field names
- `ItemDef` fields and `CommodityKind` variants
- `FacilityDef` fields and `WorkstationTag` variants
- `ResourceSourceDef` fields

This is the authoritative source for what the RON format supports.

#### Step 2: Discover Available Enums

Grep the codebase for:
- `CommodityKind` enum variants (what commodities exist)
- `WorkstationTag` enum variants (what facilities can be placed)
- `PlaceTag` enum variants (what place types exist)
- `ControlSource` variants (Human, Ai, None)

#### Step 3: Discover Agent Behaviors

Grep for `GoalKind` enum variants to understand what behaviors agents can exhibit. This informs which profile types matter:
- If combat goals exist → agents can have `CombatProfile`
- If trade goals exist → agents can have `MerchandiseProfileDef`, `TradeDispositionProfile`
- If patrol goals exist → agents can have `PatrolProfile`, `PatrolRouteDef`
- If crime goals exist → agents can have `TheftDispositionProfile`, `JusticeDispositionProfile`
- If social goals exist → agents can have `TellProfile`, `CommunicationProfile`

#### Step 4: Read Existing Scenario

Read `scenarios/cli-evaluation.ron` (or the most complex existing scenario) as a format reference. Note:
- RON syntax conventions (field order, nesting, comment style)
- How profiles are structured in practice
- Which `Permille` values are typical (e.g., `pm(500)` for moderate, `pm(800)` for high)

#### Step 5: Coverage Awareness

Read `docs/generated/golden-coverage-matrix.md` to understand which GoalKinds, ActionDomains, and systems have golden coverage. Prefer designing scenarios that exercise well-tested systems.

---

### Phase 2 — Scenario Design (with user gate)

Interpret the user's theme/request against the capability palette from Phase 1.

#### Step 6: Design Topology

Create places with tags and travel edges:
- **Place naming**: Evocative names that match the theme (e.g., "Thornwall Market", "Forest Crossing")
- **Place tags**: Match the functional role (Village for hubs, Farm for production, Store for trade, Forest for concealment, Camp for bandit bases, etc.)
- **Edge design**: Create meaningful distances. Not everything adjacent. Include at least one remote location requiring multi-hop travel. Bidirectional by default; one-way shortcuts are interesting.
- **Target**: 5-10 places for rich scenarios, 3-5 for focused ones.

#### Step 7: Design Agent Cast

For each agent, determine:
- **Name**: Thematic, evocative
- **Role**: What they do in the world (merchant, guard, farmer, bandit, magistrate, traveler, etc.)
- **Control source**: Exactly one `Human` agent. All others `Ai`.
- **Starting location**: Where the agent begins
- **Starting needs**: Non-zero hunger/thirst to create immediate pressure. Vary between agents.
- **Profile selection**: Choose profiles that match the role. Every AI agent must have at least `utility_profile` and `perception_profile`. Role-specific profiles on top:
  - Merchant: `merchandise_profile`, `trade_disposition`, `commodity_valuation`
  - Guard: `combat_profile`, `patrol_profile`, `patrol_route`, `justice_disposition`, `violation_disposition`
  - Farmer: `metabolism_profile`, preference for production
  - Bandit: `combat_profile`, `pursuit_profile`, `theft_disposition`, high courage
  - Politician: high `enterprise_weight`, office-related profiles
- **Profile diversity** (P22): Agents in the same role should differ. Different utility weights, perception fidelity, courage, patience.
- **Target**: 5-8 agents for rich scenarios, 3-4 for focused ones.

#### Step 8: Design Economy

- **Items**: Starting commodities placed at agents or locations. Include coin for trade. Include food for immediate needs. Include tools/weapons for combat/production.
- **Facilities**: Workstations at appropriate locations (Mill at village, OrchardRow at farm, Forge at smithy, GravePlot at cemetery).
- **Resource sources**: Regenerating resources at production locations (apples at orchards, grain at fields). Set capacity and regeneration rate.
- **Scarcity**: Don't over-provision. Agents should need to travel, trade, or produce to satisfy needs.

#### Step 9: Identify Tensions

For every scenario, identify at least 2-3 natural tensions:
- **Resource scarcity**: Not enough food for everyone → competition, trade, theft
- **Contested facilities**: Limited workstations → contention queues
- **Information asymmetry**: Bandits know the forest; merchants don't
- **Jurisdictional overlap**: Guard's patrol doesn't cover the trading road
- **Political ambition**: Multiple candidates for a vacant office
- **Supply chain dependency**: Village needs bread → needs grain → needs farmer → farmer needs safety

#### Step 10: Present Design

Present to the user:

```
## Scenario Design: <name>

### Theme
<1-sentence theme interpretation>

### Topology
<Place list with tags and edge diagram>

### Agent Cast
<For each agent: name, role, control, key profiles, starting condition>

### Economy
<Items, facilities, resource sources>

### Expected Dynamics
<Brief narrative: what systems are exercised, what tensions exist, what emergent chains you might observe over 100-500 ticks>

### Systems Exercised
<Bulleted list of which AI systems this scenario activates>
```

**Wait for user approval.** If the user wants changes, revise and re-present.

---

### Phase 3 — Write RON File

#### Step 11: Write Scenario

After approval, write to `scenarios/<kebab-name>.ron`.

The RON file must:
1. Start with a comment block containing the brief narrative (theme, tensions, expected dynamics)
2. Use the exact field names and types from `types.rs`
3. Use `Permille` values via `pm(N)` syntax
4. Include a `seed` for reproducibility
5. Match the formatting conventions of existing scenarios (indentation, field order)

#### Step 12: Present Summary

After writing:
```
Scenario written to `scenarios/<name>.ron`.

To play: `cargo run -p worldwake-cli -- --scenario scenarios/<name>.ron`
```

Do NOT commit. Leave the file for user review.

---

## Scenario Design Principles

These principles guide every scenario the skill creates:

1. **Every agent has a reason to act** — no idle agents. Hunger, enterprise, danger, social weight, patrol duty — every agent has at least one drive that will generate goals within the first 20 ticks.
2. **Natural tensions** — scarce resources, contested facilities, overlapping jurisdictions, information asymmetry. The scenario should create situations where agents' goals conflict or compete.
3. **Exercise multiple systems** — a good scenario involves at least 3 of: needs/metabolism, production, trade, combat, perception/belief, social/tell, offices/politics, patrol, crime/justice.
4. **Realistic topology** — meaningful travel times. Not everything adjacent. Include at least one remote location requiring multi-hop travel. Travel should cost enough ticks that bladder/fatigue escalation matters.
5. **One human-controlled agent** — exactly one `Human` control source for play-testing. Place them at a central location with enough starting resources to survive but not enough to be idle.
6. **Profile diversity** — agents in the same role differ (different utility weights, perception fidelity, courage, patience) per FOUNDATIONS Principle 22. Two guards should not be identical.
7. **Seed reproducibility** — always include a seed so the scenario is deterministic and reproducible.
8. **Don't over-provision** — scarcity drives behavior. If everyone has enough food, nobody trades. If there's no danger, nobody patrols. Make agents *need* to act.

## Guardrails

- **Codebase truth**: Only use types, enums, profile fields, and values that actually exist in the codebase as discovered in Phase 1. Never hallucinate CommodityKinds, WorkstationTags, PlaceTags, or profile fields.
- **RON format fidelity**: The output must parse as valid RON matching the `ScenarioDef` structure in `types.rs`. Use the existing scenario as format reference.
- **No commit**: Write the file and stop. The user handles the file lifecycle.
- **Worktree discipline**: If working in a worktree, ALL file operations use the worktree root path.
- **FOUNDATIONS alignment**: Scenarios must not violate project principles. No magic spawners (P1), no omniscient agents (P14), no zero-tick travel (P8). Every resource must have a source. Every agent's starting inventory must be consistent with their location.
- **Profile completeness**: Every AI agent must have at least `utility_profile` and `perception_profile`. Universal profiles (`HomeostaticNeeds`, `MetabolismProfile`) are always applied with defaults if not specified. Role-specific profiles added only when the role demands them.
- **Preserve existing scenarios**: Never modify `scenarios/cli-evaluation.ron`. Always write new files.
