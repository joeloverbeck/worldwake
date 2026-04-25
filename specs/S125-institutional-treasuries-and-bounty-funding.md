# S125: Institutional Treasuries and Bounty Funding

## Summary

Give offices a first-class institutional asset model and a mandatory reward-reservation contract so social artifacts such as bounties can be funded without using incidental loose items, personal-funds shortcuts, or hidden manager state. This spec is motivated by the failed `survival-justice` bounty-posting landing: the current runtime can validate `RewardSource::InstitutionalTreasury`, but the authored scenario and AI proof surface cannot introduce institutional reward funds without either perturbing local perception in the theft scene or falling back to an architecturally weaker personal-funds bounty.

The clean target is a concrete treasury model that remains ordinary world state: an office's funds live as conserved item lots inside an office-owned treasury container so they are scoped out of incidental scene perception; the office holder spends them through explicit rights; bounty posting reserves the promised reward against those lots; and `PostBounty` candidate generation reads fund availability through a belief-view accessor, never through omniscient world reads.

This spec is scoped to **offices**. Faction treasuries follow in a future spec once factions have a scenario spawn surface (none exists today); see Non-Goals.

## Phase

Phase 7 adjunct: Consequence Carriers / Institutions

## Status

Draft

## Crates

- `worldwake-core` (treasury container ownership wiring, scenario-authorable office treasury fields, encumbrance/reservation state component, schema registration)
- `worldwake-sim` (`GoalBeliefView` / `RuntimeBeliefView` accessor for institutional reward-source availability)
- `worldwake-systems` (funding authorization helper, reward reservation/release on `post_bounty` / claim / expiry / withdrawal, validation extension)
- `worldwake-ai` (funding-aware `PostBounty` candidate generation and ranking)
- `worldwake-cli` (scenario authoring of office treasuries without local-scene clutter)

## Dependencies

- E17 crime/theft/justice stack — completed ([archive/specs/E17-crime-theft-justice.md](../archive/specs/E17-crime-theft-justice.md))
- S45 social artifacts and bounty lifecycle — completed ([archive/specs/S45-unified-social-artifact-model.md](../archive/specs/S45-unified-social-artifact-model.md))
- S51 autonomous artifact issuance — completed ([archive/specs/S51-artifact-issuance-goals.md](../archive/specs/S51-artifact-issuance-goals.md))
- S59 expectation/obligation substrate — completed ([archive/specs/S59-expectation-obligation-substrate.md](../archive/specs/S59-expectation-obligation-substrate.md))
- S63 contested evidence and warrants — optional downstream consumer, not a prerequisite ([specs/S63-contested-evidence-warrants.md](S63-contested-evidence-warrants.md), Draft)

## Evidence From Reassessment

The `survival-justice` roadmap extension currently asks for non-zero `bounty_posting_weight`, selected `PostBounty`, committed `post_bounty`, and authoritative bounty artifact materialization after accusation/fine.

Live branch facts:

1. `GoalKind::PostBounty` already carries concrete `BountyTerms` (`crates/worldwake-core/src/social_artifact.rs:64-71`): `target`, `proof_requirement`, `reward_commodity`, `reward_quantity`, `reward_source`, `claim_place`.
2. `crates/worldwake-ai/src/candidate_generation.rs::emit_bounty_posting_candidates()` (lines 765-878) emits institutional bounty candidates from consulted accusation records, office-holder belief, jurisdictional rights, and `bounty_posting_weight`. It does not pre-check fund availability before emitting.
3. The emitted reward source is currently hard-coded to `RewardSource::InstitutionalTreasury { treasury_entity: office }` (lines 867-868).
4. `crates/worldwake-systems/src/artifact_actions.rs::validate_reward_source()` (lines 353-418) lawfully requires the office treasury entity to control enough reward commodity by calling `world.controlled_commodity_quantity(treasury_entity, payload.reward_commodity)` (line 367). Validation runs at start/commit only; no reservation is recorded.
5. Scenario authoring currently supports item lots placed at places or agents (`crates/worldwake-cli/src/scenario/types.rs:454-461` / `crates/worldwake-cli/src/scenario/mod.rs::spawn_item`). It does not provide a stable authored surface for office-owned institutional funds that are available to `controlled_commodity_quantity(office, Coin)` without also becoming an incidental local item in the justice scene.
6. Attempting to add office-owned or office-possessed reward coin directly to `survival-justice` changed the local observation/perception environment enough that the theft investigation recorded `SuspectedTheft { suspect: None }` (`crates/worldwake-core/src/violation.rs:39-42`), preventing accusation/fine and therefore preventing a truthful bounty proof.
7. A local personal-funds fallback would make the golden easier to pass, but it changes the row-owned mechanic from institutional bounty funding to a private bounty and avoids the missing office asset contract.
8. `with_payload_override_validator(validate_post_bounty_payload_override)` is already wired at `crates/worldwake-systems/src/artifact_actions.rs:39`; payload revalidation infrastructure exists and only needs to grow with the new reservation contract.
9. `EntityKind::Container` and `EntityKind::Office` both exist (`crates/worldwake-core/src/entity.rs:8-19`); `OwnedBy` and `PossessedBy` relations exist with `controlled_item_lots_for` and `has_control` already supporting office→lot ownership chains.
10. No institutional asset / treasury balance / encumbrance component exists today.

