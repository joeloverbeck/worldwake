# Design: `docs/scenario-roadmap.md` + `docs/generated/scenario-coverage.md`

## Brainstorm Context

**Original request.** Design a document in `docs/*` that captures (1) the prioritary order in which gameplay features should be added to new scenarios, and (2) which gameplay features are already supported by which scenarios. Identifying gameplay features from `scenarios/*.ron` is non-trivial: features are gated behind profile structs on agents, but some profiles (notably `TellProfile`) are *present but inactivated* by zero-valued fields, and those should not count as covered. Deliverable is the document itself.

**Context.** After ~15 gameplay features landed with unit-scoped goldens, a 1440-tick observer run revealed the AI architecture could not even sustain basic needs when all features were active. Fixing the architecture required breaking many existing goldens. Philosophy shifted: goldens must be backed by `scenarios/*.ron` observer runs. Baseline survival competence is now proven; future scenarios stack features one at a time while survival remains a coexistence invariant.

**Reference files read.** `scenarios/*.ron` (all 5), `crates/worldwake-cli/src/scenario/types.rs` (`AgentDef` as authoritative feature catalog), `docs/profiles/all-profiles.md`, `docs/generated/golden-coverage-matrix.md`, `docs/golden-e2e-testing.md`, `crates/worldwake-systems/src/` (system modules).

**Interview insights that shaped the design.**
- Priority criterion: **architectural risk**, not gameplay-surface importance. Rationale: solidifying the architecture as features integrate produces a sturdier final system than feature-order driven by player-facing priorities.
- Each entry is not just a to-do — it's a **scenario contract** listing what features the scenario must exercise and what invariants the backing golden must prove.
- **Survival-always is a first-class invariant.** Every future scenario runs the proven survival loop AND the new feature. No feature-isolation scenarios. Done-when gates always include survival-health contract compliance.
- Maintenance: hand-authored priority and contract content + machine-generated status/coverage companion file, matching the project's `scripts/profile_docs.py` + `scripts/golden_inventory.py` precedent.

**Final confidence.** 95% at the interview-to-approaches transition. No assumptions carried forward — every gap was resolved in-band.

---

## Deliverables

Three artifacts, created together:

1. **`docs/scenario-roadmap.md`** — hand-authored roadmap + feature catalog + per-scenario retrospective + maintenance workflow + detection rule appendix.
2. **`docs/generated/scenario-coverage.md`** — machine-generated feature×scenario matrix + per-scenario profile detail. Parses every `scenarios/*.ron` through the existing `ScenarioDef` deserializer.
3. **`crates/worldwake-cli/src/bin/scenario_coverage.rs`** — small Rust binary that produces (2). Supports `--write` and `--check` (CI).

`docs/scenario-roadmap.md` is the canonical source of editorial intent. The generated file is its evidence companion — the two must agree on what is Landed, and CI enforces it.

---

## 1. Structure of `docs/scenario-roadmap.md`

Top-level outline:

1. **Preamble / Philosophy** — scenarios back goldens; survival-always invariant; architectural-risk ordering; feature-stacking rule; one-feature-at-a-time cadence.
2. **Gameplay Feature Catalog** — canonical list of every gameplay feature the simulation supports, mapped to the profiles and fields that activate it. Reference lookup, not a roadmap.
3. **Status Summary** — short table: feature → first scenario that landed it (or "Planned — see row N"). Derived from the generated companion.
4. **Priority Roadmap** — ordered list of next scenarios to build, each entry using the contract template (Section 2 below).
5. **Landed Scenarios** — one section per landed `.ron`, using the same contract template in retrospective form, plus pointers to backing goldens.
6. **Maintenance Workflow** — how to add an entry, run the coverage binary, close an entry, handle schema drift.
7. **Detection Rule Appendix** — the formal "active vs present-but-inactive" rule, per profile and world feature.

---

## 2. Entry Contract Template

Every row in the Priority Roadmap (and the retrospective Landed Scenarios section) uses this template so there's no guesswork picking up the next one:

