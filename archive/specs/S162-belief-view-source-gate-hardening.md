# S162 — Belief-View Source-Gate Hardening

**Status:** COMPLETED
**Type:** Correctness fix (closes FND-14/FND-14A leaks in `PerAgentBeliefView`
accessors; adds adversarial belief-wall goldens; locks the snapshot-through-view
invariant). No new authoritative simulation state, system, component, action, or
feedback loop.
**Priority:** High. This is the release-blocking belief/world boundary work. No
new AI behavior should be added on top of the belief view until these leaks close.
**Crates:** `worldwake-sim` (`per_agent_belief_view.rs` accessor gates),
`worldwake-ai` (adversarial goldens + snapshot-source invariant test).
**Foundations:** FND-7, FND-14, FND-14A, FND-14B, FND-19, FND-27, FND-31
**Extends:** `S158-belief-view-remote-truth-leak-closure.md`. S158
closed the economic/production/physical/contention leaks "under one source-class
rule" but **explicitly deferred the social/control rights value path** (per
S155/S158 scope). S162 completes that deferred path (`has_control`, `record_data`,
`office_data`, `believed_rights`/`can_control`, `loyalty_to`) **and** the residual
contention accessors S158's wave did not gate (`actor_can_claim_extraction_slot`,
`has_extraction_queues`, `facility_queue_join_tick`, `reservation_conflicts`,
`reservation_ranges`). Like S158, it rejects the `Sourced<T>`/field-source-typing
and trait-split proposals as out-of-scope migration risk.
**Source:** `reports/ai-architecture-consolidation-third-iteration.md` (triage
`docs/triage/2026-05-21-ai-architecture-consolidation-third-iteration-triage.md`).

## Problem Statement

### Motivation

`PerAgentBeliefView` wraps `&World`, and several accessors gate a current
authoritative read on `knows_entity` (or on nothing at all) rather than on the
correct source class. FND-14A is explicit: co-location exposes only *directly
perceivable physical* facts; ownership, rights, control, institutional claims,
records, loyalty, and contention state are **belief-gated even when co-located**.
FND-14B extends this to every planner-visible input. Knowing that an entity
*exists* is not permission to read all of its *current* social/legal/contention
fields.

These accessors feed `get_affordances`, candidate generation, ranking, HTN method
selection, and the planning snapshot. A leak here is a leak into every planner
surface and into the CLI player menu (which shares the affordance path).

### The snapshot is lawful by construction once the view is lawful

`crates/worldwake-ai/src/planning_snapshot.rs` performs **zero direct `world.`
reads** (verified 2026-05-21: `grep -c "world\." planning_snapshot.rs` → `0`).
Every snapshot field is sourced through a `view.*` accessor. Therefore the
snapshot cannot launder a fact the view does not first surface: fixing the view
makes the snapshot lawful **by construction**.

This is why the third-iteration report's headline "Critical — snapshot field
laundering / field-level source typing" proposal is **rejected** (see triage).
FND-14B requires a snapshot to *preserve the source classification or be treated
as illegal* — it does **not** mandate per-field source *types* on ~50 fields. The
correct architectural rule is the much smaller invariant in Deliverable 6: the
snapshot may read only through the belief view. The capability-trait split of
`RuntimeBeliefView` is likewise rejected — every leak below closes by fixing the
accessor body, not by splitting the trait.

### Implementation scope note (verified 2026-05-21)

Although the leaky methods also *appear* in `crates/worldwake-sim/src/belief_view.rs`,
the **only** world-reading implementations live in `per_agent_belief_view.rs` (the
sole edit target). The `belief_view.rs` copies are safe: trait-default impls that
return `None` (`record_data`/`office_data` at `:750/:754` and `:1507/:1511`) or a
forwarding shim (`:2500`). The `impl DebugWorldView for &World` at
`belief_view.rs:961` is the intentionally-omniscient observer/debug surface and
**must not be gated** — it is the lawful authoritative read permitted to debug/test
harnesses by FND-14B (final paragraph) and FND-29.

### Evidence (verified against code on 2026-05-21)

All line numbers are in `crates/worldwake-sim/src/per_agent_belief_view.rs`
unless noted.

**Confirmed unlawful — no gate / authoritative current read:**

