# Simulation Observer Dump

## Section 1 — Run Metadata

- **Scenario**: `scenarios/survival-baseline.ron`
- **Seed**: 104004
- **Ticks simulated**: 1440
- **Total events**: 47002

### Agents

| Name | EntityId |
|------|----------|
| Agent A | e4g0 |
| Agent B | e5g0 |
| Agent C | e6g0 |

### Places

| Name | EntityId |
|------|----------|
| Riverside Camp | e0g0 |
| Fertile Fields | e1g0 |
| Forest Clearing | e2g0 |
| Hillside Shelter | e3g0 |

## Section 2 — Per-Agent Summary

### Agent A

**Actions** (total lifecycle events: 493)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 4 | 4 | 0 | 0 |
| eat | 30 | 30 | 0 | 0 |
| harvest:Harvest Apples | 15 | 15 | 0 | 1 |
| harvest:Harvest Water | 3 | 3 | 0 | 0 |
| pick_up | 18 | 18 | 0 | 0 |
| relieve_wilderness | 22 | 22 | 0 | 0 |
| sleep | 144 | 144 | 0 | 0 |
| toilet | 1 | 1 | 0 | 0 |
| travel | 7 | 7 | 0 | 0 |
| wash | 2 | 2 | 0 | 0 |

**Perception**: 193 total observations, 173 passed, 55 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 2 | 490 | 82 |
| Thirst | 3 | 283 | 144 |
| Fatigue | 122 | 338 | 288 |
| Bladder | 4 | 452 | 172 |
| Dirtiness | 1 | 627 | 292 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 15 |
| e1g0 | 1173 |
| e2g0 | 242 |

**Max consecutive idle ticks**: 36

### Agent B

**Actions** (total lifecycle events: 503)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 3 | 3 | 0 | 0 |
| eat | 32 | 32 | 0 | 0 |
| harvest:Harvest Apples | 16 | 16 | 0 | 0 |
| harvest:Harvest Water | 3 | 3 | 0 | 1 |
| pick_up | 21 | 21 | 0 | 0 |
| relieve_wilderness | 22 | 22 | 0 | 0 |
| sleep | 145 | 145 | 0 | 0 |
| travel | 6 | 6 | 0 | 0 |
| wash | 3 | 3 | 0 | 0 |

**Perception**: 212 total observations, 195 passed, 58 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 3 | 506 | 87 |
| Thirst | 3 | 269 | 148 |
| Fatigue | 142 | 338 | 289 |
| Bladder | 4 | 504 | 175 |
| Dirtiness | 1 | 635 | 289 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 2 |
| e1g0 | 1283 |
| e2g0 | 148 |

**Max consecutive idle ticks**: 41

### Agent C

**Actions** (total lifecycle events: 542)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 11 | 11 | 0 | 0 |
| eat | 31 | 31 | 0 | 0 |
| harvest:Harvest Apples | 16 | 16 | 0 | 4 |
| harvest:Harvest Water | 7 | 7 | 0 | 0 |
| pick_up | 23 | 23 | 0 | 0 |
| relieve_wilderness | 25 | 25 | 0 | 0 |
| sleep | 146 | 146 | 0 | 0 |
| travel | 7 | 7 | 0 | 0 |
| wash | 3 | 3 | 0 | 0 |

**Perception**: 199 total observations, 173 passed, 53 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 2 | 434 | 94 |
| Thirst | 4 | 284 | 121 |
| Fatigue | 112 | 336 | 288 |
| Bladder | 4 | 548 | 173 |
| Dirtiness | 1 | 588 | 245 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Behavioral transition** at tick 1000: action repertoire narrowed (8 types -> 4 types)
  Needs: hunger=94, thirst=84, fatigue=282, bladder=208, dirtiness=108

**Behavioral transition** at tick 1400: action repertoire narrowed (8 types -> 3 types)
  Needs: hunger=86, thirst=68, fatigue=282, bladder=184, dirtiness=101

**Locations visited**

| Place | Ticks |
|-------|-------|
| e1g0 | 1102 |
| e2g0 | 331 |

**Max consecutive idle ticks**: 27

## Section 3 — Anomaly Flags

No anomalies detected.

## Section 4 — Raw Event Sample

### First 100 events

```
[0] tick=0 actor=None action=None place=None tags={} deltas=0
[1] tick=0 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[2] tick=0 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[3] tick=0 actor=None action=None place=None tags={System} deltas=0
[4] tick=1 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[5] tick=1 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[6] tick=1 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[7] tick=1 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[8] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[9] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[10] tick=1 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[11] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[12] tick=1 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[13] tick=1 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[14] tick=1 actor=None action=None place=None tags={System} deltas=0
[15] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[16] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[17] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[18] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[19] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("travel") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted, Travel} deltas=0
[20] tick=2 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[21] tick=2 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[22] tick=2 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[23] tick=2 actor=None action=None place=None tags={System} deltas=0
[24] tick=3 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[25] tick=3 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[26] tick=3 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[27] tick=3 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[28] tick=3 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=0
[29] tick=3 actor=None action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, System} deltas=0
[30] tick=3 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[31] tick=3 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[32] tick=3 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[33] tick=3 actor=None action=None place=None tags={System} deltas=0
[34] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[35] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[36] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[37] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[38] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[39] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[40] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[41] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[42] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("travel") place=None tags={ActionCommitted, Travel} deltas=0
[43] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[44] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[45] tick=4 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[46] tick=4 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[47] tick=4 actor=None action=None place=None tags={System} deltas=0
[48] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[49] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[50] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[51] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[52] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[53] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[54] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[55] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[56] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("harvest:Harvest Apples") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[57] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[58] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[59] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[60] tick=5 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[61] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[62] tick=5 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[63] tick=5 actor=None action=None place=None tags={System} deltas=0
[64] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[65] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[66] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[67] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[68] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("toilet") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[69] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("relieve_wilderness") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[70] tick=6 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[71] tick=6 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=0
[72] tick=6 actor=None action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, System} deltas=0
[73] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[74] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[75] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[76] tick=6 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[77] tick=6 actor=None action=None place=None tags={System} deltas=0
[78] tick=7 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("harvest:Harvest Apples") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[79] tick=7 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[80] tick=7 actor=None action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, System} deltas=0
[81] tick=7 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[82] tick=7 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[83] tick=7 actor=None action=None place=None tags={System} deltas=0
[84] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[85] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[86] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[87] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[88] tick=8 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[89] tick=8 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[90] tick=8 actor=None action=None place=None tags={System} deltas=0
[91] tick=9 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[92] tick=9 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[93] tick=9 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[94] tick=9 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[95] tick=9 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=0
[96] tick=9 actor=None action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, System} deltas=0
[97] tick=9 actor=None action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, System} deltas=0
[98] tick=9 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[99] tick=9 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
```

