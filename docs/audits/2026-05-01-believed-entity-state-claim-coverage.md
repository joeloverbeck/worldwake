# BelievedEntityState Claim-Aspect Coverage Audit

Date: 2026-05-01

Ticket: `archive/tickets/BELASPCOV-001-believed-entity-state-claim-aspect-coverage.md`

## Scope

This audit maps every non-metadata `BelievedEntityState` field in
`crates/worldwake-core/src/belief.rs` to the `EntityBeliefAspect` and
`ClaimValue` lanes that hydrate it through:

1. `AgentBeliefStore::record_entity_snapshot_claims`
2. `entity_claims_for_snapshot`
3. `derive_entity_summary`
4. `AgentBeliefStore::prune_decayed_beliefs`

`presentation_ticks`, `presentation_tick_count`, and `source` are observation
metadata. They are not belief-content fields and are excluded from the
coverage verdicts.

## Verdict Summary

No missing claim-aspect coverage gaps were found for mutable belief-content
fields.

`believed_kind` is the only exposed summary field without an
`EntityBeliefAspect`. It is intentionally preserved as entity identity /
presentation metadata rather than a mutable claim: direct observation sets it
from `World::entity_kind`, `record_entity_snapshot_claims` installs it on a new
summary, and `refresh_entity_summary_from_claims` preserves the prior value
across claim rehydration. It does not follow the pre-CIREM-003 wash-basin
failure mode, where a mutable state field vanished while the known entity was
otherwise retained.

Because the audit found no `gap` verdicts, no `BELASPCOV-002` follow-up ticket
was filed.

## Coverage Table

| `BelievedEntityState` field | Claim aspect / value | Direct observation records claim? | Claim hydration projection | Stale-claim decay behavior | Verdict |
| --- | --- | --- | --- | --- | --- |
| `believed_kind` | None | No claim; set from `ObservedEntitySnapshot::believed_kind` / `World::entity_kind` | Preserved by `preserve_believed_kind` after `derive_entity_summary` | Survives claim-summary refresh while the entity remains known; removed only when the known entity itself is pruned | Intentionally-derived metadata |
| `last_known_place` | `EntityBeliefAspect::Location` / `ClaimValue::Place` | Always records a location claim, including `None` | Sets `summary.last_known_place` | Survives while the winning `Location` claim survives; if the claim decays, the field is correctly absent from the retained summary | Covered |
| `last_known_inventory` | `EntityBeliefAspect::Inventory(CommodityKind)` / `ClaimValue::Quantity` | Records one claim per current commodity and zero-quantity claims for prior commodities that disappeared | Positive quantities are inserted; zero quantities clear that commodity | Per-commodity lanes decay independently; disappeared commodities are explicitly represented by zero claims | Covered |
| `workstation_tag` | `EntityBeliefAspect::WorkstationPresent` / `ClaimValue::WorkstationTag` | Records when currently present or when prior summary had a tag, including `None` clears | Sets `summary.workstation_tag` | Survives while the `WorkstationPresent` claim survives; direct observed concrete-opportunity retention can boost relevant infrastructure claims | Covered |
| `resource_source` | `EntityBeliefAspect::ResourceAvailable(CommodityKind)` / `ClaimValue::ResourceSource` | Records the current resource source by commodity and emits a `None` claim for a prior commodity lane when removed or changed | Sets `summary.resource_source` from the winning resource-available lane | Survives while the resource claim survives; direct observed resource infrastructure receives salience retention when paired with a workstation tag | Covered |
| `wash_basin_state` | `EntityBeliefAspect::WashBasinState` / `ClaimValue::WashBasinState` | Records when currently present or when prior summary had basin state, including `None` clears | Sets `summary.wash_basin_state` | Survives through the claim lane added by S129CIREM-003; direct observed wash-basin infrastructure receives salience retention | Covered |
| `alive` | `EntityBeliefAspect::Alive` / `ClaimValue::Bool` | Records when false or when the prior summary had a different alive value | Sets `summary.alive`; default summary value is true | Death is explicit claim content; alive true may be omitted for baseline observations unless needed to correct prior state | Covered |
| `wounds` | `EntityBeliefAspect::Wounded` / `ClaimValue::WoundSnapshot` | Records when wounds are present or prior wounds must be cleared | Replaces `summary.wounds` | Wound state survives only through the wound claim lane; empty wound snapshots clear prior wounds | Covered |
| `last_known_courage` | `EntityBeliefAspect::Courage` / `ClaimValue::Courage` | Records when currently present or when prior summary had courage, including `None` clears | Sets `summary.last_known_courage` | Survives while the courage claim survives | Covered |
| `believed_activity` | `EntityBeliefAspect::Activity` / `ClaimValue::Activity` | Records when currently present or when prior summary had activity, including `None` clears | Sets `summary.believed_activity` | Survives while the activity claim survives; activity helpers also record/refute this aspect directly | Covered |
| `believed_artifact` | `EntityBeliefAspect::ArtifactState` / `ClaimValue::ArtifactState` | Records when currently present or when prior summary had artifact state, including `None` clears | Sets `summary.believed_artifact` | Survives while the artifact-state claim survives | Covered |
| `believed_contention` | `EntityBeliefAspect::ContentionState` / `ClaimValue::ContentionState` | Records when currently present or when prior summary had contention state, including `None` clears | Sets `summary.believed_contention` | Survives while the contention claim survives | Covered |
| `believed_evidence` | `EntityBeliefAspect::Evidence` / `ClaimValue::EvidenceState` | Records when currently present or when prior summary had evidence state, including `None` clears | Sets `summary.believed_evidence` | Survives while the evidence claim survives | Covered |

## Projection Soundness

Every `EntityBeliefAspect` variant in
`crates/worldwake-core/src/entity_belief_claim.rs` has one unambiguous
projection in `derive_entity_summary`:

| `EntityBeliefAspect` variant | `ClaimValue` shape | Summary target |
| --- | --- | --- |
| `Location` | `Place` | `last_known_place` |
| `Inventory(CommodityKind)` | `Quantity` | `last_known_inventory[commodity]`, with zero clearing |
| `Alive` | `Bool` | `alive` |
| `Wounded` | `WoundSnapshot` | `wounds` |
| `Activity` | `Activity` | `believed_activity` |
| `WorkstationPresent` | `WorkstationTag` | `workstation_tag` |
| `ResourceAvailable(CommodityKind)` | `ResourceSource` | `resource_source` |
| `ContentionState` | `ContentionState` | `believed_contention` |
| `WashBasinState` | `WashBasinState` | `wash_basin_state` |
| `ArtifactState` | `ArtifactState` | `believed_artifact` |
| `Courage` | `Courage` | `last_known_courage` |
| `Evidence` | `EvidenceState` | `believed_evidence` |

Mismatched aspect/value pairs are ignored by the wildcard arm in
`derive_entity_summary`. The live round-trip test
`belief::tests::entity_belief_claim_roundtrips_through_bincode` covers the
serialized claim carrier, including the current aspect/value enum surface.

## Verification

Commands run:

1. `grep -n 'EntityBeliefAspect::' crates/worldwake-core/src/belief.rs | sort -u`
2. `cargo test -p worldwake-core entity_belief_claim_roundtrips_through_bincode -- --list`
3. `cargo test -p worldwake-core entity_belief_claim_roundtrips_through_bincode`
4. `cargo test --workspace`

The drafted integration-test command
`cargo test -p worldwake-core --test entity_belief_claim_roundtrips_through_bincode`
does not match the live repo layout. The bincode proof is a `worldwake-core`
library unit test, so the live selector above is the truthful command.