- `has_control` (`:461`) reads `AgentData.control_source` for **any** entity with
  **no gate**. `ControlSource` is meta-control (FND-19: "changes only who chooses
  the next action, never what reality allows"), not a character-perceivable fact.
- `record_data` (`:1653`) and `office_data` (`:1659`) return the **current**
  `RecordData` / `OfficeData` component, gated only on
  `entity_kind(..) == Record/Office`, which resolves through `knows_entity`
  (co-location, possession, belief, or last-seen). `RecordData`/`OfficeData` carry
  institutional facts (holder, vacancy, jurisdiction, succession, penalties,
  bounties) that must come from a consulted record or an institutional belief, not
  live truth.
- Contention reads with **no co-location/own-ticket gate** (their siblings *are*
  gated — see below): `actor_can_claim_extraction_slot` (`:1177`),
  `has_extraction_queues` (`:1191`), `facility_queue_join_tick` (`:1220`),
  `reservation_conflicts` (`:1238`), `reservation_ranges` (`:1245`).

**Confirmed unlawful — `knows_entity` used where `believed_entity` is required:**

- `loyalty_to` (`:1695`): self-gated on subject, but target gated on
  `knows_entity` rather than an explicit belief. Loyalty is a social/relational
  fact.
- `stock_storage_policy` (`:2267`): gated on `knows_entity`. Storage policy is an
  institutional/management fact, not a perceivable physical property.

**Confirmed unlawful — social fact read under a co-location/self-relational gate:**

- `believed_rights` (`:428`) and `can_control` (`:441`) compute accessibility with
  `world.possessor_of(entity) == Some(self.agent)` / `world.owner_of(entity) ==
  Some(self.agent)` (`:433-434`, `:453-454`), then call `world.effective_rights` /
  `world.can_exercise_control`. The owner/possessor accessibility probes read
  authoritative relational state; effective rights/control are social facts.
- `can_control` unowned-item branch (`:442-449`): returns `true` for a co-located,
  unowned, item-kind entity using `world.owner_of(entity).is_none()`. Ownership
  *absence* is still an ownership claim (FND-14A), not a physical observation.

**Confirmed already-lawful (no action — the report flagged some of these as
leaks; it was wrong):**

- `merchandise_profile` (`:2191`): gated on `self || believed_entity`. Lawful.
- `visible_reward_encumbrance` (`:1738`): gated on self + believed office-holder.
  Lawful.
- `factions_of` for non-self (`:1567`): reads only `known_institutional_beliefs`.
  Lawful.
- Co-location-gated contention siblings: `extraction_slot_queue_position`
  (`:1147`), `actor_holds_extraction_slot_grant` (`:1161`), `facility_queue_position`
  (`:1129`), `facility_grant` (`:1138`), `contention_queue_is_full` (`:1197`,
  belief-backed when remote), `facility_queue_patience_ticks` (`:1232`, self).
  These are the **template** the no-gate methods must match.

### Key scoping decisions (brainstorm 2026-05-21)

- **Fix the accessors; do not split the trait or add field-level snapshot source
  types.** Both are rejected as churn the leaks don't require (see triage; user
  confirmed). The snapshot-through-view invariant (Deliverable 6) is the
  root-cause guard for the snapshot concern.
- **`record_data`/`office_data` must trace their call sites before the fix lands.**
  If a consulted-record / believed-institutional substrate does not already carry
  the facts a caller needs, the accessor returns the believed snapshot (or `None`)
  and the dependent candidate must originate from institutional belief — it must
  **not** fall back to live truth. Implementers must enumerate consumers (the
  Authoritative-to-AI checklist below) before choosing the believed carrier.
- This spec changes **no authoritative validation**. Authoritative dispatch
  (`can_exercise_control`, reservation/queue arbitration at action start) keeps
  reading `World` — that is lawful at the dispatch boundary (FND-14B final
  paragraph). Only the *belief-facing* accessors change.

## Deliverables

1. **`has_control` (`:461`)** — gate to self / explicit belief only. For `self`,
   self-authoritative. For other entities, control source is meta (FND-19): the
   accessor must not read `AgentData.control_source` from world. Return `false`
   (or a believed-control value if an institutional control belief exists for the
   subject) for non-self entities. The snapshot `SnapshotControl.has_control` field
   inherits the lawful result.

2. **`record_data` (`:1653`) / `office_data` (`:1659`)** — replace the live
   `get_component_record_data` / `get_component_office_data` read with a
   belief-backed source: a consulted-record snapshot or the believed-institutional
   accessors that already exist (`believed_office_holder`,
   `believed_force_controller`, `believed_support_declarations_for_office`,
   `known_institutional_beliefs`). Note these existing accessors cover only office
   *holder*, *force controller*, and *support declarations* — **not** `OfficeData`'s
   jurisdiction/succession/vacancy/reward-policy fields — so a believed `office_data`
   cannot be fully reconstructed from them today. Because `record_data`/`office_data`
   return whole `Option<RecordData>`/`Option<OfficeData>` structs (not field-by-field),
   the accessor returns `None` (the whole `Option`) unless a believed snapshot covers
   the read — it must never read current truth. `../tickets/S162BELVIESOU-006.md`
   landed that carrier as `BelievedOfficeDataSnapshot` /
   `BelievedRecordDataSnapshot`, with `consult_record` as the first lawful
   acquisition path. The dependent candidate then depends on the institutional
   belief or is correctly absent. Trace every call site first (see
   checklist).