## Design Goals

- Offices can own spendable assets without those funds being modeled as distracting loose market-floor objects co-located with unrelated scene perception.
- Bounty posting reserves the promised reward against actual office-owned item lots; reservations release on claim, expiry, or withdrawal.
- Scenario authors can configure an office's treasury directly and readably as part of the office definition.
- AI candidate generation reads a belief-view accessor to determine whether a lawful reward source exists; it never reads world treasury state directly.
- The `survival-justice` bounty extension proves an institutional bounty branch under the existing justice/search survival envelope without perturbing the theft-scene perception that the existing branches depend on.

## Non-Goals

- Full taxation, payroll, budget politics, or debt/rationing policy — broader economic/institutional work.
- A generic bank account abstraction for every agent. This spec is about office-held institutional assets.
- Replacing existing item-lot conservation. Treasury balances are conserved item lots inside an office-owned container; no new conservation domain is introduced.
- **Faction treasuries**. Factions have no scenario spawn surface today (no `factions` section in `ScenarioDef`, no `spawn_faction`). Faction-fund symmetry is deferred to a future spec that lands faction authoring; the architecture chosen here (office-owned container holding conserved lots) extends symmetrically when factions arrive.
- Reworking the entire justice row. The existing accusation, fine, search, and report branches remain the prerequisite substrate.

## FOUNDATIONS Alignment

| Principle | Alignment |
|---|---|
| FND-3 Concrete State | Funding is conserved item lots, not an abstract balance. No derived score is promoted to truth. |
| FND-4 Persistent Identity / Transfer | Reward funds are conserved lots; reservation, release, claim, and expiry are explicit state transitions. |
| FND-7 Locality | Agents learn that funds are available through office-holder role knowledge (succession record) plus same-tick observation of co-located treasury contents (FND-14A); the AI crate reads this through a belief-view accessor, never through omniscient world queries. |
| FND-8 Preconditions / Cost | Posting a bounty has a concrete reward source and fails when the office cannot reserve enough commodity. |
| FND-14 World State ≠ Belief State | Candidate generation consults the belief view; world reads happen only at authoritative validation. |
| FND-14A Same-Tick Local Observation | Office holder co-located with the treasury container observes contained lot quantities directly; the *right to spend* remains an explicit belief grounded in succession/appointment records. |
| FND-18 Records Are World State | Reservation/encumbrance is a real record; bounty artifacts and their reserved-funds linkage are inspectable. |
| FND-23 Institutions Are World State | Treasuries are tied to offices, holders, jurisdiction, and assets — not a singleton service. |
| FND-24 Ownership / Custody / Access / Jurisdiction | Office *owns* the treasury container and its lots; office *holder* has *access rights* through the office; physical *custody* is the container's location; *jurisdiction* limits enforcement scope. All four remain distinct. |
| FND-26 Systems Interact Through State | Funding authorization is a domain helper over authoritative state; `post_bounty`, AI candidate generation, and future taxation/rationing read shared state and produce records, not direct cross-system calls. |
| FND-28 No Backward Compatibility | The treasury container is the single canonical institutional funding surface; no alias around arbitrary office-owned loose items. |
| FND-30 Causal Hooks | Section H below declares information path, source/sink, contention/reservation, lifecycle, and dampeners. |

