# S49: Golden Gaps — Unified Social Artifact Model

## Summary

Post-implementation golden gap analysis for S45 (Unified Social Artifact Model). The live suite now proves elimination-bounty lifecycle, bounty expiration, and threat-warning route diversion through Scenarios 105-107, but two materially different S45 emergence chains remain unproved at golden E2E level:

1. delivery-bounty fulfillment from perceived artifact through cargo movement and later claim
2. office-vacancy notice uptake changing political planning through the notice-artifact path rather than record consultation or Tell

These are not field-permutation gaps. They are the two remaining cross-system contracts that still distinguish the S45 artifact substrate from existing lower-layer tests and pre-S45 political or cargo goldens.

## Scenario: Delivery Bounty Completes Through Cargo And Claim

An agent perceives an active delivery bounty, uses lawful cargo movement to satisfy the delivery target, then travels to the claim place and claims the bounty reward from the posted artifact.

### Description

1. A human issuer posts a `BountyTarget::DeliverCommodity` bounty with a real reserved reward lot.
2. An AI courier already knows it controls enough of the required commodity to satisfy the bounty.
3. The courier perceives the bounty artifact locally and generates `FulfillBounty`.
4. The courier moves cargo to the destination through the existing transport / stock path.
5. Once the destination delivery condition is satisfied, the courier travels to the claim place.
6. `claim_bounty` becomes lawful only after delivery completion and transfers the real reward.
7. The bounty artifact becomes `Fulfilled`.

### GoalKinds Exercised

- `FulfillBounty`
- `MoveCargo`

### ActionDomains Exercised

- `Social` — `post_bounty`, `claim_bounty`
- `Transport` — `pick_up`, `put_down`, or equivalent cargo staging path
- `Travel` — destination and claim-place routing

### Systems Exercised

- **Social artifact actions**: bounty posting, delivery-side claim validation, reward transfer
- **AI planning**: delivery-bounty operator admission, destination-first then claim-place progression
- **Transport / cargo**: concrete commodity movement to the bounty destination
- **Perception / belief**: local bounty perception via `BelievedArtifactState`

### Setup Requirements

- One AI courier with enough already-controlled commodity to satisfy the bounty without introducing a separate acquisition pipeline
- One human issuer with a real reserved reward lot
- Distinct `destination` and `claim_place` so the scenario proves the two-step delivery-then-claim contract
- Topology that requires at least one real travel leg

### What Emergence It Demonstrates

This proves that delivery bounties are not decorative claim shells. A posted social artifact can drive real cargo progression through the existing transport substrate and only later unlock the terminal claim, with no bounty-only shortcut and no hidden “delivery complete” flag.

### Foundation Principle Alignment

- **Principle 4**: reward transfers from a real reserved lot
- **Principle 7**: bounty knowledge still arrives through local artifact perception
- **Principle 12**: delivery uses canonical cargo and claim paths rather than a separate bounty subsystem
- **Principle 25**: the bounty remains a first-class world artifact with stable identity, destination, and claim place

### Why It Is Not A Duplicate

- **Scenario 105** proves elimination-bounty combat and later claim, not delivery-side cargo progression.
- Focused planner tests from `S45UNISOCART-007` prove lower-layer operator admission and search shape, but there is still no end-to-end golden that shows delivery completion and later claim in one world chain.

## Scenario: Vacancy Notice Unlocks Political Action Without Record Consult

An office-vacancy notice is posted at a place, a local agent perceives it as a social artifact, internalizes the vacancy through the notice path, and then generates political action without needing remote record consultation or a tell relay.

### Description

1. An office is vacant but the claimant has no prior office-holder belief and no consulted office register.
2. A human issuer posts a `NoticeTopic::OfficeVacancy { office }` artifact at the same place as the claimant.
3. The claimant perceives the notice and internalizes `InstitutionalClaim::OfficeHolder { holder: None }` through the artifact path.
4. On the next AI tick, the claimant generates lawful political action from that notice-derived vacancy belief.
5. The selected plan proceeds directly to the local political action surface rather than detouring through `consult_record`.

### GoalKinds Exercised

- `ClaimOffice` or `SupportCandidateForOffice` (whichever is lawful for the chosen succession law)

### ActionDomains Exercised

- `Social` — `post_notice`
- `Generic` — political action such as `declare_support` or `press_force_claim`

### Systems Exercised

- **Social artifact actions**: notice posting
- **Perception**: artifact projection plus notice-topic internalization into institutional belief
- **AI candidate generation / planning**: political goal emission from notice-derived vacancy knowledge
- **Politics / succession**: the existing local claim or support action path

### Setup Requirements

- One vacant office with a single clear lawful claimant
- No seeded office-holder-none belief and no pre-consulted office register for the claimant
- Claimant co-located with the notice posting place
- Succession-law choice that gives one clean local political action path

### What Emergence It Demonstrates

This proves that notice artifacts are not just readable metadata. A posted vacancy notice becomes concrete political knowledge through the perception path and changes behavior without cheating through a direct record-read or a tell-driven shortcut.

### Foundation Principle Alignment

- **Principle 7**: political knowledge arrives through local perception of the posted artifact
- **Principle 12**: notice artifacts feed the existing institutional-belief and political-action lanes rather than a duplicate special-case planner hook
- **Principle 18**: the notice persists as world state and has real downstream consequences
- **Principle 25**: notices are first-class social artifacts whose discovery can alter future behavior

### Why It Is Not A Duplicate

- **Scenario 73** proves remote record consultation as the prerequisite for political action when vacancy knowledge is unknown.
- **Scenario 46** proves Tell-based political knowledge propagation.
- **Scenario 107** proves threat-warning route avoidance, not notice-driven political uptake.

## Ticket Breakdown

### S49GOLGAP-001: Delivery-bounty golden closeout

- Add a golden scenario plus deterministic replay companion for delivery-bounty fulfillment
- Assert:
  - local bounty perception
  - delivery-side `FulfillBounty` selection
  - cargo reaches the bounty destination through lawful transport
  - `claim_bounty` commits only after delivery completion
  - reward conservation and `ArtifactState::Fulfilled`

**Files**: `crates/worldwake-ai/tests/golden_integration.rs`
**Effort**: Medium

### S49GOLGAP-002: Vacancy-notice political uptake golden

- Add a golden scenario plus deterministic replay companion for office-vacancy notice discovery unlocking local political action
- Assert:
  - notice perception and `believed_artifact`
  - internalized vacancy belief through the artifact path
  - political candidate appears without `consult_record`
  - local political action starts or commits through the normal politics surface

**Files**: `crates/worldwake-ai/tests/golden_integration.rs` or `crates/worldwake-ai/tests/golden_offices.rs`
**Effort**: Medium

## Tests

- [ ] delivery bounty completes through cargo movement and later claim
- [ ] deterministic replay companion for delivery bounty scenario
- [ ] office-vacancy notice unlocks political action without record consult
- [ ] deterministic replay companion for vacancy-notice scenario

## Acceptance Criteria

1. Delivery-bounty golden proves perception -> cargo progress -> claim -> reward without a bounty-only shortcut
2. Vacancy-notice golden proves artifact perception can unlock political action through the existing institutional-belief lane
3. Both primary scenarios include deterministic replay companions
4. Conservation holds for the delivery-bounty reward path
5. Assertions use the strongest honest surfaces available: authoritative world state for durable outcomes, action traces for lifecycle order, decision traces for candidate or plan-shape claims