3. **No-gate contention reads** — add the same `has_authoritative_local_visibility`
   co-location gate (and/or own-ticket / belief gate) the lawful siblings use:
   - `actor_can_claim_extraction_slot` (`:1177`)
   - `has_extraction_queues` (`:1191`)
   - `facility_queue_join_tick` (`:1220`)
   - `reservation_conflicts` (`:1238`)
   - `reservation_ranges` (`:1245`)
   When the actor is not co-located and holds no own ticket/reservation belief, the
   accessor returns `false` / empty. Reservations are authoritative dispatch facts;
   lawful planner knowledge of them is local observation or the actor's own
   reservation belief only.

4. **`loyalty_to` (`:1695`) / `stock_storage_policy` (`:2267`)** — replace the
   `knows_entity` gate with `believed_entity(..).is_some()` (plus the existing
   self gate on `loyalty_to`'s subject). Social/institutional facts require an
   explicit belief entry, not last-seen/co-location.

5. **`believed_rights` (`:428`) / `can_control` (`:441`)** — remove the
   authoritative owner/possessor accessibility probes (`:433-434`, `:453-454`) in
   favor of self / believed-entity gates; and gate the unowned-item co-location
   branch (`:442-449`) on a belief of unownedness rather than `world.owner_of(..)
   .is_none()`. Authoritative `effective_rights` / `can_exercise_control` calls are
   permitted only behind a lawful (self or belief) accessibility gate. If
   implementers find a co-located physical sub-fact that is genuinely
   FND-14A-perceivable (e.g., "this item lot is physically here and unattended"),
   it must be expressed as a physical observation, not an ownership read.

6. **Snapshot-through-view invariant test** (`worldwake-ai`) — add a guard test
   asserting `planning_snapshot.rs` performs no direct authoritative `world.`
   read (all entity/field data flows through the `RuntimeBeliefView`/`PerAgentBeliefView`
   surface). This locks lawfulness-by-construction and prevents a future field
   from regressing to a direct world read. (Mechanism is a ticket-time detail — a
   source-scan test, a `#![deny]`-style module boundary, or a review checklist
   encoded as a test; it must fail if a `world.` read is reintroduced.)

7. **Adversarial belief-wall goldens** (`worldwake-ai`) — prove that a remote
   authoritative change with **no lawful carrier** changes no affordance,
   candidate, ranking, or HTN method for a distant actor. Minimum scenarios (map
   to FOUNDATIONS canonical regressions I/J and report §17):
   - Remote owner/control change unseen → no new control/rights affordance.
   - Remote office vacancy / record entry change unseen → no claim/support
     candidate, no method selection change, until consult/testimony.
   - Remote extraction slot filled / reservation created unseen → distant actor's
     candidate/ranking unchanged; authoritative start may still fail lawfully.
   - Remote loyalty change unseen → no political/economic candidate shift.
   Each golden must assert the candidate/affordance is *absent* (or unchanged)
   while authoritative truth changed — not merely that the run "looked plausible"
   (FND-31).

## Authoritative-to-AI Impact Analysis

This spec changes belief-facing accessors that feed affordances, candidates,
ranking, HTN selection, and revalidation. The CLAUDE.md checklist applies and must
be walked per accessor before each change lands:

1. `get_affordances` — must still produce correct candidates for *lawfully known*
   entities; candidates depending on a now-gated remote fact must disappear (that
   is the intended FND-14B behavior, not a regression).
2. `generate_candidates` — goal kinds depending on `record_data`/`office_data`/
   contention reads must be traced; confirm they originate from institutional/local
   belief after the fix, or are correctly absent.
3. `search_plan` — terminal ordering/barrier logic unaffected; the snapshot simply
   carries fewer remote facts.
4. `BestEffort` action start — unaffected; authoritative validation is unchanged.
5. `handle_plan_failure` — if a candidate vanishes because its supporting belief is
   absent, replanning must proceed without thrash; assert via decision trace.
6. **Payload revalidation** — `requested_affordance_matches` /
   `with_payload_override_validator`: confirm no synthesized payload depended on a
   now-gated read such that affordance-vs-planner payloads diverge.
7. ALL golden tests pass (`cargo test -p worldwake-ai`). Existing goldens that seed
   broad world beliefs for convenience remain valid only if their assertions do not
   claim ignorance/stale behavior (report §13); update any that silently relied on
   a leak.

## FND-01 Section H Analysis

Correctness fix; no new authoritative state, system, component, action, or feedback
loop. Per `docs/spec-drafting-rules.md`, mandatory headers included with
applicability noted:

- **Information-path analysis:** Core to this spec but expressed as *removal* of
  unlawful paths. After the fix, social/legal/contention facts reach a distant
  actor only through perception (co-location), testimony, record consultation, or
  institutional belief — the lawful carriers (FND-7, FND-15). No new information
  path is introduced; illegitimate same-tick omniscient paths are closed.
- **Positive-feedback analysis:** Not applicable. No amplifying loop introduced or
  removed.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** No new authoritative state.
  `PerAgentBeliefView` is a derived read-model over `World` + belief stores; this
  spec narrows what it derives. The planning snapshot remains a transient derived
  read-model (FND-27); Deliverable 6 makes its lawful-by-construction property
  explicit. No derived value is promoted to truth.
- **Planner-formalism analysis:** No planner formalism change. Plain
  GOAP/affordance search and the existing StageHint HTN layer are unchanged; they
  simply receive lawfully-sourced inputs. No goal becomes method-required.

### Belief-View Accessor Source-Class Declarations (per spec-drafting-rules)

| Accessor | Source class after fix | Stale/unknown behavior |
| --- | --- | --- |
| `has_control` | Self (self-authoritative); else belief-backed institutional control, if any | `false` for non-self without belief |
| `record_data` | Belief-backed (consulted-record snapshot / institutional belief) | `None` per absent field; never current truth |
| `office_data` | Belief-backed (institutional belief / consulted record) | `None` per absent field; never current truth |
| `actor_can_claim_extraction_slot` | Same-tick local physical (co-located) or own-ticket belief | `false` when remote without own ticket |
| `has_extraction_queues` | Same-tick local physical (co-located) | `false` when remote |
| `facility_queue_join_tick` | Same-tick local physical (co-located) or own-ticket | `None` when remote |
| `reservation_conflicts` / `reservation_ranges` | Same-tick local physical (co-located) or own reservation belief | `false` / empty when remote |
| `loyalty_to` | Self (subject) + belief-backed (target) | `None` without explicit target belief |
| `stock_storage_policy` | Belief-backed | `None` without explicit facility belief |
| `believed_rights` / `can_control` | Self or belief-backed accessibility; physical-only co-located reads via FND-14A | empty / `false` without lawful source |

### Systemic-Validation Analysis (FND-31)

Cross-system feature (belief boundary → planner → CLI). Negative illegal-path cases
the feature must **not** produce: a distant actor gaining a candidate, affordance,
ranking change, or HTN method selection from a remote ownership, control, record,
office, loyalty, queue, or reservation change for which no carrier updated the
actor's belief. Feature-scoped checks: the adversarial belief-wall goldens
(Deliverable 7), the snapshot-through-view invariant test (Deliverable 6), focused
unit tests per accessor (remote vs. co-located vs. self), and a causal/decision
trace audit showing the candidate's source belief. Existing `worldwake-ai` goldens
serve as the regression/composition suite.

## Resolved Implementation Questions

- `../tickets/S162BELVIESOU-006.md` landed the lawful whole-record/office substrate as
  `BelievedRecordDataSnapshot` / `BelievedOfficeDataSnapshot`, with
  `consult_record` as the first lawful acquisition path.
- `../tickets/S162BELVIESOU-004.md` landed the snapshot-through-view
  invariant as a compile-time source guard plus focused tests.
- `../tickets/S162BELVIESOU-005.md` completed the package-level golden
  proof surface by reusing the existing `belief_wall_trap` matrix, repairing the
  lawful office snapshot carriers, and truth-syncing the stale remote-lot and
  obligation-satiation goldens.

## Outcome

Completed on 2026-05-21.

- Closed the S162 belief-view source gates through the archived ticket family:
  contention co-location gates, control/rights gates, institutional/social gates,
  lawful record/office snapshot carriers, snapshot-through-view guard coverage,
  and package-level adversarial golden proof.
- Deviated from the draft's assumption that a new adversarial golden module was
  required: live reassessment found the active `belief_wall_trap` matrix already
  covered the D7 proof surface, so the final ticket repaired and truth-synced
  existing goldens instead of duplicating them.
- Verified the final package seam with `cargo test -p worldwake-ai`; focused
  supporting proof included `scenarios::belief_wall_trap`, `scenarios::offices`,
  `golden_consume_pipeline_records_start_failure_after_remote_lot_change`,
  `obligation_satiation_allows_survival_needs_to_override_posting`, and
  `python3 scripts/golden_inventory.py --write --check-docs`.