### Last 100 events

```
[46902] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46903] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46904] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46905] tick=1436 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[46906] tick=1436 actor=None action=None place=None tags={System} deltas=0
[46907] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("relieve_wilderness") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted, WildernessRelief} deltas=7
[46908] tick=1437 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[46909] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46910] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46911] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46912] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46913] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46914] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46915] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46916] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46917] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46918] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46919] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46920] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46921] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46922] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46923] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46924] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46925] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46926] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46927] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46928] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46929] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46930] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46931] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46932] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46933] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46934] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46935] tick=1437 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[46936] tick=1437 actor=None action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, System} deltas=1
[46937] tick=1437 actor=None action=None place=None tags={System} deltas=0
[46938] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[46939] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[46940] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("sleep") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[46941] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("sleep") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=1
[46942] tick=1438 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[46943] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46944] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46945] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46946] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46947] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46948] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46949] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46950] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46951] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46952] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46953] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46954] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46955] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46956] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46957] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46958] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46959] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46960] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46961] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46962] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46963] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46964] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46965] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46966] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46967] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46968] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46969] tick=1438 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[46970] tick=1438 actor=None action=None place=None tags={System} deltas=0
[46971] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[46972] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("relieve_wilderness") place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, ActionCommitted, WildernessRelief} deltas=7
[46973] tick=1439 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[46974] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46975] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46976] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46977] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46978] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46979] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46980] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46981] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46982] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46983] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46984] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46985] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46986] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46987] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46988] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46989] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46990] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46991] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46992] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46993] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46994] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46995] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46996] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46997] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46998] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[46999] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[47000] tick=1439 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[47001] tick=1439 actor=None action=None place=None tags={System} deltas=0
```

### Action Trace Summary

Total action trace events: 1538

#### Per-Agent Action Timeline (100-tick bins)