## Section H: Required Analyses

### Information-path analysis

- **Office holder → own treasury balance.** The office holder is co-located with the office seat during bounty posting. The treasury container is owned by the office and located at the seat. Contained lots are co-located with the holder, so per FND-14A the holder may read lot commodity/quantity from authoritative state without a belief detour. The *right to spend* the funds is **not** belief-equivalent at co-location (FND-14A explicitly excludes ownership/effective-rights from the same-tick exception); it must be an explicit belief entry sourced from the holder's succession/appointment record.
- **Office holder → non-co-located treasury.** When the holder is not at the seat, balance must come from a stale belief (last-known balance memory or a periodically refreshed treasury record); the spec scopes the proof case to the co-located posting flow and leaves stale-balance memory mechanics to a follow-up.
- **Non-holder → treasury knowledge.** Other agents may know the treasury exists only through observable carriers: a posted notice, a record consulted at the office, witness testimony, or direct co-located observation while at the seat. They cannot plan against the balance without one of those carriers.
- **AI crate read path.** `emit_bounty_posting_candidates` calls a new `GoalBeliefView` accessor (e.g., `actor_lawful_reward_source_for_case`) that returns `Option<RewardSource>`. The accessor consults the actor's role/holder belief and co-located observation through the existing belief view; it does **not** read world state directly. Authoritative validation at action start/commit re-reads world state via the existing `controlled_commodity_quantity` helper.

### Positive-feedback analysis

Bounties create amplifying loops: more accusations → more bounties → more enforcement and violence → more accusations. Successful claims deplete or redirect institutional funds. If bounty posting is cheap and unconstrained, an office could saturate the world with obligations.

### Concrete dampeners

- **Finite treasury lots** — the office's actual coin lots are conserved; spending or reserving them depletes the available pool.
- **Reservation overlap** — reserved lots are not available to back another bounty until released, so concurrent posting is bounded by uncommitted balance.
- **Office-holder time and survival needs** — posting takes action time and competes with eating, sleeping, patrolling, etc.
- **Artifact TTL** — `ArtifactPostingProfile.bounty_ttl` (default 144 ticks, `crates/worldwake-core/src/social_artifact.rs:18-38`) ages bounties out, releasing the reservation back to the treasury.
- **Obligation satiation** — already applied to posting goals upstream.
- **Jurisdiction and proof requirements** — bounded by `S45`/E17 institutional rules.
- **Claim contention** — multiple claimants race for the same reward; only the winning claim transfers funds.

No invisible cap on bounty count is acceptable as the primary dampener.

### Stored state vs. derived read models

**Authoritative stored state**

- Treasury container entity (`EntityKind::Container`) at the office's seat place.
- `OwnedBy(treasury_container, office)` relation.
- Item lots inside the treasury container, each `OwnedBy(lot, office)` so existing `controlled_item_lots_for(office)` continues to surface them.
- New `RewardEncumbrance` (or equivalent) component recording per-bounty reservation: which bounty artifact, which commodity, how much, against which office.
- Optional treasury record/notice for FND-18 propagation to non-holders (out of scope for this spec; may land in S63 follow-on).

**Derived read models (caches over the above)**

- "funds available for a new bounty of size X by office O" — recomputable from controlled lots minus active encumbrances.
- "actor can spend this office's funds" — recomputable from the actor's office-holder belief plus jurisdiction.
- "expected reward source for candidate generation" — recomputable from the above.

Derived reads must remain recomputable from stored lots, ownership relations, encumbrance records, and the actor's belief view.

### Reservation lifecycle (FND-30 §12)

- `Active` — encumbrance recorded at `commit_post_bounty`; reduces the office's available balance for further postings.
- `Released` — reached when `withdraw_bounty` runs, when the bounty's TTL expires, or when claim resolution selects a different reward source.
- `Claimed` — reached when `claim_bounty` succeeds; the reserved lot is transferred from the office to the claimant in the same authoritative transaction.

