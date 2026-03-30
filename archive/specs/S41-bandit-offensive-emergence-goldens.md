# S41: Bandit Offensive Emergence — Golden E2E Coverage

**Status**: COMPLETED

## Summary

Three golden E2E suites covering the offensive/economic half of E18 bandit dynamics. The existing T22 suite proves the defensive chain (external attack -> camp destruction -> regrouping -> re-establishment). S41 extends that coverage with proactive raid emergence, belief-mediated merchant rerouting, and wound-dampened raid suppression.

## Motivation

E18 implemented a rich emergent system — bandit camps where raids, looting, route danger, and merchant adaptation all arise from interacting subsystems without scripted triggers. T22 validated only the destruction/regrouping path. S41 closes that gap with live golden coverage for offensive emergence and its downstream consequences.

| Behavior | Systems Involved | Why It Matters |
|----------|-----------------|----------------|
| Bandit-initiated raids (`RaidTarget` goal) | AI candidate gen, Combat, Loot | Core offensive behavior now has direct golden proof |
| Supply pressure interacting with raid opportunity | Needs, AI ranking, Combat | Offensive emergence is now exercised through concrete local opportunity and commodity motive |
| Merchant route adaptation from raid beliefs | Beliefs, Enterprise, Travel, Trade | Downstream economic impact remains belief-local and physically transmitted |
| Wound accumulation dampening raid frequency | Combat, Wounds, AI pressure | Physical dampener is now proven through shared wound-deterrence pressure logic |

## Crate

`worldwake-ai` (golden tests in `crates/worldwake-ai/tests/`)

## Dependencies

All completed:
- E18 (bandit dynamics — `RaidTarget`, `EstablishBanditCamp`, `RegroupWithFaction`, `BanditCamp`, `BanditFactionPolicy`)
- E12 (combat — `CombatProfile`, `WoundList`, attack/loot actions)
- E13 (decision architecture — GOAP planner, pressure system, candidate generation)
- E14/E15 (beliefs, perception — danger beliefs, `BelievedActivity`)
- E11 (trade — `MerchandiseProfile`, `DemandMemory`, enterprise signals)

## Foundational Alignment

| Principle | How These Suites Validate Compliance |
|-----------|-------------------------------------|
| FND-1 (Maximal Emergence) | Raids emerge from lawful co-location, visible loot, and local motive rather than scripted schedules |
| FND-2 (No Ungrounded Triggers) | No `raidChance` or encounter dial exists; raid proof depends on concrete entities and beliefs |
| FND-3 (Concrete State Over Abstract Scores) | Route danger is still belief-derived planner cost, and wound deterrence is derived from concrete `WoundList` state |
| FND-7 (Locality) | Merchant route changes only after a witness physically travels and tells them |
| FND-10 (Physical Dampeners) | Raid suppression is driven by accumulated wounds crossing a faction/courage-scaled threshold |
| FND-12 (World State != Belief State) | Merchant acts on acquired belief, not authoritative omniscience |
| FND-17 (Agent Symmetry) | Bandits use the same attack, loot, travel, and planner machinery as everyone else |
| FND-24 (Systems Interact Through State) | Needs, wounds, combat, beliefs, and enterprise interact through shared state instead of direct calls |
| FND-25 (Derived Summaries Are Caches) | Route threat and raid deterrence remain derived planning inputs, not authoritative stored state |

## Deliverables

### Suite 1: Pressure-Driven Raid Emergence (Scenario 47)

**File**: `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`

**Tests**: `golden_pressure_driven_raid_emergence` + `golden_pressure_driven_raid_emergence_replays_deterministically`

Covered in live test source and generated scenario inventory.

### Suite 2: Raid-Belief Economic Cascade (Scenario 48)

**File**: `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`

**Tests**: `golden_raid_belief_economic_cascade` + `golden_raid_belief_economic_cascade_replays_deterministically`

Covered in live test source and generated scenario inventory.

### Suite 3: Wound-Dampened Raid Spiral (Scenario 49)

**File**: `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`

**Tests**: `golden_wound_dampened_raid_spiral` + `golden_wound_dampened_raid_spiral_replays_deterministically`

Covered in live test source and generated scenario inventory.

## Outcome

- Completion date: 2026-03-30
- What actually changed: S41 shipped as six live goldens in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`, plus regenerated inventory/docs coverage for Scenarios 47-49.
- Deviations from original plan: the draft spec still described Scenario 49 as a pending engine-gap item. Live code already resolved that architecture by centralizing wound-based raid deterrence in `crates/worldwake-ai/src/pressure.rs` and reusing that helper from both `candidate_generation.rs` and `ranking.rs`, which is the cleaner long-term design.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_pressure_driven_raid_emergence` ✅
  - `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_raid_belief_economic_cascade` ✅
  - `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_wound_dampened_raid_spiral` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `python3 scripts/golden_inventory.py --write --check-docs` ✅