**Agent A (e4g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | travel×3, drink×2, eat×2, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2, sleep×2, harvest:Harvest Water×1, toilet×1 |
| 100–199 | sleep×11, eat×1, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 200–299 | sleep×10, eat×3, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 300–399 | sleep×11, eat×2, harvest:Harvest Apples×2, relieve_wilderness×2, pick_up×1 |
| 400–499 | sleep×11, eat×2, pick_up×2, relieve_wilderness×2, drink×1, harvest:Harvest Water×1, travel×1, wash×1 |
| 500–599 | sleep×10, eat×1, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1, travel×1 |
| 600–699 | sleep×11, eat×3, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 700–799 | sleep×10, eat×2, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 800–899 | sleep×11, eat×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×1 |
| 900–999 | sleep×11, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1, travel×1 |
| 1000–1099 | sleep×10, relieve_wilderness×2, drink×1, harvest:Harvest Water×1, pick_up×1, wash×1 |
| 1100–1199 | sleep×11, eat×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1, travel×1 |
| 1200–1299 | sleep×11, eat×2, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2 |
| 1300–1399 | sleep×10, eat×3, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 1400–1499 | sleep×4, eat×1, relieve_wilderness×1 |

**Agent B (e5g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | eat×3, sleep×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2, harvest:Harvest Water×1, travel×1 |
| 100–199 | sleep×11, eat×3, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 200–299 | sleep×10, eat×2, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 300–399 | sleep×11, eat×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×1 |
| 400–499 | sleep×11, relieve_wilderness×2, drink×1, eat×1, harvest:Harvest Water×1, pick_up×1, travel×1, wash×1 |
| 500–599 | sleep×10, eat×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1, travel×1 |
| 600–699 | sleep×11, eat×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 700–799 | sleep×11, eat×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2 |
| 800–899 | sleep×10, eat×2, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 900–999 | sleep×11, eat×2, pick_up×2, travel×2, drink×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, relieve_wilderness×1, wash×1 |
| 1000–1099 | sleep×11, eat×1, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 1100–1199 | sleep×10, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 1200–1299 | sleep×11, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 1300–1399 | sleep×10, eat×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 1400–1499 | sleep×4, pick_up×3, drink×1, harvest:Harvest Water×1, relieve_wilderness×1, travel×1, wash×1 |

**Agent C (e6g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | relieve_wilderness×3, drink×2, eat×2, pick_up×2, sleep×2, harvest:Harvest Apples×1, harvest:Harvest Water×1, travel×1 |
| 100–199 | sleep×11, eat×3, harvest:Harvest Apples×3, pick_up×2, relieve_wilderness×1 |
| 200–299 | sleep×10, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 300–399 | sleep×11, drink×2, harvest:Harvest Water×2, pick_up×2, relieve_wilderness×2, travel×2, harvest:Harvest Apples×1, wash×1 |
| 400–499 | sleep×11, eat×2, harvest:Harvest Apples×2, drink×1, pick_up×1, relieve_wilderness×1 |
| 500–599 | sleep×11, eat×2, harvest:Harvest Apples×2, relieve_wilderness×2, pick_up×1 |
| 600–699 | sleep×10, eat×4, pick_up×2, harvest:Harvest Apples×1, relieve_wilderness×1 |
| 700–799 | sleep×11, eat×4, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2 |
| 800–899 | sleep×11, eat×2, pick_up×2, harvest:Harvest Apples×1, harvest:Harvest Water×1, relieve_wilderness×1, travel×1, wash×1 |
| 900–999 | sleep×11, drink×2, pick_up×2, relieve_wilderness×2, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, travel×1 |
| 1000–1099 | sleep×10, relieve_wilderness×2, drink×1, eat×1 |
| 1100–1199 | sleep×11, eat×4, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2 |
| 1200–1299 | sleep×10, eat×2, harvest:Harvest Apples×2, pick_up×2, harvest:Harvest Water×1, relieve_wilderness×1, travel×1, wash×1 |
| 1300–1399 | sleep×12, drink×2, pick_up×2, relieve_wilderness×2, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, travel×1 |
| 1400–1499 | sleep×4, drink×1, relieve_wilderness×1 |

#### Raw Action Trace (last 50 events)

```
tick 1390 seq 4: e5g0 committed 'sleep' (instance ai740, 0 materializations)
tick 1390 seq 5: e6g0 committed 'sleep' (instance ai741, 0 materializations)
tick 1400 seq 0: e4g0 started 'sleep' targeting []
tick 1400 seq 1: e5g0 started 'sleep' targeting []
tick 1400 seq 2: e6g0 started 'sleep' targeting []
tick 1400 seq 3: e4g0 committed 'sleep' (instance ai742, 0 materializations)
tick 1400 seq 4: e5g0 committed 'sleep' (instance ai743, 0 materializations)
tick 1400 seq 5: e6g0 committed 'sleep' (instance ai744, 0 materializations)
tick 1404 seq 0: e4g0 started 'eat' targeting [EntityId { slot: 190, generation: 0 }]
tick 1405 seq 0: e4g0 committed 'eat' (instance ai745, 0 materializations)
tick 1406 seq 0: e5g0 started 'travel' targeting [EntityId { slot: 2, generation: 0 }]
tick 1407 seq 0: e5g0 committed 'travel' (instance ai746, 0 materializations)
tick 1410 seq 0: e4g0 started 'sleep' targeting []
tick 1410 seq 1: e5g0 started 'sleep' targeting []
tick 1410 seq 2: e6g0 started 'sleep' targeting []
tick 1410 seq 3: e4g0 committed 'sleep' (instance ai747, 0 materializations)
tick 1410 seq 4: e5g0 committed 'sleep' (instance ai748, 0 materializations)
tick 1410 seq 5: e6g0 committed 'sleep' (instance ai749, 0 materializations)
tick 1411 seq 0: e5g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 8, generation: 0 }]
tick 1413 seq 0: e5g0 committed 'harvest:Harvest Water' (instance ai750, 0 materializations)
tick 1414 seq 0: e4g0 started 'relieve_wilderness' targeting []
tick 1414 seq 1: e5g0 started 'pick_up' targeting [EntityId { slot: 17, generation: 0 }]
tick 1414 seq 2: e5g0 committed 'pick_up' (instance ai752, 0 materializations)
tick 1415 seq 0: e5g0 started 'pick_up' targeting [EntityId { slot: 59, generation: 0 }]
tick 1415 seq 1: e5g0 committed 'pick_up' (instance ai753, 0 materializations)
tick 1416 seq 0: e5g0 started 'pick_up' targeting [EntityId { slot: 195, generation: 0 }]
tick 1416 seq 1: e5g0 committed 'pick_up' (instance ai754, 0 materializations)
tick 1417 seq 0: e5g0 started 'wash' targeting [EntityId { slot: 195, generation: 0 }]
tick 1420 seq 0: e6g0 started 'sleep' targeting []
tick 1420 seq 1: e6g0 committed 'sleep' (instance ai756, 0 materializations)
tick 1421 seq 0: e4g0 committed 'relieve_wilderness' (instance ai751, 0 materializations)
tick 1422 seq 0: e4g0 started 'sleep' targeting []
tick 1422 seq 1: e4g0 committed 'sleep' (instance ai757, 0 materializations)
tick 1428 seq 0: e5g0 committed 'wash' (instance ai755, 0 materializations)
tick 1429 seq 0: e5g0 started 'sleep' targeting []
tick 1429 seq 1: e6g0 started 'drink' targeting [EntityId { slot: 186, generation: 0 }]
tick 1429 seq 2: e5g0 committed 'sleep' (instance ai758, 0 materializations)
tick 1429 seq 3: e6g0 committed 'drink' (instance ai759, 0 materializations)
tick 1430 seq 0: e4g0 started 'sleep' targeting []
tick 1430 seq 1: e5g0 started 'sleep' targeting []
tick 1430 seq 2: e6g0 started 'relieve_wilderness' targeting []
tick 1430 seq 3: e4g0 committed 'sleep' (instance ai760, 0 materializations)
tick 1430 seq 4: e5g0 committed 'sleep' (instance ai761, 0 materializations)
tick 1431 seq 0: e5g0 started 'drink' targeting [EntityId { slot: 195, generation: 0 }]
tick 1431 seq 1: e5g0 committed 'drink' (instance ai763, 0 materializations)
tick 1432 seq 0: e5g0 started 'relieve_wilderness' targeting []
tick 1437 seq 0: e6g0 committed 'relieve_wilderness' (instance ai762, 0 materializations)
tick 1438 seq 0: e6g0 started 'sleep' targeting []
tick 1438 seq 1: e6g0 committed 'sleep' (instance ai765, 0 materializations)
tick 1439 seq 0: e5g0 committed 'relieve_wilderness' (instance ai764, 0 materializations)
```

### Perception Trace Summary

Total perception trace events: 604

**Agent A (e4g0)** — 193 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 13 | 2 | 9 |
| 100–199 | 13 | 3 | 8 |
| 200–299 | 13 | 3 | 10 |
| 300–399 | 11 | 1 | 7 |
| 400–499 | 11 | 4 | 7 |
| 500–599 | 9 | 1 | 9 |
| 600–699 | 12 | 0 | 8 |
| 700–799 | 20 | 2 | 10 |
| 800–899 | 11 | 2 | 6 |
| 900–999 | 11 | 0 | 7 |
| 1000–1099 | 6 | 0 | 5 |
| 1100–1199 | 12 | 1 | 9 |
| 1200–1299 | 17 | 0 | 9 |
| 1300–1399 | 10 | 1 | 6 |
| 1400–1499 | 4 | 0 | 5 |

**Agent B (e5g0)** — 212 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 18 | 1 | 9 |
| 100–199 | 14 | 2 | 9 |
| 200–299 | 16 | 0 | 10 |
| 300–399 | 12 | 0 | 7 |
| 400–499 | 12 | 1 | 8 |
| 500–599 | 14 | 3 | 10 |
| 600–699 | 11 | 1 | 8 |
| 700–799 | 21 | 1 | 10 |
| 800–899 | 13 | 0 | 6 |
| 900–999 | 13 | 1 | 12 |
| 1000–1099 | 8 | 0 | 7 |
| 1100–1199 | 16 | 2 | 9 |
| 1200–1299 | 14 | 3 | 7 |
| 1300–1399 | 10 | 1 | 7 |
| 1400–1499 | 3 | 1 | 4 |

**Agent C (e6g0)** — 199 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 18 | 2 | 10 |
| 100–199 | 13 | 3 | 9 |
| 200–299 | 14 | 2 | 9 |
| 300–399 | 8 | 1 | 6 |
| 400–499 | 8 | 2 | 7 |
| 500–599 | 14 | 2 | 9 |
| 600–699 | 10 | 2 | 8 |
| 700–799 | 20 | 2 | 11 |
| 800–899 | 7 | 2 | 6 |
| 900–999 | 15 | 1 | 10 |
| 1000–1099 | 8 | 0 | 7 |
| 1100–1199 | 15 | 3 | 7 |
| 1200–1299 | 11 | 3 | 9 |
| 1300–1399 | 8 | 1 | 9 |
| 1400–1499 | 4 | 0 | 5 |

#### Raw Perception Trace (last 50 events)

```
tick 1290 seq 3: e5g0 observed ev42051 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1291 seq 0: e6g0 observed ev42089 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1294 seq 0: e6g0 observed ev42186 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1297 seq 0: e4g0 observed ev42282 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1297 seq 1: e5g0 observed ev42282 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1298 seq 0: e4g0 observed ev42315 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1298 seq 1: e5g0 observed ev42315 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1305 seq 0: e4g0 observed ev42546 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1305 seq 1: e5g0 observed ev42546 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1308 seq 0: e6g0 observed ev42660 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1315 seq 0: e6g0 observed ev42883 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1333 seq 0: e4g0 observed ev43460 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1333 seq 1: e5g0 observed ev43460 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1335 seq 0: e4g0 observed ev43533 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1335 seq 1: e5g0 observed ev43533 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1352 seq 0: e4g0 observed ev44048 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1352 seq 1: e5g0 observed ev44048 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1354 seq 0: e6g0 observed ev44106 (FAILED @ 900‰), 0 entities, 0 institutional claims
tick 1357 seq 0: e6g0 observed ev44203 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1358 seq 0: e6g0 observed ev44232 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1359 seq 0: e4g0 observed ev44259 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1359 seq 1: e5g0 observed ev44259 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1365 seq 0: e6g0 observed ev44434 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1368 seq 0: e4g0 observed ev44526 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1368 seq 1: e5g0 observed ev44526 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1370 seq 0: e4g0 observed ev44598 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1370 seq 1: e5g0 observed ev44598 (FAILED @ 900‰), 0 entities, 0 institutional claims
tick 1371 seq 0: e4g0 observed ev44627 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1371 seq 1: e5g0 observed ev44627 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1378 seq 0: e4g0 observed ev44848 (FAILED @ 900‰), 0 entities, 0 institutional claims
tick 1378 seq 1: e5g0 observed ev44848 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1378 seq 2: e6g0 observed ev44848 (passed @ 720‰), 3 entities, 0 institutional claims
tick 1380 seq 0: e4g0 observed ev44928 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1380 seq 1: e5g0 observed ev44928 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1380 seq 2: e6g0 observed ev44928 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1382 seq 0: e4g0 observed ev45030 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1382 seq 1: e5g0 observed ev45030 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1382 seq 2: e6g0 observed ev45030 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1413 seq 0: e5g0 observed ev46239 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1414 seq 0: e4g0 observed ev46269 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1414 seq 1: e6g0 observed ev46269 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1416 seq 0: e5g0 observed ev46326 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1421 seq 0: e4g0 observed ev46458 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1421 seq 1: e6g0 observed ev46458 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1430 seq 0: e4g0 observed ev46701 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1430 seq 1: e6g0 observed ev46701 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1432 seq 0: e5g0 observed ev46768 (FAILED @ 900‰), 0 entities, 0 institutional claims
tick 1437 seq 0: e4g0 observed ev46907 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1437 seq 1: e6g0 observed ev46907 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1439 seq 0: e5g0 observed ev46972 (passed @ 900‰), 3 entities, 0 institutional claims
```

## Section 5 — Per-Agent Belief Summary

### Agent A

**Known entities**: 87
- Agents: 3
- Places: 2
- Items: 80
- Other: 2

**Believed entity locations**:
- (place entity — no parent location): Fertile Fields, Forest Clearing
- Fertile Fields: Agent A, Agent C, OrchardRow, ItemLot#153, ItemLot#156, ItemLot#159, ItemLot#162, ItemLot#166, ItemLot#168, ItemLot#171, ItemLot#175, ItemLot#177, ItemLot#184, ItemLot#186, ItemLot#190, 1× Apple, 55× Waste
- Forest Clearing: Agent B, Well, ItemLot#148, 11× Waste

**Social observations**: 0
**Told beliefs**: 0
**Heard beliefs**: 0
**Institutional beliefs**: 0

### Agent B

**Known entities**: 91
- Agents: 3
- Places: 2
- Items: 84
- Other: 2

**Believed entity locations**:
- (place entity — no parent location): Fertile Fields, Forest Clearing
- Fertile Fields: Agent A, Agent C, OrchardRow, ItemLot#134, ItemLot#141, ItemLot#144, ItemLot#153, ItemLot#156, ItemLot#159, ItemLot#162, ItemLot#166, ItemLot#168, ItemLot#171, ItemLot#175, ItemLot#177, ItemLot#184, ItemLot#186, ItemLot#190, 1× Apple, 53× Waste
- Forest Clearing: Agent B, Well, ItemLot#195, 14× Waste

**Social observations**: 0
**Told beliefs**: 0
**Heard beliefs**: 0
**Institutional beliefs**: 0

### Agent C

**Known entities**: 91
- Agents: 3
- Places: 2
- Items: 84
- Other: 2

**Believed entity locations**:
- (place entity — no parent location): Fertile Fields, Forest Clearing
- Fertile Fields: Agent A, Agent C, OrchardRow, ItemLot#134, ItemLot#141, ItemLot#144, ItemLot#153, ItemLot#156, ItemLot#159, ItemLot#162, ItemLot#166, ItemLot#168, ItemLot#171, ItemLot#175, ItemLot#184, ItemLot#186, ItemLot#190, 1× Apple, 55× Waste
- Forest Clearing: Agent B, Well, ItemLot#178, 13× Waste

**Social observations**: 0
**Told beliefs**: 0
**Heard beliefs**: 0
**Institutional beliefs**: 0

## Section 6 — End-State Inventory & Resources

### Agent Inventories

**Agent A**: (empty)

**Agent B**: 2× Waste

**Agent C**: 1× Apple

### Place Contents

**Riverside Camp (e0g0)**: Well (Well), 1× Waste

**Fertile Fields (e1g0)**: Agent A (agent), Agent C (agent), OrchardRow (OrchardRow), 1× Apple, 55× Waste

**Forest Clearing (e2g0)**: Agent B (agent), Well (Well), 14× Waste

**Hillside Shelter (e3g0)**: (empty)

## Section 7 — Per-Agent Decision Summary

### Agent A (1440 decision ticks)

**Tick breakdown**: 1181 planning, 259 active-action, 0 dead
**Plan search outcomes**: 249 found, 0 frontier-exhausted, 0 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Apple, purpose: SelfConsume }, AcquireCommodity { commodity: Bread, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Apple }, ConsumeOwnedCommodity { commodity: Water }, ExploreLocation { target_place: EntityId { slot: 1, generation: 0 }, motivating_need: Hunger }, ExploreLocation { target_place: EntityId { slot: 2, generation: 0 }, motivating_need: Dirtiness }, Relieve, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed] (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 19 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=175000, total=175000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=250, weight=700, score=175000, recovery_relevant=true); Thirst(pressure=93, weight=700, score=65100, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=186200, total=186200, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, replacement=SameGoalSiblingReplaced, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=266, weight=700, score=186200, recovery_relevant=true); Thirst(pressure=112, weight=700, score=78400, recovery_relevant=true)], feasibility=Likely; ... and 8 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=182, weight=700, score=127400, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely; ... and 8 more |
| 300–399 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=141400, total=141400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=202, weight=700, score=141400, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ... and 9 more |
| 400–499 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×8); ... and 20 more |
| 500–599 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×12); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, frame=[resumed]; ... and 11 more |
| 600–699 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=160300, total=160300, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=96, weight=700, score=67200, recovery_relevant=true); Thirst(pressure=229, weight=700, score=160300, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=46, weight=700, score=32200, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely; ... and 8 more |
| 700–799 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 11 more |
| 800–899 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Relieve@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 15 more |
| 900–999 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ACTIVE: travel — interrupt: NoInterrupt; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=68, weight=700, score=47600, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 12 more |
| 1000–1099 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ACTIVE: wash — interrupt: NoInterrupt (×11); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=2, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e1g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=198100, total=198100, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=2, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, stop=EncounteredDifferentGoal(Sleep), drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=283, weight=700, score=198100, recovery_relevant=true)], feasibility=Likely, ranking=MotiveScore ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none>Sleep@none; ... and 9 more |
| 1100–1199 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 12 more |
| 1200–1299 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Relieve@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ... and 14 more |
| 1300–1399 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=142800, total=142800, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=204, weight=700, score=142800, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=142800, total=142800, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=50, weight=700, score=35000, recovery_relevant=true); Thirst(pressure=204, weight=700, score=142800, recovery_relevant=true)], feasibility=Likely; ... and 10 more |
| 1400–1499 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=141400, total=141400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=64, weight=700, score=44800, recovery_relevant=true); Thirst(pressure=202, weight=700, score=141400, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=Relieve, selected_opportunity=Relieve@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Relieve, path=Relieve, primary=195000, total=195000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=from=EntityId { slot: 1, generation: 0 }@2, kept=[EntityId { slot: 0, generation: 0 }[base=3, threat=0, penalty=0, direct=3, remain=0, total=3],EntityId { slot: 2, generation: 0 }[base=2, threat=0, penalty=0, direct=2, remain=0, total=2]], pruned=[]], candidates=1, plans_found=1, same_goal=trigger=Relieve@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Bladder(pressure=300, weight=650, score=195000, recovery_relevant=false)], feasibility=Likely; PLAN (dirty: CLEAN): selected=Sleep, selected_opportunity=Sleep@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Sleep, path=Sleep, primary=195000, total=195000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=Sleep@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Fatigue(pressure=300, weight=650, score=195000, recovery_relevant=true)], feasibility=Likely (×3); ... and 2 more |

