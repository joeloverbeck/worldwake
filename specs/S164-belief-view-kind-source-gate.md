# S164 — Belief-View Kind Source-Gate + Faction-Policy Footgun Closure

**Status:** DRAFT
**Type:** Correctness fix (closes the residual FND-14/14A entity-kind leak in
`PerAgentBeliefView` that S158/S162's accessor sweep missed; removes the ungated
faction-policy accessor footgun; adds confirming coverage for
`facility_controller_at`). No new authoritative simulation state, system,
component, action, or feedback loop. (Deliverable 2 may add a *belief-carrier*
field — observed kind on the last-seen memory record — which is belief/memory
state, not authoritative world truth.)
**Priority:** Medium. Sequence after `specs/S163-cli-player-pov-boundary.md` in the
fourth-iteration belief-boundary wave. Independent of S163's CLI work — touches the
shared belief view, not the CLI — but S163 is the higher-priority pending item and
should land first.
**Crates:** `worldwake-sim` (`per_agent_belief_view.rs` accessors + last-seen
synthesis), possibly `worldwake-core` (`expectation.rs` `LastSeenRecord` if a kind
field is added), `worldwake-ai` (adversarial goldens).
**Foundations:** FND-7, FND-14, FND-14A, FND-15, FND-19, FND-27, FND-31
**Source:** `reports/ai-architecture-consolidation-fourth-iteration.md` (triage
`docs/triage/2026-05-22-ai-architecture-consolidation-fourth-iteration-triage.md`).

## Problem Statement

### Motivation

The fourth-iteration hostile audit re-raised, as Critical/High, several proposals
the second and third iterations already considered and **explicitly rejected with
documented reasoning** (the `&World`-holding view / capability-trait split; per-field
`SnapshotFieldSource` typing; the `believed_rights`/`can_control` live-read behind a
belief gate; `direct_container`/`direct_possessor`; `merchandise_profile` and reward
encumbrance). It also re-raised the CLI player-menu leak, which is already specced
as pending **S163**. The triage dismisses all of those (see triage doc).

Stripped of re-litigation, the report surfaced **one genuinely new, confirmed leak**
plus two latent footguns worth closing while the belief view is open:

`entity_kind` and the last-seen belief synthesis read **live `world.entity_kind`**
for remote, non-co-located known entities. FND-14A admits kind as a *co-located*
physical observation; reading the *current* kind of a remote entity known only via
last-seen memory is not lawful — kind, like location, must be a stored belief that
can go stale. The leak is narrow but real: if a remote entity changes kind (e.g.,
an agent becomes a corpse on death), the observer "knows" the new kind with no
perception, testimony, record, or memory carrier.

### Evidence (verified against code on 2026-05-22)

- **`crates/worldwake-sim/src/per_agent_belief_view.rs:604-609`** — `entity_kind`
  returns `Place` for places (lawful: public topology) but otherwise
  `self.knows_entity(entity).then_some(self.world.entity_kind(entity))`. `knows_entity`
  includes last-seen-only remote entities, so this returns **current world kind** for
  an entity the actor has not co-located-observed this tick.
- **`crates/worldwake-sim/src/per_agent_belief_view.rs:1293-1304`** — when building
  `known_entity_beliefs`, last-seen-only entities are synthesized with
  `believed_kind: self.world.entity_kind(*entity)` while `last_known_place` and
  `alive: true` come from the (stale-correct) `LastSeenRecord`. This is an internal
  inconsistency: location and aliveness are correctly frozen at observation, but
  **kind is pulled live from world truth**.
- **`crates/worldwake-core/src/expectation.rs:126-132`** — `LastSeenRecord` stores
  `subject`, `place`, `observed_tick`, `source`, `provenance` — **no observed kind**.
  So the synthesis at `:1296` has nothing belief-local to use and reaches for live
  world.
- **`crates/worldwake-sim/src/per_agent_belief_view.rs:611-621`** —
  `bandit_flee_wound_threshold` / `bandit_camp_establishment_ticks` read
  `world.get_component_bandit_faction_policy(faction)` for **any** faction with **no
  accessibility gate**. Today every planner-visible call site
  (`planning_snapshot.rs:701-716`, `pressure.rs:74-86`, `planning_state.rs:1256-1260`)
  passes `bandit_factions_of(actor)` — the actor's *own/believed* factions
  (`:1592-1601`, filtered through the belief/self-gated `factions_of`), so the reads
  are **lawful self-state today**. The accessor signature nonetheless invites a
  future caller to pass an arbitrary faction and silently leak that faction's hidden
  behavioral policy. This is a latent footgun, not an active leak.
