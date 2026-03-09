# E20: Companion Behaviors

## Epic Summary
Implement companion need escalation during travel, priority overrides, fallback behaviors when ideal options unavailable, and material/social consequences.

## Phase
Phase 4: Group Adaptation, CLI & Verification

## Crate
`worldwake-systems`

## Dependencies
- E13 (decision architecture for companion planning)

## Deliverables

### Companion Need Escalation
- Companions are regular agents with needs (E09)
- During travel, needs escalate:
  - Bladder increases faster with movement
  - Hunger/thirst progress normally
  - Fatigue increases with travel duration
- Need urgency can override travel plans

### Priority Override
- When companion need reaches critical threshold during travel:
  - Interrupt current travel action (if interruptible)
  - Insert urgent need action into plan
  - Options evaluated in priority order:
    1. Ideal option available? → Use it
    2. No ideal option → Fall to fallback chain

### Fallback Behavior Chain
When ideal option unavailable (e.g., no toilet facility):

1. **Ask to Stop**: companion requests party stop at current/next location
   - Social action: emits request event
   - May be denied by party/circumstances

2. **Seek Privacy/Wilderness**: look for private area
   - If near forest or wilderness node: travel there
   - Duration: varies by distance

3. **Use Wilderness**: address need in non-ideal location
   - Effect: need addressed but hygiene penalty
   - Waste placed in world at location
   - Reduced social standing if witnessed

4. **Accident (Loss of Control)**: if need reaches maximum
   - Involuntary: need addressed without choice
   - Maximum hygiene penalty
   - Waste at current location
   - Social consequences if witnessed
   - Embarrassment/shame effect

### Material Consequences
- Waste entity created at world location (per E09 toilet action)
- Hygiene change on companion
- Resource consumption (water for washing if available)
- All consequences persist in world state

### Social Consequences
- Witnesses react to fallback behaviors:
  - Private behavior: no social impact if unseen
  - Public accident: witnesses gain negative impression
  - Relationship changes: respect/disgust based on circumstances
- Events emitted with appropriate visibility
- Social standing need affected

## Invariants Enforced
- 9.16: Need continuity - no silent reset of bladder/hunger/hygiene
- 9.15: Off-camera continuity - consequences persist regardless of observation

## Tests
- [ ] T23: Companion physiology chain:
  - Needs escalate during travel
  - If toilet available and reachable: companion uses it
  - If blocked: fallback behavior observed
  - Fallback produces material and social consequences
  - Consequences persist in world state
  - At least one fallback observed when ideal unavailable
- [ ] Need escalation rate increases during travel
- [ ] Priority override interrupts travel plan
- [ ] Fallback chain evaluated in order (ideal → ask → seek → wilderness → accident)
- [ ] Waste entity created at correct location
- [ ] Social consequences depend on witnesses
- [ ] No silent need reset after any fallback

## Acceptance Criteria
- Companions have realistic need escalation
- Fallback chain produces emergent behavior variety
- Material consequences (waste, hygiene) persist
- Social consequences depend on perception
- No special-casing: companions use standard agent systems

## Spec References
- Section 1 (exemplar scenario 4: companion bodily needs)
- Section 4.4 (needs: bladder, hygiene)
- Section 7.5 (physiological/social propagation)
- Section 9.16 (need continuity)