**Affordances available at tick 0** (at e0g0)

- sleep
- toilet
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 17, arrived at Fertile Fields)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 21, arrived at Riverside Camp)

- drink (1 targets)
- sleep
- toilet
- wash (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 24, arrived at Fertile Fields)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 451, arrived at Forest Clearing)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 563, arrived at Fertile Fields)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 989, arrived at Forest Clearing)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 1121, arrived at Fertile Fields)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordance changes** (tick 1): +ask_witness, +harvest:Harvest Water, +queue_for_facility_use
**Affordance changes** (tick 4): +collect_display_stock, +pick_up, +stage_stock_for_sale, +steal, +unstage_stock, -ask_witness
**Affordance changes** (tick 5): +drink, +drop_item, +put_down, +store_stock, +wash, -pick_up, -steal
**Affordance changes** (tick 14): +pick_up, +steal
**Affordance changes** (tick 17): +ask_witness, +bribe, +harvest:Harvest Apples, +relieve_wilderness, -harvest:Harvest Water, -pick_up, -steal, -toilet (at Fertile Fields)
**Affordance changes** (tick 21): +harvest:Harvest Water, +pick_up, +steal, +toilet, -ask_witness, -bribe, -harvest:Harvest Apples, -relieve_wilderness (at Riverside Camp)
**Affordance changes** (tick 24): +ask_witness, +bribe, +harvest:Harvest Apples, +relieve_wilderness, -harvest:Harvest Water, -toilet (at Fertile Fields)
**Affordance changes** (tick 28): +eat
**Affordance changes** (tick 32): -eat
**Affordance changes** (tick 99): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 132): +bribe
**Affordance changes** (tick 134): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 204): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 245): +bribe
**Affordance changes** (tick 256): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 283): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 324): +bribe
**Affordance changes** (tick 325): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 361): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 402): +bribe
**Affordance changes** (tick 404): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 440): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 451): +harvest:Harvest Water, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 481): +bribe
**Affordance changes** (tick 482): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 498): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 509): -ask_witness
**Affordance changes** (tick 563): +ask_witness, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 566): +bribe
**Affordance changes** (tick 568): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 605): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 655): +bribe
**Affordance changes** (tick 657): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 683): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 724): +bribe
**Affordance changes** (tick 726): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 763): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 804): +bribe
**Affordance changes** (tick 807): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 841): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 882): +bribe
**Affordance changes** (tick 884): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 924): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 935): -ask_witness
**Affordance changes** (tick 962): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 974): +ask_witness, +bribe
**Affordance changes** (tick 989): +harvest:Harvest Water, -ask_witness, -bribe, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 998): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1040): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1063): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 1121): +ask_witness, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 1125): +bribe
**Affordance changes** (tick 1127): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1170): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1211): +bribe
**Affordance changes** (tick 1213): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1248): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1289): +bribe
**Affordance changes** (tick 1300): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1328): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1369): +bribe
**Affordance changes** (tick 1370): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1406): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Final affordances** (tick 1439)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

