# S125: Institutional Treasuries and Bounty Funding

## Summary

Give offices and factions a first-class institutional asset and budget contract so social artifacts such as bounties can be funded without using incidental loose items, personal-funds shortcuts, or hidden manager state. This spec is motivated by the failed `survival-justice` bounty-posting landing: the current runtime can validate `RewardSource::InstitutionalTreasury`, but the authored scenario and AI proof surface cannot introduce institutional reward funds without either perturbing local perception in the theft scene or falling back to an architecturally weaker personal-funds bounty.

The clean target is a concrete treasury/accounting model that remains ordinary world state: assets have identity or aggregate backing, offices/factions control them through explicit rights, agents learn about usable funding through records or local office knowledge, and `PostBounty` chooses a lawful reward source from that state.

## Phase

Phase 7 adjunct: Consequence Carriers / Institutions

## Status

Draft

## Crates

- `worldwake-core` (institutional asset state, budget policy, ledger claims)
- `worldwake-systems` (funding validation, reward reservation/release, bounty posting integration)
- `worldwake-ai` (funding-aware `PostBounty` candidate generation and ranking)
- `worldwake-cli` (scenario authoring for institutional assets without incidental local-scene clutter)

## Dependencies

- E17 crime/theft/justice stack - completed
- S45 social artifacts and bounty lifecycle - completed
- S51 autonomous artifact issuance - completed
- S59 expectation/obligation substrate - completed
- S63 contested evidence and warrants - optional downstream consumer, not a prerequisite

## Evidence From Reassessment

The `survival-justice` roadmap extension currently asks for non-zero `bounty_posting_weight`, selected `PostBounty`, committed `post_bounty`, and authoritative bounty artifact materialization after accusation/fine.

Live branch facts:

1. `GoalKind::PostBounty` already carries concrete `BountyTerms`, including `reward_source`, `reward_commodity`, `reward_quantity`, proof requirement, and claim place.
2. `crates/worldwake-ai/src/candidate_generation.rs::emit_bounty_posting_candidates()` emits institutional bounty candidates from consulted accusation records, office-holder belief, jurisdictional rights, and `bounty_posting_weight`.
3. The emitted reward source is currently `RewardSource::InstitutionalTreasury { treasury_entity: office }`.
4. `crates/worldwake-systems/src/artifact_actions.rs::validate_reward_source()` lawfully requires the office/faction treasury entity to control enough reward commodity.
5. Scenario authoring currently supports item lots placed at places or agents. It does not provide a stable authored surface for office-owned institutional funds that are available to `controlled_commodity_quantity(office, Coin)` without also becoming an incidental local item in the justice scene.
6. Attempting to add office-owned or office-possessed reward coin directly to `survival-justice` changed the local observation/perception environment enough that the theft investigation recorded `SuspectedTheft { suspect: None }`, preventing accusation/fine and therefore preventing a truthful bounty proof.
7. A local personal-funds fallback would make the golden easier to pass, but it changes the row-owned mechanic from institutional bounty funding to a private bounty and avoids the missing office asset contract.

## Design Goals

- Offices and factions can own or control spendable assets without those funds being modeled as distracting loose market-floor objects.
- Bounty posting can reserve, encumber, or otherwise account for reward funds without creating coin from nowhere.
- Scenario authors can configure institutional funds directly and readably.
- AI candidate generation can determine whether a lawful bounty reward source exists without omniscient access to unrelated global state.
- The `survival-justice` bounty extension can prove an institutional bounty branch under the existing justice/search survival envelope.

## Non-Goals

- Full taxation, payroll, budget politics, or debt/rationing policy. Those remain broader economic/institutional work.
- A generic bank account abstraction for every agent. This spec is about institutional assets held by offices/factions.
- Replacing existing item-lot conservation. Treasury balances must still map to conserved commodity state or explicit ledger-backed assets.
- Reworking the entire justice row. The existing accusation, fine, search, and report branches remain the prerequisite substrate.

## FOUNDATIONS Alignment

| Principle | Alignment |
|---|---|
| FND-3 Concrete State | Funding must be explicit state, not a planner default or hidden boolean. |
| FND-4 Persistent Identity / Transfer | Reward funds are conserved and transferred, reserved, or released through explicit state transitions. |
| FND-7 Locality | Agents learn that funds are available through office/faction role knowledge, records, or local access, not global queries. |
| FND-8 Preconditions / Cost | Posting a bounty has a concrete reward source and can fail when funds are unavailable or inaccessible. |
| FND-18 Records Are World State | Budget or fund availability can be represented by ledgers/records where agents need to inspect it. |
| FND-23 Institutions Are World State | Treasuries are tied to offices/factions, holders, jurisdiction, and assets. |
| FND-24 Ownership / Custody / Access / Jurisdiction | The model must distinguish institution-owned funds, holder access, physical custody, and jurisdictional authority. |

## Section H: Required Analyses

### Information-path analysis

The fund itself is authoritative world state owned by an office or faction. An office holder may act on funds they can lawfully access through their role. Non-holders may only know funds exist through a local record, artifact, testimony, or observation of the physical/ledger carrier. `PostBounty` candidate generation must therefore use the actor's belief/role view to establish authority and funding, then authoritative action validation rechecks the actual funds at start/commit.

### Positive-feedback analysis

Bounties can create an amplifying loop: more accusations create more bounties; more bounties attract more violence or enforcement; successful claims deplete or redirect institutional funds. If bounty posting is cheap and unconstrained, offices could saturate the world with obligations.

### Concrete dampeners