```markdown
### N. <Feature Name>

**Status**: Planned | Drafting | In Progress | Landed
**Source scenario**: `scenarios/<name>.ron` (or "—" until authored)
**Backing goldens**: `crates/worldwake-ai/tests/golden_<name>.rs` (or "—")
**Depends on**: #<prior entry numbers>, landed specs S<NNN>

**Architectural risk rationale**
1–3 sentences. Why integrating this feature into the survival loop is
non-trivial — the planner, perception, or cross-system interaction
we expect to stress. Cite FOUNDATIONS principles when relevant
(e.g., FND-14 belief-only planning, FND-26 system decoupling).

**Activation checklist**
- **Always required (survival baseline)**: HomeostaticNeeds, UtilityProfile
  with non-zero survival weights, MetabolismProfile, DriveThresholds,
  PerceptionProfile, ExplorationProfile, known_recipes for baseline food/water.
- **Newly activated for this feature**:
  - <profile> — why it needs to be Some(...)
  - <UtilityProfile field> must be > 0 — why
  - <world feature> — e.g., add a visibility_profile on forest places
- **Survival-health contract**: `max_authored_critical_run_ticks`,
  `required_self_care_families`, and any `critical_run_limits`.

**Must-exercise behaviors** (what the 1440-tick run must actually produce)
- Concrete behavioral events, not just profile presence. Examples:
  - "At least one agent executes a Tell action that is accepted by another
    agent whose CommunicationProfile allows testimony."
  - "Trade settlement observed at least N times between <roles>."
- Each bullet must be assertable from action traces, authoritative state,
  decision traces, or event log per `docs/golden-e2e-testing.md`.

**Must-prove invariants** (what the backing golden asserts)
- Survival-health contract passes for every agent (baseline invariant,
  always listed first).
- Feature-specific invariants derived from Must-exercise.
- Negative invariants when appropriate: "No agent collapses from dehydration
  while trading" guards against new-feature regressions on survival.

**Deliberately inactive** (cumulative from prior entries plus anything this
entry still zeros out)
- Explicit list. Each item links to the roadmap entry that will activate it.

**Done-when**
- Scenario file exists and `cargo run -p worldwake-cli --bin scenario-coverage`
  shows it as Landed.
- Golden file exists, passes, and is referenced from the entry.
- Every Must-exercise bullet is proved by a trace-backed assertion.
- Survival-health contract passes across all observer-run ticks.
- Status flipped to Landed; generated companion regenerated and committed.
```

**Design notes embedded in the template:**

- **Deliberately inactive** is cumulative until a later entry flips the item. This keeps the feature-stacking rule auditable — looking at entry N tells you exactly what is still off.
- **Must-exercise is behavioral, not structural.** A scenario declaring `tell_profile: Some(...)` but producing no Tell actions across 1440 ticks does not count as covering Tell. The coverage binary verifies structural activation; the golden proves behavioral activation. The template makes the split explicit.

---

## 3. Gameplay Feature Catalog (Section 2 of the doc)

Reference lookup. Hand-curated because "gameplay feature" is an editorial grouping that doesn't map 1:1 to profile structs. Shape:

| Feature | Activation signal | Backing systems |
|---|---|---|
| Basic needs (Eat/Drink/Sleep/Relieve/Wash) | `HomeostaticNeeds` + survival weights > 0 + `MetabolismProfile` + `DriveThresholds` + matching `known_recipes` | needs, needs_actions |
| Travel physiology | `MetabolismProfile.travel_*_multiplier > 0`, `wilderness_relief_dirtiness_penalty > 0` | needs, travel_actions |
| Drive escalation | `DriveEscalationProfile` non-default or per-need override | needs |
| Need-driven exploration | `ExplorationProfile.curiosity_weight > 0` + frontier places | perception, needs |
| Activation-decay perception | `PerceptionProfile.entity_activation_threshold`, `observation_buffer_capacity` | perception |
| Place concealment | `PlaceDef.visibility_profile.base_concealment > 0` | perception |
| Tell / peer info transfer | `TellProfile.max_tell_candidates > 0` + `CommunicationProfile` acceptance > 0 | tell_actions |
| Ask-about-person | `UtilityProfile.social_weight > 0` + handler preconditions | ask_about_person_actions |
| Consult-record | office/record context + `social_weight > 0` | consult_record_actions |
| Obligation satiation | `ObligationSatiationProfile` present | obligation |
| Diversification / curiosity | `DiversificationProfile` present | diversification |
| Experience preferences (learned routes) | `PreferenceProfile` present | experience_recording |
| Production (multi-input recipes) | multi-input `known_recipes` + facilities | production_actions |
| Merchant selling | `MerchandiseProfile` present | trade_actions |
| Trade negotiation | `TradeDispositionProfile` present | trade_actions |
| Commodity valuation | `CommodityValuationProfile` present | trade_actions |
| Substitute preferences | `SubstitutePreferences` present | substitute preference module |
| Item decay | `ScenarioDef.commodity_decay` present | item_decay |
| Disposal | `DisposalProfile` present | disposal handler |
| Facility-queue contention | `ContentionDispositionProfile` present | facility_queue_actions |
| Offices / succession / force-claim | offices spawned + `OfficeForceProfile` | office_actions |
| Bounty posting | `UtilityProfile.bounty_posting_weight > 0` + `ArtifactPostingProfile` | artifact_actions |
| Notice posting | `UtilityProfile.notice_posting_weight > 0` + `ArtifactPostingProfile` | artifact_actions |
| Theft | `TheftDispositionProfile` present | theft handler |
| Justice / accusation | `JusticeDispositionProfile` present | justice_actions |
| Violation investigation | `ViolationDispositionProfile` present | investigate_actions |
| Patrol | `PatrolProfile` + `PatrolRoute` present | patrol_actions |
| Pursuit | `PursuitProfile` present | pursuit handler |
| Combat | `CombatProfile` present | combat |
| Escort | `UtilityProfile.care_weight > 0` + handler preconditions | escort_actions |
| Bandit camps | Bandit-agent entities + camp places | bandit_camp_actions |
| Report / witness | perception sees event + `TellProfile` active | report_actions |
| Search (investigation) | `ViolationDispositionProfile` + evidence | search_actions |
| Stock / transport | facility stock state + merchant role | stock_actions, transport_actions |

Each row links to the relevant spec (if any), the module source file, and — once a scenario lands — the roadmap entry that first activates it.

---

## 4. Priority Roadmap (Section 4 of the doc)

**Ordering criterion stated at the top of the section:**

> Architectural risk ≈ how likely this feature is to destabilize the proven survival loop when combined with it. Factors, in order: (1) does it add goals that can starve survival for planner attention? (2) does it mutate the belief store in ways that can corrupt need-driven perception? (3) does it stress planner budget (depth, beam, candidate expansion)? (4) does it require cross-system information flow that tests FND-26 decoupling? (5) does it introduce multi-agent coordination?

**Proposed order (cohorts — each cohort is one roadmap entry, refined at authoring time):**

| # | Scenario (working name) | New features | Risk driver |
|---|---|---|---|
| 1 | survival-baseline | basic needs + activation-decay perception + need-driven exploration | Landed — `scenarios/survival-baseline.ron` |
| 2 | survival-scattered | + travel physiology + wilderness penalty | Landed — `scenarios/survival-scattered.ron` |
| 3 | survival-contested + drive-escalation-wash-priority | + implicit facility contention + drive escalation | Landed — two files, one cohort |
| 4 | survival-tell | + Tell + CommunicationProfile non-default + obligation satiation | Goal-attention competition; belief mutation from peers |
| 5 | survival-ask-consult | + ask-about-person + consult-record | Epistemic queries compete with needs; cross-system belief flow |
| 6 | survival-preferences | + PreferenceProfile + DiversificationProfile | Learned-route lock-in vs. exploration; curiosity overriding needs |
| 7 | survival-production | + multi-input recipes + new facility kinds | Planner depth + prerequisite stacking |
| 8 | survival-trade | + MerchandiseProfile + TradeDispositionProfile + CommodityValuationProfile + SubstitutePreferences | Multi-party coordination, trade timing vs. needs, belief-only ownership |
| 9 | survival-items-decay | + commodity_decay + DisposalProfile + explicit ContentionDispositionProfile | Notice/act on decay without starving; contention profile becomes explicit |
| 10 | survival-offices | + offices + succession + force-claim + bounty/notice posting + ArtifactPostingProfile | Institutional goals vs. survival; artifact posting cadence |
| 11 | survival-theft | + TheftDispositionProfile + place visibility/concealment | Antagonistic goals; concealment affects perception |
| 12 | survival-justice | + JusticeDispositionProfile + ViolationDispositionProfile + investigate + search + report | Witness-chain belief propagation; accusation cascade |
| 13 | survival-patrol | + PatrolProfile + PatrolRoute + PursuitProfile | Scheduled routes vs. needs; pursuit interruption |
| 14 | survival-combat | + CombatProfile + bandit camps | Wound-induced survival compromise; adversarial AI |
| 15 | survival-escort | + care_weight + escort | Coordinated travel; multi-agent survival |
| 16 | **final integration** | all of the above, tuned for coexistence | Full cross-system stress |