### Agent B (1440 decision ticks)

**Tick breakdown**: 1176 planning, 264 active-action, 0 dead
**Plan search outcomes**: 253 found, 0 frontier-exhausted, 0 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Apple, purpose: SelfConsume }, AcquireCommodity { commodity: Bread, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Apple }, ConsumeOwnedCommodity { commodity: Water }, ExploreLocation { target_place: EntityId { slot: 1, generation: 0 }, motivating_need: Hunger }, ExploreLocation { target_place: EntityId { slot: 2, generation: 0 }, motivating_need: Dirtiness }, Relieve, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×4); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ACTIVE: travel — interrupt: NoInterrupt (×2); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=166500, total=166500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=222, weight=750, score=166500, recovery_relevant=true); Thirst(pressure=170, weight=700, score=119000, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0; ... and 13 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Relieve@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=117, weight=750, score=87750, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ... and 9 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=160300, total=160300, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=144, weight=750, score=108000, recovery_relevant=true); Thirst(pressure=229, weight=700, score=160300, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ... and 9 more |
| 300–399 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=117, weight=750, score=87750, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ... and 12 more |
| 400–499 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ACTIVE: travel — interrupt: NoInterrupt; ... and 15 more |
| 500–599 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 12 more |
| 600–699 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Relieve@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 9 more |
| 700–799 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=151200, total=151200, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=132, weight=750, score=99000, recovery_relevant=true); Thirst(pressure=216, weight=700, score=151200, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 18 more |
| 800–899 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×8); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=117, weight=750, score=87750, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 8 more |
| 900–999 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0 (×2); ... and 22 more |
| 1000–1099 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=133, weight=750, score=99750, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 7 more |
| 1100–1199 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=160300, total=160300, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=144, weight=750, score=108000, recovery_relevant=true); Thirst(pressure=229, weight=700, score=160300, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 12 more |
| 1200–1299 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×8); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=117, weight=750, score=87750, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=69, weight=750, score=51750, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely; ... and 8 more |
| 1300–1399 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ... and 10 more |
| 1400–1499 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ACTIVE: travel — interrupt: NoInterrupt; ACTIVE: wash — interrupt: NoInterrupt (×11); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=143500, total=143500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=2, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=205, weight=700, score=143500, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e1g0; ... and 12 more |

