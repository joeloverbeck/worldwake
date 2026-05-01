# Direct Root Synthesis Locality Audit

Date: 2026-05-01

## Boundary

Audited `GoalOffer::synthesized_root_candidate_targets` in
`crates/worldwake-ai/src/goal_model.rs` against co-location-bearing
`TargetSpec` registrations and the planner contract in
`docs/planner-contracts.md`.

The upstream affordance query is local for these target specs:

- `TargetSpec::ActorPlace` enumerates the actor's current effective place.
- `TargetSpec::EntityAtActorPlace` enumerates entities from
  `view.entities_at(actor_place)` and filters by kind.

Synthesized roots bypass upstream target enumeration when no affordance
candidate for the same action definition exists yet, so any synthesized root
that names an `ActorPlace` or `EntityAtActorPlace` target must enforce the
same locality contract at synthesis time.

## Findings

| Arm | Goal kinds | TargetSpec | Synthesizer locality gate | Upstream filter | Verdict |
|---|---|---|---|---|---|
| Trade | `AcquireCommodity`, `ConsumeOwnedCommodity`, `RestockCommodity`, `TreatWounds` | `EntityAtActorPlace` | yes: actor place must be in `evidence_places` | yes: local entity enumeration | covered |
| Harvest | `AcquireCommodity`, `RestockCommodity` | `EntityAtActorPlace` | yes: actor place must be in `evidence_places` | yes: local entity enumeration | covered |
| Wash | `Wash` | `EntityAtActorPlace` | yes: actor place must be in `evidence_places` | yes: local entity enumeration | covered |
| Investigate | `InvestigateViolation` | `ActorPlace` | yes: `actor_place == violation place` | yes: actor place target | covered |
| SearchPlace | `SearchForMissing` | `ActorPlace` | yes: synthesizes only the current actor place | yes: actor place target | covered |
| ReportMissing | `ReportMissing` without office target | `ActorPlace` | yes: synthesizes only the current actor place | yes: actor place target | covered |
| EstablishCamp | `EstablishBanditCamp` | `ActorPlace` | yes: `actor_place == camp place` | yes: actor place target | covered |
| Tell | `ShareBelief` | `EntityAtActorPlace` | yes: actor place must be in `evidence_places` | yes: local entity enumeration | covered |
| Fine / Exile | `PunishAccused` | `EntityAtActorPlace` | yes: actor place must be in `evidence_places` | yes: local entity enumeration | covered |
| EscortToSafety | `EscortToSafety` | `EntityAtActorPlace` | yes: actor place must be in `evidence_places` | yes: local entity enumeration | covered |
| PostBounty | `PostBounty` | `ActorPlace` | yes: `actor_place == posting place` | yes: actor place target | covered |
| PostNotice | `PostNotice` | `ActorPlace` | yes: `actor_place == posting place` | yes: actor place target | covered |
| PressForceClaim | `ClaimOffice` | no targets | not applicable | not applicable | covered |
| Accuse | `Accuse` | `SpecificEntity` | not a co-location target spec | not applicable | out of scope |
| ClaimBounty | `FulfillBounty` | `SpecificEntity` | not a co-location target spec | not applicable | out of scope |
| Attack | `EngageHostile`, `RaidTarget` | `EntityAtActorPlace` action family, but synthesis intentionally returns `NoSynthesisPath` | not applicable | yes: local entity enumeration for ordinary affordance candidates | covered |

## Implementation Result

The audit found real synthesis-layer gaps in more arms than the ticket's
initial Trade/Harvest examples. The fix centralizes the synthesis locality
predicate in `GoalOffer` and applies it to every synthesized root that can
otherwise name an `ActorPlace` or `EntityAtActorPlace` target directly.

Remote goals still compose through travel: when the actor reaches the evidence
place, `actor_place` matches the goal's evidence place and the direct local
root can synthesize lawfully.