Transitions emit appended event records; none are silent. Visibility of the encumbrance is scoped to the office and to anyone who can inspect the bounty artifact's record.

### Conservation integration

Treasury funds are item lots; existing `verify_authoritative_conservation` and `verify_live_lot_conservation` (`crates/worldwake-core/src/conservation.rs:20-48`) cover them without modification. `RewardEncumbrance` records are not conserved quantities; they are claims against existing conserved lots, equivalent in semantics to the existing `SaleListing` pattern (`crates/worldwake-core/src/trade.rs:25-29`).

## Proposed Architecture

### 1. Institutional Asset Carrier — Treasury Container

The canonical funding surface is a `Container` entity owned by an office, located at the office's seat place, holding conserved item lots that are themselves owned by the office. The container scopes its contents out of place-floor perception so unrelated scene investigation (e.g., the theft case at the seat) is not perturbed by the office's funds.

- Treasury container entity: `EntityKind::Container`.
- `OwnedBy(treasury_container, office)`.
- Item lots inside the container: `OwnedBy(lot, office)` so `controlled_item_lots_for(office)` continues to enumerate them via the existing `has_control` chain.
- Container contents are not co-located with the container's place for the purpose of place-floor perception; they are co-located with the container itself. (If the existing perception model already enforces this nested-locality semantic, this is a doc-only declaration in this spec; if not, the implementation adds the scoping rule as a small extension. Ticket decomposition will determine which.)

Considered alternatives, rejected:

- *Component-with-balance shape.* A first-class `InstitutionalTreasury` component on the office holding raw commodity balances. Rejected because it introduces a new conserved domain that `verify_authoritative_conservation` would have to extend, and because it abandons FND-3's preference for concrete items over abstract scores.
- *Ledger-backed shape with mint/source/sink events.* Rejected for this spec — even more invasive, and not motivated by the survival-justice proof case. May become useful for later fiscal/taxation work; not blocked by this design.

### 2. Funding Authorization

A shared authorization helper (in `worldwake-systems`) gates institutional fund spending for `post_bounty` and any future rationing/taxation work:

- The actor must hold the office whose treasury they propose to spend.
- The proposed expenditure must fall within the office's jurisdiction/policy.
- Personal funds remain legal for private bounties but are not the institutional row proof.
- All authorization is revalidated authoritatively at action start/commit.

The helper consumes `World` and reads ownership/role/jurisdiction state. It is *not* called from candidate generation — candidate generation goes through the belief view (D6).

### 3. Reward Reservation (Mandatory)

`commit_post_bounty` creates a `RewardEncumbrance` record on the office naming the bounty artifact, the reward commodity, and the reward quantity. The encumbrance reduces the office's available balance for any subsequent posting.

Release paths:

- `claim_bounty` resolution transfers the reserved lot from office to claimant in the same authoritative transaction; the encumbrance is consumed.
- Bounty TTL expiry (driven by the existing artifact-lifecycle path that ages out bounty artifacts) releases the encumbrance.
- `withdraw_bounty` releases the encumbrance.

If multiple postings race in the same tick, ordering is resolved by the existing scheduler tie-break rules; the second posting's `validate_reward_source` sees the first encumbrance and fails authoritatively.

### 4. Scenario Authoring

Extend `OfficeDef` (`crates/worldwake-cli/src/scenario/types.rs`) with an optional `treasury` field referencing a treasury container scaffold (e.g., `treasury: Option<TreasuryDef>` carrying initial commodity/quantity entries). `spawn_office` (`crates/worldwake-cli/src/scenario/mod.rs`) materializes the container, sets `OwnedBy(container, office)`, and authors the initial item lots inside the container with `OwnedBy(lot, office)`.

Existing `ItemDef` placement (which today resolves only place- and agent-keyed `location` strings) is **not** required to grow new placement targets; the treasury content is authored through the office-scoped `TreasuryDef`, not through global `items:` entries. This keeps the institutional surface readable and avoids accidental loose-item authoring.

Linting (extending `crates/worldwake-cli/src/scenario/lints.rs`):

- Reject treasuries authored on offices whose seat does not exist.
- Reject treasury commodities or quantities that are zero/invalid.
- Optionally warn when a scenario authors `bounty_posting_weight > 0` for an office holder whose office has no treasury (no funded path to a bounty).