**Fully blocked desires** (goal generated but all opportunities blocked)

| Goal | Times Blocked |
|------|---------------|
| AcquireCommodity { commodity: Water, purpose: SelfConsume } | 1 |

**Affordances available at tick 0** (at e0g0)

- sleep
- toilet
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 5, arrived at Fertile Fields)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 439, arrived at Forest Clearing)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 510, arrived at Fertile Fields)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 936, arrived at Forest Clearing)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 982, arrived at Fertile Fields)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 1408, arrived at Forest Clearing)

- sleep
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordance changes** (tick 1): +ask_witness, +harvest:Harvest Water, +queue_for_facility_use
**Affordance changes** (tick 5): +harvest:Harvest Apples, +relieve_wilderness, -ask_witness, -harvest:Harvest Water, -toilet (at Fertile Fields)
**Affordance changes** (tick 8): +collect_display_stock, +pick_up, +stage_stock_for_sale, +steal, +unstage_stock
**Affordance changes** (tick 9): +drink, +drop_item, +eat, +put_down, +store_stock, -pick_up, -steal
**Affordance changes** (tick 13): +pick_up, +steal, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 21): +ask_witness
**Affordance changes** (tick 65): +bribe
**Affordance changes** (tick 66): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 113): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 154): +bribe
**Affordance changes** (tick 155): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 192): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 242): +bribe
**Affordance changes** (tick 244): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 270): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 311): +bribe
**Affordance changes** (tick 313): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 349): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 390): +bribe
**Affordance changes** (tick 392): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 437): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 439): +harvest:Harvest Water, -ask_witness, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 451): +ask_witness
**Affordance changes** (tick 469): +bribe
**Affordance changes** (tick 470): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 485): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 510): +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 513): +bribe
**Affordance changes** (tick 515): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 581): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 626): +bribe
**Affordance changes** (tick 628): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 664): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 714): +bribe
**Affordance changes** (tick 716): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 743): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 784): +bribe
**Affordance changes** (tick 788): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 821): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 862): +bribe
**Affordance changes** (tick 864): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 909): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 936): +harvest:Harvest Water, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 941): +bribe
**Affordance changes** (tick 942): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 958): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 973): -ask_witness
**Affordance changes** (tick 982): +ask_witness, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 985): +bribe
**Affordance changes** (tick 987): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1053): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1098): +bribe
**Affordance changes** (tick 1100): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1136): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1186): +bribe
**Affordance changes** (tick 1188): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1214): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1255): +bribe
**Affordance changes** (tick 1257): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1293): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1334): +bribe
**Affordance changes** (tick 1336): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1381): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1408): -ask_witness, -harvest:Harvest Apples, -queue_for_facility_use (at Forest Clearing)
**Affordance changes** (tick 1409): +harvest:Harvest Water, +queue_for_facility_use
**Affordance changes** (tick 1415): +drop_item, +put_down, +store_stock
**Affordance changes** (tick 1417): +drink, +wash
**Affordance changes** (tick 1432): -drink, -wash
**Final affordances** (tick 1432)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

