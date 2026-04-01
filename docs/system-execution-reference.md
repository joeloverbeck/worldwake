# System Execution Reference

## Tick System Execution Order

Systems run in this order each tick (defined in `system_manifest.rs`):

```
Needs → Production → Trade → Combat → FacilityQueue → Politics → Perception
```

The ordering is load-bearing. Key constraint: **Politics runs before Perception** so that institutional state changes (`OfficeController`, contested state, vacancy) are visible to co-located observers in the same tick via `force_control_claims_for_event()`. Without this, Perception cannot project institutional beliefs from political events (violates Principle 7).

## Force-Control Lifecycle

Force claims do not immediately transfer control. The lifecycle has distinct phases:

```
press_force_claim → hostility + ContestsOffice → (vacancy required) → succession processes → controller → holder
```

- `press_force_claim` creates `ContestsOffice` relation and `hostile_to(challenger, incumbent)` if an `office_holder` exists.
- The succession system (`evaluate_office_succession`) returns `OccupiedNoAction` while a living holder exists — force claims are NOT processed until the office vacates.
- After vacancy, the succession system evaluates pending force claims and establishes a controller.
- After uncontested hold for `succession_period_ticks`, the controller is installed as `office_holder`.

Golden tests that need both hostility (requires incumbent) AND controller establishment (requires vacancy) must include an explicit vacancy step between them.