### 5. AI Candidate Generation

`emit_bounty_posting_candidates` queries a new `GoalBeliefView` accessor (D6) that returns `Option<RewardSource>` for the accusation case. The accessor consults the actor's office-holder belief, co-located observation of treasury container contents, and any active encumbrances visible to the actor; it returns `None` when no lawful funded source exists.

The emitter must not hard-code `RewardSource::InstitutionalTreasury { treasury_entity: office }`. It must use the accessor's return value or skip emission if no lawful source exists. It must not silently fall back to `PersonalFunds` for a roadmap-owned institutional bounty unless the scenario or policy explicitly classifies the agent's bounty as a private bounty.

### 6. Golden Landing

After the substrate lands, `survival-justice` retains its existing branches and adds the bounty extension as a new test `survival_justice_proves_institutional_bounty_posted` (in `crates/worldwake-ai/tests/golden_survival_justice.rs`):

- accusation/fine still occur (existing branches unchanged)
- `PostBounty` ranks/selects after the local crime case exists
- `post_bounty` commits with an institutional reward source backed by the office's treasury
- active bounty artifact materializes with `RewardSource::InstitutionalTreasury { treasury_entity: <office> }`
- a `RewardEncumbrance` record exists against the office for the bounty's commodity/quantity
- survival-health contract still passes for the owning agent
- the existing three goldens (`survival_justice_proves_accusation_substrate`, `survival_justice_proves_fine_punishment_for_same_theft_case`, `survival_justice_proves_search_and_report_found`) continue to pass without modification

## SystemFn Integration

No new periodic SystemFn is required. Reservation release rides on existing pathways:

- The artifact-lifecycle path that expires bounty TTL also releases their encumbrances.
- `claim_bounty` and `withdraw_bounty` action handlers release/consume encumbrances inline at commit.

If implementation discovers that the existing artifact-lifecycle path does not yet invoke a release hook, ticket decomposition will add a small extension (action-handler integration, not a new SystemFn) and surface it as part of D5/D7.

## Cross-System Interactions

- **`worldwake-systems::artifact_actions` ↔ `worldwake-core` ownership**: validation reads `controlled_commodity_quantity` and writes `RewardEncumbrance` records; commit/expire/claim transitions emit appended events. No direct cross-system calls beyond the shared state.
- **`worldwake-ai::candidate_generation` ↔ `worldwake-sim::belief_view`**: candidate generation reads the new accessor; it does not import or call `worldwake-systems` validators.
- **`worldwake-cli::scenario` ↔ `worldwake-core`**: scenario load materializes treasury containers and lots through standard component/relation insertion; no new authoritative paths.

## Component Registration

`RewardEncumbrance` is a new ECS component. It must be registered through `crates/worldwake-core/src/component_schema.rs` via `forward_authoritative_components!` and propagated to `delta.rs`, `world.rs`, and `component_tables.rs`. The kind-check restricts attachment to `EntityKind::Office` (later extensible to `EntityKind::Faction` when factions land).

This spec does not add an `EntityKind::Agent` behavior profile, so `docs/spec-drafting-rules.md` section 5 does not apply.

## Authoritative-to-AI Impact (CLAUDE.md checklist)

The spec extends `validate_reward_source`, `emit_bounty_posting_candidates`, and `post_bounty` action handlers. Per CLAUDE.md the implementation must trace:

1. `get_affordances` — N/A; `PostBounty` is goal-emitted from accusation records.
2. `generate_candidates` — emitter consumes the new belief-view accessor and skips emission when no funded source exists (D4 + D6).
3. `search_plan` — N/A; no new `PlannerOpKind`.
4. `BestEffort` action start — `start_post_bounty` re-runs `validate_reward_source` (which now also checks against active encumbrances).
5. `handle_plan_failure` — replan must back out cleanly when revalidation rejects (e.g., a sibling bounty's encumbrance shrank the available balance between selection and start).
6. Payload revalidation — `with_payload_override_validator(validate_post_bounty_payload_override)` is already wired at `crates/worldwake-systems/src/artifact_actions.rs:39`. Implementation extends the existing validator to enforce the new reservation contract; no new wiring is needed.
7. Golden tests — all four `survival-justice` goldens pass under the ignored `golden-survival` lane.

## Deliverables

1. Treasury container ownership wiring in core: relation conventions for `OwnedBy(container, office)` + lots `OwnedBy(office)` so `controlled_item_lots_for(office)` continues to surface them; component-schema, `delta.rs`, `world.rs`, and `component_tables.rs` registration for the new `RewardEncumbrance` component; conservation reuse confirmed (lots remain conserved; no extension of `verify_*_conservation`).
2. Scenario authoring: extend `OfficeDef` with `treasury: Option<TreasuryDef>`; `spawn_office` materializes the container and seeds initial lots owned by the office; lint rules cover unreachable/zero-quantity/missing-seat cases.
3. Funding authorization helper in `worldwake-systems` (office-holder may spend office-owned lots within jurisdiction/policy), reused by `validate_reward_source` and any future fiscal work. The office-holder extraction landed in `S125INSTREBOU-003`; the remaining jurisdiction/policy enforcement branch is owned by `S125INSTREBOU-008`.
4. `PostBounty` candidate generation: `emit_bounty_posting_candidates` consumes the new belief-view accessor (D6); removes the hard-coded `treasury_entity: office`; skips emission when the accessor returns `None`.
5. `post_bounty` authoritative validation and commit integration: extend `validate_reward_source` to consider active encumbrances; `commit_post_bounty` records `RewardEncumbrance`; `claim_bounty` / `withdraw_bounty` / TTL-expiry paths release/consume the encumbrance; existing `validate_post_bounty_payload_override` covers the payload contract.
6. `GoalBeliefView` accessor (`actor_lawful_reward_source_for_case` or equivalent) in `crates/worldwake-sim/src/belief_view.rs`, with a `RuntimeBeliefView` impl and `impl_goal_belief_view!` macro forwarding so the AI crate reads it through the existing trait surface.
7. Focused tests in `worldwake-systems` for authorization, insufficient funds, reservation creation/release/claim, expiry release, and overlap rejection; focused tests in `worldwake-cli` for scenario spawn (lint and materialization).
8. `survival-justice` golden extension: new test `survival_justice_proves_institutional_bounty_posted` proving the institutional bounty branch under the existing survival envelope; the three existing goldens continue to pass unchanged.
9. Generated golden docs and `survival-justice` scenario roadmap update once the row is actually landed.

## Acceptance Criteria

1. A scenario can author a `Market Warden` treasury with funds without creating an extra place-floor item that changes theft-scene perception.
2. `PostBounty` is not emitted for an accusation case when no lawful reward source exists (the office's treasury is missing, empty, or fully encumbered).
3. `PostBounty` emits with `RewardSource::InstitutionalTreasury { treasury_entity: <office> }` only when the office has accessible unencumbered funds.
4. `post_bounty` fails authoritatively if funds disappear or become fully encumbered before start/commit.
5. Multiple active bounties cannot overpromise the same reserved funds; the second posting fails because the first's encumbrance shrinks the available balance.
6. `survival_justice_proves_accusation_substrate`, `survival_justice_proves_fine_punishment_for_same_theft_case`, `survival_justice_proves_search_and_report_found`, and the new `survival_justice_proves_institutional_bounty_posted` all pass under the ignored `golden-survival` lane.
7. The bounty artifact and its `RewardEncumbrance` record remain inspectable: a debug query can show "this bounty's reward is reserved against this office's treasury" without reading source code.

## Open Questions

1. Does office-held fine revenue feed this treasury in the same spec, or should fine revenue remain existing office-owned property until a follow-up explicitly connects the two? *(Tentative: defer; fine routing is its own information path.)*
2. Should the funding authorization helper from D3 be promoted immediately to a generic `worldwake-systems` API used by future rationing/taxation specs, or kept private to `artifact_actions` until a second consumer appears? *(Tentative: keep private; widen when the second consumer arrives, per FND-26.)*
3. Should non-co-located stale-balance memory for office holders be modeled now (so a holder away from the seat can still post a bounty against a remembered balance) or deferred to a follow-up spec? *(Tentative: defer; the survival-justice proof posts at the seat.)*