- **`crates/worldwake-sim/src/per_agent_belief_view.rs:385-401`** —
  `facility_controller_at` resolves seller/controller identity by calling
  `world.can_exercise_control(entity, facility)` for each agent in
  `self.entities_at(place)`. The candidate set is **belief-filtered** (only agents the
  observer believes are present), so this is defensible as local observation of who is
  staffing a believed-present facility. It is borderline and currently **untested for
  the remote-control-change case**; it warrants a confirming test, not necessarily a
  behavior change.

### Key scoping decisions (brainstorm 2026-05-22)

- **This spec does not re-open the rejected proposals.** Per the triage, the
  `&World`-holding view / `RuntimeBeliefView` capability-trait split and per-field
  `SnapshotFieldSource` typing remain rejected (the snapshot has zero direct `world.`
  reads and is lawful by construction once the view is lawful — S162 Deliverable 6
  locks this; the belief view legitimately holds `&World` because it lives in
  `worldwake-sim`, the lawful observation/dispatch layer). `believed_rights`/
  `can_control` keep S162's deliberate self/belief-accessibility-gated design.
- **Kind for non-co-located entities must come from stored belief, never live world.**
  Mechanism is a ticket-time choice (Deliverable 2): either record the observed kind
  on `LastSeenRecord` at observation time (preferred — preserves kind-at-observation),
  or synthesize `believed_kind: None` when no stored kind exists. Either way the
  accessor must never read `world.entity_kind` for a non-co-located, non-place entity.
- **The faction-policy and `facility_controller_at` items are hardening, not active
  fixes.** They close footguns and add coverage; they must not change lawful current
  behavior (own-faction policy reads stay available; believed-present-staff
  observation stays available).
- **No CI grep-gate banning `World` in `worldwake-ai`.** The report's enforcement
  proposal is low-benefit here: the AI crate is already proven free of direct world
  reads by the snapshot-through-view invariant test, and the legitimate `world.` reads
  live in `worldwake-sim`'s belief view (which is allowed `World`). Re-add only if a
  future regression demonstrates need.

## Deliverables

1. **`entity_kind` returns stored belief for non-co-located entities**
   (`per_agent_belief_view.rs:604-609`) — keep the `Place` → public-topology branch
   and the FND-14A co-located physical branch (an entity the actor can lawfully
   observe this tick may have its current kind read). For a non-co-located,
   non-place entity, return the **stored `believed_kind`** from the belief store /
   last-seen synthesis, never `world.entity_kind`. A remote entity that changes kind
   with no carrier must keep its last-known kind (or remain unknown).

2. **Last-seen kind synthesis must not read live world**
   (`per_agent_belief_view.rs:1293-1304`) — set `believed_kind` for a last-seen-only
   entity from a belief-local source, never `self.world.entity_kind(*entity)`.
   Ticket-time mechanism choice, with a hard constraint that no live world read is
   introduced:
   - **Preferred:** add an `observed_kind: Option<EntityKind>` field to
     `LastSeenRecord` (`crates/worldwake-core/src/expectation.rs`) populated at
     observation time, and the matching `LastSeenRecordDef` field in
     `crates/worldwake-cli/src/scenario/types.rs`; the synthesis reads that.
   - **Fallback:** synthesize `believed_kind: None` (kind unknown for last-seen-only
     entities). Acceptable but loses kind-at-observation; choose only if the carrier
     field proves disproportionate.
   This is belief/memory carrier state (FND-7/FND-15), not authoritative world state.

3. **Gate the bandit faction-policy accessors to lawfully known factions**
   (`per_agent_belief_view.rs:611-621`) — `bandit_flee_wound_threshold` and
   `bandit_camp_establishment_ticks` must return `None` for a faction the actor does
   not lawfully know (i.e., gate on the same belief/self path `bandit_factions_of`
   already enforces — own/believed faction membership). Current call sites pass
   `bandit_factions_of(actor)`, so lawful behavior is unchanged; the gate removes the
   footgun of a future caller leaking an arbitrary faction's hidden policy.

4. **`facility_controller_at` confirming test** (`per_agent_belief_view.rs:385-401`) —
   add a focused test proving that a **remote** control change (a controller the
   observer does not believe is present, or a control transfer with no carrier) does
   **not** alter the resolved controller/seller identity for a distant actor. If the
   test reveals an actual leak (controller resolved for a non-believed-present agent),
   gate the `can_exercise_control` probe on belief-presence; if it confirms lawfulness,
   the test stands as a regression guard. No behavior change unless the test fails.

5. **Adversarial belief-wall goldens** (`worldwake-ai`) — extend the S162 belief-wall
   golden family with a **remote kind change** scenario: an entity changes kind
   (e.g., agent → corpse) at a remote place with no carrier reaching a distant actor;
   assert the distant actor's `entity_kind` / candidate / affordance set is unchanged
   (keeps the stale kind), and that authoritative truth diverged. Mirror the S162
   assertion discipline: assert the candidate/affordance is *absent or unchanged*
   while authoritative truth changed (FND-31), not merely that the run "looked
   plausible."