### Agent C (1440 decision ticks)

**Tick breakdown**: 1148 planning, 292 active-action, 0 dead
**Plan search outcomes**: 272 found, 0 frontier-exhausted, 0 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Apple, purpose: SelfConsume }, AcquireCommodity { commodity: Bread, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Apple }, ConsumeOwnedCommodity { commodity: Water }, ExploreLocation { target_place: EntityId { slot: 1, generation: 0 }, motivating_need: Hunger }, ExploreLocation { target_place: EntityId { slot: 2, generation: 0 }, motivating_need: Dirtiness }, Relieve, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×17); ACTIVE: travel — interrupt: NoInterrupt; ... and 15 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×11); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=133400, total=133400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=184, weight=725, score=133400, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 15 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 11 more |
| 300–399 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: travel — interrupt: NoInterrupt (×2); ... and 21 more |
| 400–499 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=179200, total=179200, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=256, weight=700, score=179200, recovery_relevant=true); Thirst(pressure=80, weight=725, score=58000, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=180600, total=180600, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]), replacement=SameGoalSiblingReplaced, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=258, weight=700, score=180600, recovery_relevant=true); Thirst(pressure=84, weight=725, score=60900, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0; ... and 10 more |
| 500–599 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×4); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 12 more |
| 600–699 | ACTIVE: eat — interrupt: NoInterrupt (×4); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=28, weight=700, score=19600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely; ... and 10 more |
| 700–799 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×3); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ... and 13 more |
| 800–899 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 17 more |
| 900–999 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 19 more |
| 1000–1099 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=173600, total=173600, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=248, weight=700, score=173600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=153700, total=153700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=2, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, stop=EncounteredDifferentGoal(ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]), drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=212, weight=725, score=153700, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; PLAN (dirty: CLEAN): selected=Relieve, selected_opportunity=Relieve@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Relieve, path=Relieve, primary=195000, total=195000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=from=EntityId { slot: 1, generation: 0 }@2, kept=[EntityId { slot: 0, generation: 0 }[base=3, threat=0, penalty=0, direct=3, remain=0, total=3],EntityId { slot: 2, generation: 0 }[base=2, threat=0, penalty=0, direct=2, remain=0, total=2]], pruned=[]], candidates=1, plans_found=1, same_goal=trigger=Relieve@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Bladder(pressure=300, weight=650, score=195000, recovery_relevant=false)], feasibility=Likely (×2); ... and 3 more |
| 1100–1199 | ACTIVE: eat — interrupt: NoInterrupt (×4); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 14 more |
| 1200–1299 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 19 more |
| 1300–1399 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ... and 21 more |
| 1400–1499 | ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=2, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, stop=EncounteredDifferentGoal(ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]), drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; PLAN (dirty: CLEAN): selected=Relieve, selected_opportunity=Relieve@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Relieve, path=Relieve, primary=338000, total=338000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=from=EntityId { slot: 1, generation: 0 }@2, kept=[EntityId { slot: 0, generation: 0 }[base=3, threat=0, penalty=0, direct=3, remain=0, total=3],EntityId { slot: 2, generation: 0 }[base=2, threat=0, penalty=0, direct=2, remain=0, total=2]], pruned=[]], candidates=2, plans_found=1, same_goal=trigger=Relieve@none, stop=EncounteredDifferentGoal(Sleep), drive=base=Low final=Low adjustment=none motive_inputs=[Bladder(pressure=520, weight=650, score=338000, recovery_relevant=false)], feasibility=Likely, ranking=MotiveScore Relieve@none>Sleep@none; PLAN (dirty: CLEAN): selected=Sleep, selected_opportunity=Sleep@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Sleep, path=Sleep, primary=195000, total=195000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=Sleep@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Fatigue(pressure=300, weight=650, score=195000, recovery_relevant=true)], feasibility=Likely (×3); PLAN (dirty: CLEAN): selected=Sleep, selected_opportunity=Sleep@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Sleep, path=Sleep, primary=205400, total=205400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=Sleep@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Fatigue(pressure=316, weight=650, score=205400, recovery_relevant=true)], feasibility=Likely; ... and 1 more |