- finite treasury funds or reserved reward lots
- office-holder time and survival needs
- artifact TTL from `ArtifactPostingProfile`
- obligation satiation already applied to posting goals
- jurisdiction limits and proof requirements
- claim contention over bounty rewards

No invisible cap on bounty count is acceptable as the primary dampener.

### Stored state vs. derived read models

Stored state:

- institutional asset carrier or ledger component
- asset owner/controller relation to office/faction
- reservation/encumbrance state for posted rewards
- bounty artifact header and `BountyTerms`
- records that expose fund availability when needed

Derived read models:

- "funds available for this bounty"
- "actor can spend this office's funds"
- "expected reward source for candidate generation"
- "budget pressure" summaries, if introduced later

Derived reads must be recomputable from stored assets, office/faction relations, and records.

## Proposed Architecture

### 1. Institutional Asset Carrier

Introduce one canonical way for an institution to hold commodity funds without requiring the funds to be a loose local item competing with scene perception.

Acceptable implementation shapes:

1. A first-class `InstitutionalTreasury` component on office/faction entities that owns explicit commodity balances and emits deltas on transfer/reservation.
2. A treasury container/site entity associated with an office/faction, with item lots stored inside and a scenario-authored reference to the owning institution.
3. A ledger-backed institutional asset component that stores balances but still participates in conservation through explicit mint/source/sink or transfer records.

The selected design must not be a backward-compatible alias around arbitrary office-owned loose items. It should become the single canonical institutional funding surface.

### 2. Funding Authorization

Add a shared authorization helper used by `post_bounty`, future rationing/taxation work, and AI candidate generation:

- office holder may spend office funds within office jurisdiction/policy
- faction member or delegated officer may spend faction funds only if policy allows
- personal funds remain legal for private bounties but are not the default institutional row proof
- all authorization is revalidated authoritatively at action start/commit

### 3. Reward Reservation

Posting a bounty should either reserve the promised reward or record an explicit encumbrance against institutional funds. Claiming a bounty then transfers from the reserved/encumbered source. If reservation is not implemented in the first ticket, the spec must require an immediate follow-up; otherwise multiple active bounties can overpromise the same funds.

### 4. Scenario Authoring

Add scenario support for institutional funds, likely under `offices` or a new top-level `institutional_assets` section. It must allow:

- office/faction name references instead of raw IDs
- commodity and quantity
- optional physical storage place/container only when the scenario wants visible custody
- linting for funds that are unreachable, ownerless, or inconsistent with the referenced institution

### 5. AI Candidate Generation

`emit_bounty_posting_candidates()` should ask the belief/view layer whether the actor has a lawful reward source for the accusation case. It must not hard-code `InstitutionalTreasury { treasury_entity: office }` unless that office has an actual funding carrier. It must not silently fall back to `PersonalFunds` for a roadmap-owned institutional bounty unless the scenario or policy explicitly makes it a private bounty.

### 6. Golden Landing

After the substrate lands, `survival-justice` should retain its existing branches and add the bounty extension:

- accusation/fine still occur
- `PostBounty` ranks/selects after the local crime case exists
- `post_bounty` commits
- active bounty artifact materializes with institutional reward source
- reward funds are reserved or otherwise accounted
- survival-health contract still passes for the owning agent

## SystemFn Integration

If reservation/encumbrance has ticking behavior, add a system function for expiration/release when bounties expire or are withdrawn. If reservation is purely action-time state, no new periodic SystemFn is required, but `artifact_actions` must release reserved rewards on expiry/claim/withdrawal paths.

## Component Registration

Any new ECS component must be registered through the existing component schema macros and imported at all expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`). If the component is stored on `EntityKind::Office`, `EntityKind::Faction`, or a new treasury entity kind, the schema must reject invalid attachment.

This spec does not add an agent behavior profile unless implementation chooses a new per-agent spending policy. If it does, the profile must follow `docs/spec-drafting-rules.md` section 5 and become scenario-definable.

## Deliverables

1. Core institutional asset / treasury state with conservation-aware transfer or reservation semantics.
2. Scenario authoring for institutional funds without local-scene side effects.
3. Shared funding authorization and validation helper.
4. `PostBounty` candidate generation using the canonical funding surface.
5. `post_bounty` authoritative validation and commit integration with reservations/encumbrances.
6. Focused tests for authorization, insufficient funds, reservation/release, and scenario spawn.
7. `survival-justice` golden extension proving institutional bounty posting under survival.
8. Generated golden docs and scenario roadmap update once the row is actually landed.

## Acceptance Criteria

1. A scenario can author `Market Warden` funds without creating an extra loose item that changes theft-scene perception.
2. `PostBounty` is not emitted for an accusation case when no lawful reward source exists.
3. `PostBounty` emits with an institutional reward source when the office/faction has accessible funds.
4. `post_bounty` fails authoritatively if funds disappear before start/commit.
5. Multiple active bounties cannot overpromise the same reserved funds.
6. `survival_justice_proves_accusation_substrate`, `survival_justice_proves_fine_punishment_for_same_theft_case`, `survival_justice_proves_search_and_report_found`, and the new bounty-posting golden all pass under the ignored `golden-survival` lane.

## Open Questions

1. Should the first implementation use explicit commodity item lots in a treasury container/site, or a ledger component with conservation events?
2. Should bounty reward funds be reserved at post time or only validated at claim time? FND-4 and overpromise prevention favor reservation.
3. Does office-held fine revenue feed this treasury in the same spec, or should fine revenue remain existing office-owned property until a follow-up connects it?
4. Should the funding surface be shared immediately with future rationing/taxation specs, or kept to bounty posting first?