## Authoritative-to-AI Impact Analysis

This spec narrows belief-facing accessors (`entity_kind`, the last-seen synthesis)
that feed affordances, candidate generation, ranking, HTN selection, the planning
snapshot, and revalidation, plus two hardening items. The CLAUDE.md checklist
applies and must be walked before the kind changes land:

1. `get_affordances` — candidates that depend on the *current* kind of a remote
   entity must now depend on the stored believed kind (or vanish when kind is
   unknown). That is the intended FND-14B behavior, not a regression.
2. `generate_candidates` — confirm no extractor silently relied on live remote kind;
   trace any goal kind that changes.
3. `search_plan` — terminal ordering/barrier logic unaffected; the snapshot carries a
   belief-sourced kind instead of a live one.
4. `BestEffort` action start — unaffected; authoritative validation unchanged.
5. `handle_plan_failure` — if a candidate vanishes because believed kind is unknown,
   replanning must proceed without thrash; assert via decision trace.
6. **Payload revalidation** — `requested_affordance_matches` /
   `with_payload_override_validator`: confirm no synthesized payload depended on a
   live remote kind read such that affordance-vs-planner payloads diverge.
7. ALL golden tests pass (`cargo test -p worldwake-ai`). Goldens that seeded a remote
   entity's kind for convenience remain valid only if their assertions do not claim
   omniscient kind knowledge; update any that silently relied on the live read.

## FND-01 Section H Analysis

Correctness fix; no new authoritative state, system, component, action, or feedback
loop. Per `docs/spec-drafting-rules.md`, mandatory headers with applicability:

- **Information-path analysis:** Core to this spec, expressed as *removal* of an
  unlawful path. After the fix, the kind of a remote entity reaches a distant actor
  only through perception (co-location), the last-seen record's observed kind, or
  later testimony/record — the lawful carriers (FND-7, FND-15). No new information
  path is introduced; the same-tick omniscient-kind path is closed. Deliverable 3
  removes a latent path; Deliverable 4 verifies an existing one.
- **Positive-feedback analysis:** Not applicable. No amplifying loop introduced or
  removed.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** `PerAgentBeliefView` is a derived
  read-model over `World` + belief stores; this spec narrows what it derives for
  kind. If Deliverable 2 takes the preferred mechanism, `LastSeenRecord` gains an
  `observed_kind: Option<EntityKind>` field — this is **belief/memory carrier state**
  (FND-7/FND-15), recorded at observation time, never promoted to authoritative
  world truth. The planning snapshot remains a transient derived read-model (FND-27),
  still lawful by construction via the snapshot-through-view invariant.
- **Planner-formalism analysis:** Not applicable; no planner-formalism change. Plain
  GOAP/affordance search and the StageHint HTN layer simply receive a belief-sourced
  kind instead of a live one. No goal becomes method-required.

### Belief-View Accessor Source-Class Declarations (per spec-drafting-rules)

| Accessor | Source class after fix | Stale/unknown behavior |
| --- | --- | --- |
| `entity_kind` (place) | Public topology | `Place` (always lawful) |
| `entity_kind` (co-located non-place) | Same-tick local physical (FND-14A) | live kind lawful only when co-located |
| `entity_kind` (remote known) | Belief-backed (stored `believed_kind`) | last-known kind, or `None` if never stored |
| last-seen `believed_kind` synthesis | Belief/memory carrier (`LastSeenRecord` observed kind) | `None` when no observed kind recorded; never live |
| `bandit_flee_wound_threshold` / `bandit_camp_establishment_ticks` | Self/belief-backed faction membership (own/believed factions) | `None` for a faction the actor does not lawfully know |
| `facility_controller_at` | Same-tick local physical (believed-present staff) + dispatch control check among them | controller unchanged for remote/non-believed-present agents |

### Systemic-Validation Analysis (FND-31)

Cross-system feature (belief boundary → planner → CLI affordance path). Negative
illegal-path case the feature must **not** produce: a distant actor (or, via the
shared affordance path, the human player) gaining a candidate, affordance, ranking
change, or HTN method selection from a **remote entity-kind change** for which no
carrier updated belief — and, for the footgun items, leaking an arbitrary faction's
hidden behavioral policy or a remote facility controller's identity. Feature-scoped
checks: the remote-kind-change belief-wall golden (Deliverable 5), the
`facility_controller_at` confirming test (Deliverable 4), and focused unit tests per
accessor (remote vs. co-located vs. self) for `entity_kind`, the last-seen synthesis,
and the gated bandit accessors. Builds on the S162 belief-wall golden family and the
snapshot-through-view invariant.