**Affordances available at tick 0** (at e2g0)

- sleep
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 16, arrived at Fertile Fields)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 311, arrived at Forest Clearing)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 400, arrived at Fertile Fields)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 832, arrived at Forest Clearing)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 974, arrived at Fertile Fields)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 1289, arrived at Forest Clearing)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 1378, arrived at Fertile Fields)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordance changes** (tick 1): +harvest:Harvest Water, +queue_for_facility_use
**Affordance changes** (tick 4): +collect_display_stock, +pick_up, +stage_stock_for_sale, +steal, +unstage_stock
**Affordance changes** (tick 5): +drink, +drop_item, +put_down, +store_stock, +wash, -pick_up, -steal
**Affordance changes** (tick 16): +ask_witness, +bribe, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 19): +pick_up, +steal
**Affordance changes** (tick 20): +eat, -pick_up, -steal
**Affordance changes** (tick 22): +pick_up, +steal
**Affordance changes** (tick 41): -eat
**Affordance changes** (tick 87): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 135): +bribe
**Affordance changes** (tick 137): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 163): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 194): +bribe
**Affordance changes** (tick 195): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 221): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 252): +bribe
**Affordance changes** (tick 254): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 280): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 311): +harvest:Harvest Water, -ask_witness, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 316): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 331): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 380): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 400): +ask_witness, +bribe, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 407): +eat
**Affordance changes** (tick 450): -ask_witness, -bribe
**Affordance changes** (tick 454): -wash
**Affordance changes** (tick 500): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 510): +ask_witness
**Affordance changes** (tick 537): +bribe
**Affordance changes** (tick 539): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 559): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 590): +bribe
**Affordance changes** (tick 601): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 618): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 649): +bribe
**Affordance changes** (tick 650): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 677): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 708): +bribe
**Affordance changes** (tick 710): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 736): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 767): +bribe
**Affordance changes** (tick 769): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 795): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 826): +bribe
**Affordance changes** (tick 828): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 832): +harvest:Harvest Water, -ask_witness, -bribe, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 854): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 886): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 902): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 936): +ask_witness
**Affordance changes** (tick 949): +bribe
**Affordance changes** (tick 951): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 974): +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 979): +eat
**Affordance changes** (tick 1034): -wash
**Affordance changes** (tick 1080): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1111): +bribe
**Affordance changes** (tick 1113): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1139): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1170): +bribe
**Affordance changes** (tick 1172): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1198): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1229): +bribe
**Affordance changes** (tick 1231): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1258): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1289): +harvest:Harvest Water, -ask_witness, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 1293): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1308): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 1357): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1378): +ask_witness, +bribe, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 1383): +eat
**Affordance changes** (tick 1430): -wash
**Final affordances** (tick 1439)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Apples (1 targets)
- staff_market
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

## Section 8 — Budget Exhaustion Snapshots

No budget exhaustion events detected.