Cohort groupings are provisional. At authoring time, an entry may split (e.g., justice into accusation + investigation) — the architectural-risk rationale must justify the split, and the new entry inserts before the original.

---

## 5. Landed Scenarios (Section 5 of the doc — seed content)

Retrospective entries for the five current `.ron` files, using the contract template.

**`survival-baseline.ron`** (entry #1) — seed: 104004; 3 Ai agents; 4 places; `max_authored_critical_run_ticks: 100`; families Eat/Drink/Sleep/Relieve/Wash. Active: basic needs, activation-decay perception, need-driven exploration, harvest-only production, implicit low-pressure contention. Deliberately inactive: Tell (zeros), all UtilityProfile non-survival weights zero, combat, trade, theft, justice, patrol, pursuit, obligation, diversification, preference, item decay, offices, place visibility, escort, bandit camps. Backing golden: *(to trace from existing `golden_*.rs` at authoring time)*.

**`survival-scattered.ron`** (entry #2) — seed: 205005; 3 Ai agents; 6 places; `max_authored_critical_run_ticks: 550`. Adds: travel physiology (`travel_*_multiplier > 0`), `wilderness_relief_dirtiness_penalty > 0`, grain recipe, spatially separated resources, isolated starting positions. Same inactive list as #1 minus travel physiology.

**`survival-contested.ron` + `drive-escalation-wash-priority.ron`** (entry #3) — the contention-and-escalation cohort. Contested: seed 306006; 4 Ai agents; 8 places; `max_authored_critical_run_ticks: 300` with `critical_run_limits.dirtiness: 1300`. Adds: capacity-4 water wells (explicit contention pressure), two water sources (mid-plan belief invalidation), chokepoint topology. Drive-escalation: seed 116006; 2 Ai agents; 3 places; focuses drive escalation on the wash-priority branch. Same inactive list as #2 minus explicit contention and drive-escalation stress.

**`cli-evaluation.ron`** — called out separately under subheading **Non-golden-backed scenarios**.
- Purpose: exercise all CLI commands with broad profile coverage. Touches combat, trade, theft, justice, patrol, posting, etc.
- Explicitly *not* a survival-health-contract scenario — no `survival_health_contract` field, not a 1440-tick observer run, not backed by a golden.
- Exists to prevent CLI regressions when profile schemas drift.
- **The roadmap notes: do not interpret CLI-evaluation's broad profile coverage as "feature X is proved".** Feature proofs require a survival-health-contract scenario with a backing golden.

---

## 6. Maintenance Workflow (Section 6 of the doc)

**Adding a new roadmap entry (planned feature)**
1. Pick the next architectural-risk cohort from Section 4.
2. Create a new entry from the Section 2 template. Status: `Planned`. Source scenario: `—`.
3. Fill rationale, activation checklist, must-exercise, must-prove, prerequisites, deliberately inactive (cumulative from the last landed entry + anything this entry still leaves off), done-when.
4. Update the Status Summary table.

**Authoring a scenario for a planned entry**
1. Flip status to `Drafting`. Copy the closest existing `.ron` as a starting point.
2. Apply the Activation checklist — each bullet maps to a RON edit.
3. Run the observer for 1440 ticks locally. Iterate on starting needs, profile weights, and topology until the survival-health contract holds and must-exercise bullets produce observable events.
4. Write the backing golden under `crates/worldwake-ai/tests/golden_<name>.rs`, asserting every must-prove invariant (survival contract first, feature invariants next, negative invariants last).
5. Run `cargo run -p worldwake-cli --bin scenario-coverage -- --write`. Verify the generated companion shows the entry as `Landed` and that active profiles match the Activation checklist. On mismatch, fix the checklist (the RON is the source of truth; the doc is derived editorial).
6. Flip status to `Landed`. Fill in Source scenario + Backing goldens paths. Commit doc + `.ron` + golden + regenerated companion together.

**Handling schema drift (profile struct adds/removes a field)**
- The binary fails loudly if `ScenarioDef` deserialization breaks on any scenario. CI runs the generator with `--check` (no writes) and fails if the committed companion file differs from the freshly generated one.
- When a profile struct changes, `scripts/profile_docs.py --write` is already run. The scenario-coverage run joins the same pre-commit workflow so both stay in lockstep.

**Closing out an entry**
- Done-when checklist must be ticked off. Status flips to `Landed` only when every box is ticked.
- If the feature is found during authoring to split naturally into two scenarios, the original entry is re-planned and a new entry inserted before it in the order. The architectural-risk rationale must justify the split.

---

## 7. Detection Rule Appendix (Section 7 of the doc)

**Rule.** A gameplay feature is *active in scenario S* iff all of:
1. Every required profile in the Feature Catalog is `Some(...)` on at least one agent in S (structural presence), AND
2. For each gating field identified in the Feature Catalog row, the value is non-zero / non-default (behavioral enablement), AND
3. The world conditions required by the feature exist in S.

**Gating fields per profile** (source of truth — the binary implements exactly this):

| Profile | Gating field(s) for "active" |
|---|---|
| `UtilityProfile` | Per-feature: `social_weight > 0` → social; `enterprise_weight > 0` → enterprise; `activity_awareness_weight > 0` → activity awareness; `side_benefit_weight > 0` → side benefits; `bounty_posting_weight > 0` → bounty posting; `notice_posting_weight > 0` → notice posting; `care_weight > 0` → escort/care |
| `TellProfile` | `max_tell_candidates > 0` AND `conversation_memory_capacity > 0` |
| `MetabolismProfile` | `travel_*_multiplier > 0` → travel physiology; `wilderness_relief_dirtiness_penalty > 0` → wilderness penalty |
| `DriveEscalationProfile` | Any per-need entry whose `start_after_ticks` or `growth_per_tick` differs from the default profile |
| `PerceptionProfile` | Universal — field-granular; `institutional_memory_capacity > 0` gates institutional memory, etc. |
| `CommunicationProfile` | `testimony_acceptance > 0` AND `gossip_acceptance > 0`, together with an active Tell gate downstream |
| Optional profiles (Combat, Merchandise, TradeDisposition, Patrol, Pursuit, Justice, Theft, Violation, Contention, Valuation, Disposal, Diversification, Preference, Obligation, Artifact, Substitute, LastSeenMemory, Expectation, Epistemic, Intention) | Presence as `Some(...)` is sufficient for structural activation — these profiles have no "all-zero means disabled" convention. Scenarios deactivate them by omission. |

**World-feature gates** (outside `AgentDef`):
- Item decay: `ScenarioDef.commodity_decay` is `Some(...)`.
- Place concealment: any `PlaceDef.visibility_profile.base_concealment > 0`.
- Office / succession: at least one spawned office entity with `OfficeForceProfile`.
- Facility-queue contention (explicit): at least one `ContentionDispositionProfile` on an agent. Implicit contention pressure is also observable via low-capacity resource sources — the appendix distinguishes explicit from implicit.

---

## 8. Companion Generated File — `docs/generated/scenario-coverage.md`

Machine-generated. Shape:

```markdown
<!-- Generated by `cargo run -p worldwake-cli --bin scenario-coverage -- --write`. -->
<!-- Do not hand-edit. -->

# Scenario Coverage (Generated)

Snapshot of every `scenarios/*.ron` at HEAD. Cross-reference this with
`docs/scenario-roadmap.md` — if a "Landed" entry there claims a feature
this file says is inactive, regenerate and fix one side.

## Feature × Scenario Matrix

| Feature | baseline | scattered | contested | drive-esc | cli-eval |
|---|:-:|:-:|:-:|:-:|:-:|
| Basic needs (Eat) | ✅ | ✅ | ✅ | ✅ | ✅ |
| Travel physiology | — | ✅ | ✅ | — | ✅ |
| Tell / peer info | — | — | — | — | ✅ |
| Trade negotiation | — | — | — | — | ✅ |
| Combat | — | — | — | — | ✅ |
| ...one row per Feature Catalog entry... |

Legend: ✅ active (rule satisfied), ⚠ structurally present but gating
field zero (present-but-inactive), — not present.

## Per-Scenario Detail

### scenarios/survival-baseline.ron

- Seed: 104004
- Agents: 3 — all Ai
- Places: 4
- Survival contract: max_critical 100, families [Eat, Drink, Sleep, Relieve, Wash]

**Active profiles (by detection rule)**
- HomeostaticNeeds, UtilityProfile (survival weights > 0),
  MetabolismProfile (travel multipliers 0 — travel physiology inactive),
  PerceptionProfile, ExplorationProfile, DriveThresholds,
  CognitiveProfile, ExecutionBudget.

**Present-but-inactive (⚠)**
- UtilityProfile.enterprise_weight = 0 (enterprise inactive)
- UtilityProfile.social_weight = 0 (social inactive)
- UtilityProfile.activity_awareness_weight = 0
- UtilityProfile.side_benefit_weight = 0
- UtilityProfile.bounty_posting_weight = 0
- UtilityProfile.notice_posting_weight = 0
- UtilityProfile.care_weight = 0
- TellProfile all zeros (Tell inactive)
- MetabolismProfile.travel_*_multiplier = 0 (travel physiology inactive)
- MetabolismProfile.wilderness_relief_dirtiness_penalty = 0

**Omitted profiles**
- Combat, Merchandise, TradeDisposition, Theft, Justice, Violation,
  Patrol, PatrolRoute, Pursuit, Contention, Valuation, Disposal,
  Diversification, Preference, Expectation, LastSeenMemory,
  ObligationSatiation, DriveEscalation, Substitute, ArtifactPosting.

**World features**
- commodity_decay: absent (item decay inactive)
- Places with visibility_profile: none (concealment inactive)
- Facilities: Well (×2), WashBasin (×2), OrchardRow (×1)
- Resource sources: Water (×2), Apple (×1)
- Known recipes on agents: ["Harvest Apples", "Harvest Water"]

### scenarios/survival-scattered.ron
... (same shape) ...
```

Deterministic output. CI diffs the generated file against the committed copy and fails on drift.

---

## 9. Rust Binary — `crates/worldwake-cli/src/bin/scenario_coverage.rs`

**Contract**
- Input: walks `scenarios/*.ron`, deserializes each via `ron::from_str::<ScenarioDef>` — reuses the existing deserializer, no parallel schema.
- Output: writes `docs/generated/scenario-coverage.md` on `--write`; prints to stdout otherwise. `--check` exits nonzero if in-tree file differs from freshly generated.
- Dependencies: stdlib + already-in-workspace `ron`, `serde`, `worldwake-core`, `worldwake-cli`.

**Core logic**
1. Enumerate `scenarios/*.ron` (glob).
2. For each, deserialize `ScenarioDef`.
3. For each agent, run the per-profile detection rule from Section 7 to classify each feature as `Active | PresentInactive | Absent`. Aggregate across agents: feature is `Active` for the scenario iff at least one agent activates it; `PresentInactive` iff every agent with the profile has it zeroed; `Absent` iff no agent has the profile.
4. Apply world-feature gates (commodity_decay, visibility_profile, resource-source capacities).
5. Emit the matrix + per-scenario detail in the Section 8 format.

**Feature catalog as data**
- The binary contains a `const FEATURES: &[FeatureDef] = &[...]` table mirroring Section 3. Each `FeatureDef` carries `name`, `required_profiles`, `gating_fields`, `world_conditions`. Single source of truth for detection — the doc catalog and binary table track together; CI's `--check` run catches divergence.

**Drift protection**
- `ScenarioDef` additions keep working via serde defaults. The binary emits a warning row at the top of the generated file — "Unrecognized fields detected" — if any scenario uses fields the `FEATURES` table doesn't know about, prompting a catalog update.

**CI integration**
- New workflow step after `scripts/profile_docs.py` and `scripts/golden_inventory.py` checks: `cargo run -p worldwake-cli --bin scenario-coverage -- --check`. Same failure surface, same fix-up flow.

---

## Open items for authoring time

- Trace each currently landed scenario to its backing golden file(s) and fill in the Landed Scenarios section paths.
- Confirm `survival-baseline.ron` has a backing observer-level golden — if not, author one as part of the doc's first commit (the doc claims baseline is landed; the claim must be evidenced).
- Exact naming for the new scenarios in entries #4–#16 can be refined when each entry is picked up; working names in Section 4 are placeholders.
