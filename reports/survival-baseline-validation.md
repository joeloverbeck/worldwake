# Simulation Observer Dump

## Section 1 — Run Metadata

- **Scenario**: `scenarios/survival-baseline.ron`
- **Seed**: 104004
- **Ticks simulated**: 1440
- **Total events**: 43173

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

**Actions** (total lifecycle events: 500)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 8 | 8 | 0 | 0 |
| eat | 24 | 24 | 0 | 0 |
| harvest:Harvest Apples | 12 | 12 | 0 | 2 |
| harvest:Harvest Water | 6 | 6 | 0 | 0 |
| pick_up | 18 | 18 | 0 | 0 |
| relieve_wilderness | 17 | 17 | 0 | 0 |
| sleep | 144 | 144 | 0 | 0 |
| toilet | 7 | 7 | 0 | 0 |
| travel | 10 | 10 | 0 | 0 |
| wash | 3 | 3 | 0 | 0 |

**Perception**: 168 total observations, 144 passed, 43 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 2 | 490 | 106 |
| Thirst | 3 | 296 | 137 |
| Fatigue | 122 | 334 | 289 |
| Bladder | 4 | 516 | 168 |
| Dirtiness | 1 | 469 | 214 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Behavioral transition** at tick 900: action repertoire narrowed (9 types -> 4 types)
  Needs: hunger=214, thirst=195, fatigue=282, bladder=12, dirtiness=162

**Behavioral transition** at tick 1200: action repertoire narrowed (10 types -> 5 types)
  Needs: hunger=94, thirst=130, fatigue=292, bladder=8, dirtiness=87

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 295 |
| e1g0 | 1125 |

**Max consecutive idle ticks**: 36

### Agent B

**Actions** (total lifecycle events: 516)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| ask_witness | 1 | 0 | 0 | 0 |
| drink | 3 | 3 | 0 | 0 |
| eat | 32 | 32 | 0 | 0 |
| harvest:Harvest Apples | 16 | 16 | 0 | 0 |
| harvest:Harvest Water | 3 | 3 | 0 | 1 |
| pick_up | 19 | 19 | 0 | 0 |
| relieve_wilderness | 19 | 19 | 0 | 0 |
| sleep | 146 | 146 | 0 | 0 |
| toilet | 3 | 3 | 0 | 0 |
| travel | 13 | 13 | 0 | 0 |
| wash | 3 | 3 | 0 | 0 |

**Perception**: 179 total observations, 156 passed, 45 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 3 | 506 | 92 |
| Thirst | 3 | 261 | 143 |
| Fatigue | 142 | 338 | 289 |
| Bladder | 4 | 512 | 172 |
| Dirtiness | 1 | 485 | 236 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Behavioral transition** at tick 1300: action repertoire narrowed (10 types -> 4 types)
  Needs: hunger=65, thirst=33, fatigue=282, bladder=160, dirtiness=43

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 85 |
| e1g0 | 1175 |
| e2g0 | 97 |
| e3g0 | 53 |

**Max consecutive idle ticks**: 41

### Agent C

**Actions** (total lifecycle events: 538)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 13 | 13 | 0 | 0 |
| eat | 27 | 27 | 0 | 0 |
| harvest:Harvest Apples | 14 | 14 | 0 | 2 |
| harvest:Harvest Water | 8 | 8 | 0 | 0 |
| pick_up | 22 | 22 | 0 | 0 |
| relieve_wilderness | 26 | 26 | 0 | 0 |
| sleep | 146 | 146 | 0 | 0 |
| travel | 9 | 9 | 0 | 0 |
| wash | 3 | 3 | 0 | 0 |

**Perception**: 192 total observations, 169 passed, 54 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 2 | 434 | 111 |
| Thirst | 4 | 284 | 117 |
| Fatigue | 112 | 328 | 288 |
| Bladder | 4 | 528 | 170 |
| Dirtiness | 1 | 448 | 211 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Locations visited**

| Place | Ticks |
|-------|-------|
| e1g0 | 1075 |
| e2g0 | 356 |

**Max consecutive idle ticks**: 27

## Section 3 — Anomaly Flags

20 anomalies detected:

### Anomaly 1 — REDUNDANT_PERCEPTION (Agent A)

Observed entity e1g0 37 times (may indicate redundant perception if entity state unchanged)

### Anomaly 2 — REDUNDANT_PERCEPTION (Agent A)

Observed entity e4g0 30 times (may indicate redundant perception if entity state unchanged)

### Anomaly 3 — REDUNDANT_PERCEPTION (Agent A)

Observed entity e5g0 23 times (may indicate redundant perception if entity state unchanged)

### Anomaly 4 — REDUNDANT_PERCEPTION (Agent A)

Observed entity e6g0 24 times (may indicate redundant perception if entity state unchanged)

### Anomaly 5 — REDUNDANT_PERCEPTION (Agent A)

Observed entity e7g0 10 times (may indicate redundant perception if entity state unchanged)

### Anomaly 6 — REDUNDANT_PERCEPTION (Agent A)

Observed entity e9g0 57 times (may indicate redundant perception if entity state unchanged)

### Anomaly 7 — STUCK_AGENT (Agent A)

No actions for 36 consecutive ticks

### Anomaly 8 — REDUNDANT_PERCEPTION (Agent B)

Observed entity e1g0 35 times (may indicate redundant perception if entity state unchanged)

### Anomaly 9 — REDUNDANT_PERCEPTION (Agent B)

Observed entity e4g0 20 times (may indicate redundant perception if entity state unchanged)

### Anomaly 10 — REDUNDANT_PERCEPTION (Agent B)

Observed entity e5g0 37 times (may indicate redundant perception if entity state unchanged)

### Anomaly 11 — REDUNDANT_PERCEPTION (Agent B)

Observed entity e6g0 25 times (may indicate redundant perception if entity state unchanged)

### Anomaly 12 — REDUNDANT_PERCEPTION (Agent B)

Observed entity e9g0 67 times (may indicate redundant perception if entity state unchanged)

### Anomaly 13 — STUCK_AGENT (Agent B)

No actions for 41 consecutive ticks

### Anomaly 14 — REDUNDANT_PERCEPTION (Agent C)

Observed entity e1g0 39 times (may indicate redundant perception if entity state unchanged)

### Anomaly 15 — REDUNDANT_PERCEPTION (Agent C)

Observed entity e4g0 25 times (may indicate redundant perception if entity state unchanged)

### Anomaly 16 — REDUNDANT_PERCEPTION (Agent C)

Observed entity e5g0 23 times (may indicate redundant perception if entity state unchanged)

### Anomaly 17 — REDUNDANT_PERCEPTION (Agent C)

Observed entity e6g0 52 times (may indicate redundant perception if entity state unchanged)

### Anomaly 18 — REDUNDANT_PERCEPTION (Agent C)

Observed entity e8g0 13 times (may indicate redundant perception if entity state unchanged)

### Anomaly 19 — REDUNDANT_PERCEPTION (Agent C)

Observed entity e9g0 58 times (may indicate redundant perception if entity state unchanged)

### Anomaly 20 — STUCK_AGENT (Agent C)

No actions for 27 consecutive ticks

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
[19] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[20] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("travel") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted, Travel} deltas=0
[21] tick=2 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[22] tick=2 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[23] tick=2 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[24] tick=2 actor=None action=None place=None tags={System} deltas=0
[25] tick=3 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[26] tick=3 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[27] tick=3 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[28] tick=3 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[29] tick=3 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=0
[30] tick=3 actor=None action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, System} deltas=0
[31] tick=3 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[32] tick=3 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[33] tick=3 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[34] tick=3 actor=None action=None place=None tags={System} deltas=0
[35] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[36] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[37] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[38] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[39] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[40] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[41] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[42] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[43] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("travel") place=None tags={ActionCommitted, Travel} deltas=0
[44] tick=4 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[45] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[46] tick=4 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[47] tick=4 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[48] tick=4 actor=None action=None place=None tags={System} deltas=0
[49] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[50] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[51] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[52] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[53] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[54] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[55] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[56] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[57] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[58] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("harvest:Harvest Apples") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[59] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[60] tick=5 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[61] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[62] tick=5 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[63] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[64] tick=5 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[65] tick=5 actor=None action=None place=None tags={System} deltas=0
[66] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[67] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=0
[68] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[69] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[70] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("toilet") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[71] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("relieve_wilderness") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[72] tick=6 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[73] tick=6 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=0
[74] tick=6 actor=None action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, System} deltas=0
[75] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[76] tick=6 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[77] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[78] tick=6 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[79] tick=6 actor=None action=None place=None tags={System} deltas=0
[80] tick=7 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("harvest:Harvest Apples") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[81] tick=7 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[82] tick=7 actor=None action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, System} deltas=0
[83] tick=7 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[84] tick=7 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[85] tick=7 actor=None action=None place=None tags={System} deltas=0
[86] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[87] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[88] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[89] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[90] tick=8 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[91] tick=8 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[92] tick=8 actor=None action=None place=None tags={System} deltas=0
[93] tick=9 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[94] tick=9 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[95] tick=9 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[96] tick=9 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[97] tick=9 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=0
[98] tick=9 actor=None action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, System} deltas=0
[99] tick=9 actor=None action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, System} deltas=0
```

### Last 100 events

```
[43073] tick=1436 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43074] tick=1436 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43075] tick=1436 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43076] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43077] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43078] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43079] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43080] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43081] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43082] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43083] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43084] tick=1436 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43085] tick=1436 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[43086] tick=1436 actor=None action=None place=None tags={System} deltas=0
[43087] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=1
[43088] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=5
[43089] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=9
[43090] tick=1437 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[43091] tick=1437 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43092] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43093] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43094] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43095] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43096] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43097] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43098] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43099] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43100] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43101] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43102] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43103] tick=1437 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43104] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43105] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43106] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43107] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43108] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43109] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43110] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43111] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43112] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43113] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43114] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43115] tick=1437 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43116] tick=1437 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[43117] tick=1437 actor=None action=None place=None tags={System} deltas=0
[43118] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=1
[43119] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=1
[43120] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[43121] tick=1438 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[43122] tick=1438 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43123] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43124] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43125] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43126] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43127] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43128] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43129] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43130] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43131] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43132] tick=1438 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43133] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43134] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43135] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43136] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43137] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43138] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43139] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43140] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43141] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43142] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43143] tick=1438 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43144] tick=1438 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[43145] tick=1438 actor=None action=None place=None tags={System} deltas=0
[43146] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=None tags={} deltas=1
[43147] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=Some("toilet") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=6
[43148] tick=1439 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[43149] tick=1439 actor=Some(EntityId { slot: 4, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43150] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43151] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43152] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43153] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43154] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43155] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43156] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43157] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43158] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43159] tick=1439 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43160] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43161] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43162] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43163] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43164] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43165] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43166] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43167] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43168] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43169] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43170] tick=1439 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[43171] tick=1439 actor=None action=None place=None tags={WorldMutation, System} deltas=3
[43172] tick=1439 actor=None action=None place=None tags={System} deltas=0
```

### Action Trace Summary

Total action trace events: 1554

#### Per-Agent Action Timeline (100-tick bins)

**Agent A (e4g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | travel×3, drink×2, eat×2, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2, sleep×2, harvest:Harvest Water×1, toilet×1 |
| 100–199 | sleep×11, eat×1, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 200–299 | sleep×10, eat×3, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 300–399 | sleep×11, drink×1, harvest:Harvest Water×1, pick_up×1, relieve_wilderness×1, toilet×1, travel×1, wash×1 |
| 400–499 | sleep×11, eat×2, harvest:Harvest Apples×2, relieve_wilderness×2, pick_up×1, travel×1 |
| 500–599 | sleep×10, eat×3, pick_up×2, harvest:Harvest Apples×1, relieve_wilderness×1 |
| 600–699 | sleep×11, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 700–799 | sleep×10, drink×1, harvest:Harvest Water×1, pick_up×1, toilet×1, travel×1, wash×1 |
| 800–899 | sleep×11, pick_up×2, drink×1, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, relieve_wilderness×1, toilet×1, travel×1 |
| 900–999 | sleep×10, drink×1, eat×1, relieve_wilderness×1 |
| 1000–1099 | sleep×10, eat×2, harvest:Harvest Apples×2, pick_up×1, relieve_wilderness×1, toilet×1, travel×1 |
| 1100–1199 | sleep×11, pick_up×2, drink×1, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, relieve_wilderness×1, toilet×1, travel×1, wash×1 |
| 1200–1299 | sleep×11, eat×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 1300–1399 | sleep×11, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 1400–1499 | sleep×4, drink×1, harvest:Harvest Water×1, pick_up×1, toilet×1, travel×1 |

**Agent B (e5g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | eat×3, sleep×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×2, harvest:Harvest Water×1, travel×1 |
| 100–199 | sleep×11, eat×3, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 200–299 | sleep×10, travel×3, eat×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1, toilet×1 |
| 300–399 | sleep×11, pick_up×2, drink×1, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, toilet×1, travel×1, wash×1 |
| 400–499 | sleep×11, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 500–599 | sleep×10, eat×2, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 600–699 | sleep×11, eat×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×1 |
| 700–799 | sleep×11, relieve_wilderness×2, drink×1, eat×1, harvest:Harvest Water×1, pick_up×1, travel×1, wash×1 |
| 800–899 | sleep×10, eat×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1, travel×1 |
| 900–999 | sleep×11, eat×2, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 1000–1099 | sleep×11, eat×3, harvest:Harvest Apples×2, pick_up×2, travel×2, ask_witness×1, relieve_wilderness×1 |
| 1100–1199 | sleep×10, eat×2, travel×2, harvest:Harvest Apples×1, pick_up×1, relieve_wilderness×1 |
| 1200–1299 | sleep×11, eat×2, pick_up×2, travel×2, drink×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, relieve_wilderness×1, toilet×1, wash×1 |
| 1300–1399 | sleep×10, eat×1, harvest:Harvest Apples×1, relieve_wilderness×1 |
| 1400–1499 | sleep×5, eat×2, pick_up×1, relieve_wilderness×1 |

**Agent C (e6g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | relieve_wilderness×3, drink×2, eat×2, pick_up×2, sleep×2, harvest:Harvest Apples×1, harvest:Harvest Water×1, travel×1 |
| 100–199 | sleep×11, eat×3, harvest:Harvest Apples×3, pick_up×2, relieve_wilderness×1 |
| 200–299 | sleep×10, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 300–399 | sleep×11, drink×2, harvest:Harvest Water×2, pick_up×2, relieve_wilderness×2, travel×2, wash×1 |
| 400–499 | sleep×11, relieve_wilderness×2, drink×1, eat×1, harvest:Harvest Apples×1, pick_up×1 |
| 500–599 | sleep×11, eat×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×1 |
| 600–699 | sleep×10, eat×2, relieve_wilderness×2, drink×1, harvest:Harvest Water×1, pick_up×1, travel×1, wash×1 |
| 700–799 | sleep×11, drink×2, pick_up×2, relieve_wilderness×2, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, travel×1 |
| 800–899 | sleep×11, eat×3, relieve_wilderness×2, harvest:Harvest Apples×1, pick_up×1 |
| 900–999 | sleep×11, eat×3, harvest:Harvest Apples×2, pick_up×2, relieve_wilderness×1 |
| 1000–1099 | sleep×10, relieve_wilderness×2, drink×1, eat×1, harvest:Harvest Water×1, pick_up×1, travel×1, wash×1 |
| 1100–1199 | sleep×11, drink×2, pick_up×2, relieve_wilderness×2, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, travel×1 |
| 1200–1299 | sleep×11, relieve_wilderness×2, drink×1, eat×1, harvest:Harvest Apples×1, harvest:Harvest Water×1, pick_up×1, travel×1 |
| 1300–1399 | sleep×10, relieve_wilderness×2, drink×1, eat×1, harvest:Harvest Apples×1, pick_up×1, travel×1 |
| 1400–1499 | sleep×5, eat×2, harvest:Harvest Apples×1, pick_up×1 |

#### Raw Action Trace (last 50 events)

```
tick 1403 seq 0: e5g0 committed 'eat' (instance ai750, 0 materializations)
tick 1404 seq 0: e5g0 started 'relieve_wilderness' targeting []
tick 1405 seq 0: e4g0 started 'sleep' targeting []
tick 1405 seq 1: e4g0 committed 'sleep' (instance ai752, 0 materializations)
tick 1406 seq 0: e6g0 committed 'relieve_wilderness' (instance ai747, 0 materializations)
tick 1407 seq 0: e6g0 started 'sleep' targeting []
tick 1407 seq 1: e6g0 committed 'sleep' (instance ai753, 0 materializations)
tick 1408 seq 0: e6g0 started 'eat' targeting [EntityId { slot: 188, generation: 0 }]
tick 1409 seq 0: e6g0 committed 'eat' (instance ai754, 0 materializations)
tick 1410 seq 0: e6g0 started 'sleep' targeting []
tick 1410 seq 1: e6g0 committed 'sleep' (instance ai755, 0 materializations)
tick 1411 seq 0: e5g0 committed 'relieve_wilderness' (instance ai751, 0 materializations)
tick 1412 seq 0: e5g0 started 'sleep' targeting []
tick 1412 seq 1: e5g0 committed 'sleep' (instance ai756, 0 materializations)
tick 1415 seq 0: e4g0 started 'sleep' targeting []
tick 1415 seq 1: e5g0 started 'sleep' targeting []
tick 1415 seq 2: e4g0 committed 'sleep' (instance ai757, 0 materializations)
tick 1415 seq 3: e5g0 committed 'sleep' (instance ai758, 0 materializations)
tick 1420 seq 0: e6g0 started 'sleep' targeting []
tick 1420 seq 1: e6g0 committed 'sleep' (instance ai759, 0 materializations)
tick 1422 seq 0: e4g0 started 'travel' targeting [EntityId { slot: 0, generation: 0 }]
tick 1424 seq 0: e4g0 committed 'travel' (instance ai760, 0 materializations)
tick 1425 seq 0: e4g0 started 'sleep' targeting []
tick 1425 seq 1: e5g0 started 'sleep' targeting []
tick 1425 seq 2: e4g0 committed 'sleep' (instance ai761, 0 materializations)
tick 1425 seq 3: e5g0 committed 'sleep' (instance ai762, 0 materializations)
tick 1426 seq 0: e4g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 7, generation: 0 }]
tick 1428 seq 0: e4g0 committed 'harvest:Harvest Water' (instance ai763, 0 materializations)
tick 1429 seq 0: e4g0 started 'pick_up' targeting [EntityId { slot: 195, generation: 0 }]
tick 1429 seq 1: e4g0 committed 'pick_up' (instance ai764, 0 materializations)
tick 1430 seq 0: e4g0 started 'sleep' targeting []
tick 1430 seq 1: e6g0 started 'sleep' targeting []
tick 1430 seq 2: e4g0 committed 'sleep' (instance ai765, 0 materializations)
tick 1430 seq 3: e6g0 committed 'sleep' (instance ai766, 0 materializations)
tick 1431 seq 0: e4g0 started 'drink' targeting [EntityId { slot: 195, generation: 0 }]
tick 1431 seq 1: e6g0 started 'harvest:Harvest Apples' targeting [EntityId { slot: 9, generation: 0 }]
tick 1431 seq 2: e4g0 committed 'drink' (instance ai767, 0 materializations)
tick 1432 seq 0: e4g0 started 'toilet' targeting []
tick 1433 seq 0: e6g0 committed 'harvest:Harvest Apples' (instance ai768, 0 materializations)
tick 1434 seq 0: e6g0 started 'pick_up' targeting [EntityId { slot: 197, generation: 0 }]
tick 1434 seq 1: e6g0 committed 'pick_up' (instance ai770, 0 materializations)
tick 1435 seq 0: e5g0 started 'sleep' targeting []
tick 1435 seq 1: e6g0 started 'sleep' targeting []
tick 1435 seq 2: e5g0 committed 'sleep' (instance ai771, 0 materializations)
tick 1435 seq 3: e6g0 committed 'sleep' (instance ai772, 0 materializations)
tick 1436 seq 0: e5g0 started 'eat' targeting [EntityId { slot: 191, generation: 0 }]
tick 1436 seq 1: e6g0 started 'eat' targeting [EntityId { slot: 197, generation: 0 }]
tick 1437 seq 0: e5g0 committed 'eat' (instance ai773, 0 materializations)
tick 1437 seq 1: e6g0 committed 'eat' (instance ai774, 0 materializations)
tick 1439 seq 0: e4g0 committed 'toilet' (instance ai769, 0 materializations)
```

### Perception Trace Summary

Total perception trace events: 539

**Agent A (e4g0)** — 168 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 13 | 2 | 9 |
| 100–199 | 13 | 3 | 8 |
| 200–299 | 12 | 2 | 9 |
| 300–399 | 1 | 3 | 1 |
| 400–499 | 15 | 1 | 10 |
| 500–599 | 14 | 2 | 8 |
| 600–699 | 13 | 2 | 8 |
| 700–799 | 1 | 1 | 1 |
| 800–899 | 11 | 1 | 9 |
| 900–999 | 11 | 2 | 7 |
| 1000–1099 | 8 | 3 | 8 |
| 1100–1199 | 7 | 1 | 6 |
| 1200–1299 | 7 | 1 | 5 |
| 1300–1399 | 12 | 0 | 8 |
| 1400–1499 | 6 | 0 | 7 |

**Agent B (e5g0)** — 179 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 18 | 1 | 9 |
| 100–199 | 14 | 2 | 9 |
| 200–299 | 7 | 0 | 6 |
| 300–399 | 6 | 0 | 2 |
| 400–499 | 17 | 2 | 10 |
| 500–599 | 14 | 2 | 7 |
| 600–699 | 11 | 4 | 8 |
| 700–799 | 6 | 2 | 5 |
| 800–899 | 11 | 2 | 8 |
| 900–999 | 10 | 3 | 7 |
| 1000–1099 | 14 | 1 | 9 |
| 1100–1199 | 5 | 1 | 4 |
| 1200–1299 | 7 | 1 | 5 |
| 1300–1399 | 11 | 1 | 8 |
| 1400–1499 | 5 | 1 | 6 |

**Agent C (e6g0)** — 192 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 18 | 2 | 10 |
| 100–199 | 13 | 3 | 9 |
| 200–299 | 12 | 2 | 9 |
| 300–399 | 8 | 0 | 5 |
| 400–499 | 17 | 2 | 11 |
| 500–599 | 11 | 5 | 7 |
| 600–699 | 11 | 2 | 10 |
| 700–799 | 7 | 1 | 7 |
| 800–899 | 13 | 1 | 9 |
| 900–999 | 13 | 0 | 8 |
| 1000–1099 | 12 | 3 | 10 |
| 1100–1199 | 13 | 1 | 9 |
| 1200–1299 | 8 | 0 | 8 |
| 1300–1399 | 8 | 0 | 7 |
| 1400–1499 | 5 | 1 | 5 |

#### Raw Perception Trace (last 50 events)

```
tick 1326 seq 0: e4g0 observed ev39691 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1326 seq 1: e5g0 observed ev39691 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1327 seq 0: e6g0 observed ev39717 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1333 seq 0: e4g0 observed ev39861 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1333 seq 1: e5g0 observed ev39861 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1334 seq 0: e6g0 observed ev39889 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1336 seq 0: e4g0 observed ev39943 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1336 seq 1: e5g0 observed ev39943 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1343 seq 0: e4g0 observed ev40118 (passed @ 720‰), 3 entities, 0 institutional claims
tick 1343 seq 1: e5g0 observed ev40118 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1345 seq 0: e4g0 observed ev40171 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1345 seq 1: e5g0 observed ev40171 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1347 seq 0: e4g0 observed ev40229 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1347 seq 1: e5g0 observed ev40229 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1354 seq 0: e4g0 observed ev40444 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1354 seq 1: e5g0 observed ev40444 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1354 seq 2: e6g0 observed ev40444 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1356 seq 0: e4g0 observed ev40533 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1356 seq 1: e5g0 observed ev40533 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1356 seq 2: e6g0 observed ev40533 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1388 seq 0: e4g0 observed ev41583 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1388 seq 1: e5g0 observed ev41583 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1388 seq 2: e6g0 observed ev41583 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1395 seq 0: e4g0 observed ev41809 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1395 seq 1: e5g0 observed ev41809 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1395 seq 2: e6g0 observed ev41809 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1399 seq 0: e4g0 observed ev41932 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1399 seq 1: e5g0 observed ev41932 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1399 seq 2: e6g0 observed ev41932 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1399 seq 3: e4g0 observed ev41935 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1399 seq 4: e5g0 observed ev41935 (FAILED @ 900‰), 0 entities, 0 institutional claims
tick 1399 seq 5: e6g0 observed ev41935 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1401 seq 0: e4g0 observed ev42004 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1401 seq 1: e5g0 observed ev42004 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1401 seq 2: e6g0 observed ev42004 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1404 seq 0: e4g0 observed ev42100 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1404 seq 1: e5g0 observed ev42100 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1404 seq 2: e6g0 observed ev42100 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1406 seq 0: e4g0 observed ev42159 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1406 seq 1: e5g0 observed ev42159 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1406 seq 2: e6g0 observed ev42159 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1411 seq 0: e4g0 observed ev42319 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1411 seq 1: e5g0 observed ev42319 (passed @ 900‰), 3 entities, 0 institutional claims
tick 1411 seq 2: e6g0 observed ev42319 (FAILED @ 900‰), 0 entities, 0 institutional claims
tick 1428 seq 0: e4g0 observed ev42829 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1431 seq 0: e4g0 observed ev42919 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1433 seq 0: e5g0 observed ev42971 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1433 seq 1: e6g0 observed ev42971 (passed @ 900‰), 1 entities, 0 institutional claims
tick 1435 seq 0: e5g0 observed ev43033 (FAILED @ 900‰), 0 entities, 0 institutional claims
tick 1435 seq 1: e6g0 observed ev43033 (passed @ 900‰), 1 entities, 0 institutional claims
```

## Section 5 — Per-Agent Belief Summary

### Agent A

**Known entities**: 80
- Agents: 3
- Places: 2
- Items: 73
- Other: 2

**Believed entity locations**:
- (place entity — no parent location): Riverside Camp, Fertile Fields
- Riverside Camp: Well, ItemLot#155, 9× Waste, 1× Water
- Fertile Fields: Agent A, Agent B, Agent C, OrchardRow, ItemLot#139, ItemLot#143, ItemLot#144, ItemLot#156, ItemLot#162, ItemLot#166, ItemLot#176, ItemLot#180, ItemLot#186, ItemLot#188, ItemLot#191, 51× Waste

**Social observations**: 0
**Told beliefs**: 0
**Heard beliefs**: 0
**Institutional beliefs**: 0

### Agent B

**Known entities**: 94
- Agents: 3
- Places: 4
- Items: 84
- Other: 3

**Believed entity locations**:
- (place entity — no parent location): Riverside Camp, Fertile Fields, Forest Clearing, Hillside Shelter
- Riverside Camp: Agent A, Well, 8× Waste
- Fertile Fields: Agent B, Agent C, OrchardRow, ItemLot#139, ItemLot#143, ItemLot#144, ItemLot#152, ItemLot#156, ItemLot#162, ItemLot#176, ItemLot#180, ItemLot#186, ItemLot#188, ItemLot#191, 1× Apple, 51× Waste
- Forest Clearing: Well, ItemLot#172, ItemLot#177, 9× Waste
- Hillside Shelter: ItemLot#164, 1× Waste

**Social observations**: 0
**Told beliefs**: 0
**Heard beliefs**: 0
**Institutional beliefs**: 0

### Agent C

**Known entities**: 83
- Agents: 3
- Places: 2
- Items: 76
- Other: 2

**Believed entity locations**:
- (place entity — no parent location): Fertile Fields, Forest Clearing
- Riverside Camp: Agent A
- Fertile Fields: Agent B, Agent C, OrchardRow, ItemLot#139, ItemLot#143, ItemLot#144, ItemLot#156, ItemLot#162, ItemLot#164, ItemLot#166, ItemLot#180, ItemLot#186, ItemLot#188, ItemLot#191, 1× Apple, 51× Waste
- Forest Clearing: Well, ItemLot#148, ItemLot#177, 11× Waste

**Social observations**: 0
**Told beliefs**: 0
**Heard beliefs**: 0
**Institutional beliefs**: 0

## Section 6 — End-State Inventory & Resources

### Agent Inventories

**Agent A**: 1× Water

**Agent B**: (empty)

**Agent C**: 1× Apple

### Place Contents

**Riverside Camp (e0g0)**: Agent A (agent), Well (Well), 9× Waste, 1× Water

**Fertile Fields (e1g0)**: Agent B (agent), Agent C (agent), OrchardRow (OrchardRow), 1× Apple, 51× Waste

**Forest Clearing (e2g0)**: Well (Well), 11× Waste

**Hillside Shelter (e3g0)**: 1× Waste

## Section 7 — Per-Agent Decision Summary

### Agent A (1440 decision ticks)

**Tick breakdown**: 1159 planning, 281 active-action, 0 dead
**Plan search outcomes**: 248 found, 5 frontier-exhausted, 5 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Apple, purpose: SelfConsume }, AcquireCommodity { commodity: Bread, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Apple }, ConsumeOwnedCommodity { commodity: Water }, ExploreLocation { target_place: EntityId { slot: 1, generation: 0 }, motivating_need: Hunger }, ProduceCommodity { recipe_id: RecipeId(2) }, Relieve, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed] (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 19 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=175000, total=175000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=250, weight=700, score=175000, recovery_relevant=true); Thirst(pressure=93, weight=700, score=65100, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=186200, total=186200, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, replacement=SameGoalSiblingReplaced, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=266, weight=700, score=186200, recovery_relevant=true); Thirst(pressure=112, weight=700, score=78400, recovery_relevant=true)], feasibility=Likely; ... and 8 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=182, weight=700, score=127400, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely; ... and 8 more |
| 300–399 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ACTIVE: toilet — interrupt: NoInterrupt (×6); ACTIVE: toilet — interrupt: NoInterrupt, frame=[resumed]; ... and 14 more |
| 400–499 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 17 more |
| 500–599 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=149100, total=149100, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=90, weight=700, score=63000, recovery_relevant=true); Thirst(pressure=213, weight=700, score=149100, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 10 more |
| 600–699 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 9 more |
| 700–799 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: toilet — interrupt: NoInterrupt (×6); ACTIVE: toilet — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: travel — interrupt: NoInterrupt (×2); ... and 13 more |
| 800–899 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ... and 17 more |
| 900–999 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=152600, total=152600, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=2, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=EncounteredDifferentGoal(ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=218, weight=700, score=152600, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=MotiveScore ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=Relieve, selected_opportunity=Relieve@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Relieve, path=Relieve, primary=288600, total=288600, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=from=EntityId { slot: 1, generation: 0 }@2, kept=[EntityId { slot: 0, generation: 0 }[base=3, threat=0, penalty=0, direct=3, remain=0, total=3],EntityId { slot: 2, generation: 0 }[base=2, threat=0, penalty=0, direct=2, remain=0, total=2]], pruned=[]], candidates=1, plans_found=1, same_goal=trigger=Relieve@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Bladder(pressure=444, weight=650, score=288600, recovery_relevant=false)], feasibility=Likely; ... and 3 more |
| 1000–1099 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ACTIVE: toilet — interrupt: NoInterrupt (×6); ACTIVE: toilet — interrupt: NoInterrupt, frame=[resumed]; ... and 14 more |
| 1100–1199 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ... and 22 more |
| 1200–1299 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=158200, total=158200, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=92, weight=700, score=64400, recovery_relevant=true); Thirst(pressure=226, weight=700, score=158200, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=143500, total=143500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=144, weight=700, score=100800, recovery_relevant=true); Thirst(pressure=205, weight=700, score=143500, recovery_relevant=true)], feasibility=Likely; ... and 9 more |
| 1300–1399 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=78, weight=700, score=54600, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 10 more |
| 1400–1499 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: toilet — interrupt: NoInterrupt (×6); ACTIVE: toilet — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: travel — interrupt: NoInterrupt (×2); ... and 9 more |

**Failed plan attempts** (showing first 20 of 10)

| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |
|------|------|---------|------------|-----------|------------|----------|--------------------|
| 18 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 231 | 9 | 374 | e1g0 | n/a |
| 21 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 231 | 9 | 374 | e0g0 | n/a |
| 404 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 231 | 9 | 2706 | e0g0 | n/a |
| 496 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 7620 | e1g0 | n/a |
| 566 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 7620 | e1g0 | n/a |
| 645 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 7620 | e1g0 | n/a |
| 813 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 231 | 9 | 2772 | e0g0 | n/a |
| 1172 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 231 | 9 | 1716 | e0g0 | n/a |
| 1273 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 2186 | e1g0 | n/a |
| 1343 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 1772 | e1g0 | n/a |

### Failed Plan Frequency Breakdown
- frontier-exhausted: 5 / 10
- budget-exhausted: 5 / 10
- Max Depth = 0 (no operators available): 0 / 10
- Had Target Beliefs = false: 0 / 10

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

**Affordances after travel** (tick 324, arrived at Riverside Camp)

- sleep
- toilet
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

**Affordances after travel** (tick 407, arrived at Fertile Fields)

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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
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
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 727, arrived at Riverside Camp)

- sleep
- toilet
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

**Affordances after travel** (tick 831, arrived at Fertile Fields)

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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 1092, arrived at Riverside Camp)

- sleep
- toilet
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

**Affordances after travel** (tick 1175, arrived at Fertile Fields)

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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 1425, arrived at Riverside Camp)

- sleep
- toilet
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
**Affordance changes** (tick 316): -ask_witness
**Affordance changes** (tick 324): +ask_witness, +harvest:Harvest Water, +toilet, -harvest:Harvest Apples, -relieve_wilderness (at Riverside Camp)
**Affordance changes** (tick 327): +bribe
**Affordance changes** (tick 328): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 342): -ask_witness, -bribe
**Affordance changes** (tick 343): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 407): +ask_witness, +harvest:Harvest Apples, +relieve_wilderness, -harvest:Harvest Water, -toilet (at Fertile Fields)
**Affordance changes** (tick 411): +bribe
**Affordance changes** (tick 413): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 450): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 499): +bribe
**Affordance changes** (tick 501): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 528): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 569): +bribe
**Affordance changes** (tick 572): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 607): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 648): +bribe
**Affordance changes** (tick 650): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 694): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 727): +harvest:Harvest Water, +toilet, -ask_witness, -harvest:Harvest Apples, -relieve_wilderness (at Riverside Camp)
**Affordance changes** (tick 732): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 747): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 818): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 831): +ask_witness, +bribe, +harvest:Harvest Apples, +relieve_wilderness, -harvest:Harvest Water, -toilet (at Fertile Fields)
**Affordance changes** (tick 835): +eat
**Affordance changes** (tick 905): -eat
**Affordance changes** (tick 944): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 1015): +bribe
**Affordance changes** (tick 1017): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1051): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1088): -ask_witness
**Affordance changes** (tick 1092): +harvest:Harvest Water, +toilet, -harvest:Harvest Apples, -relieve_wilderness (at Riverside Camp)
**Affordance changes** (tick 1105): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1120): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 1175): +ask_witness, +harvest:Harvest Apples, +relieve_wilderness, -harvest:Harvest Water, -toilet (at Fertile Fields)
**Affordance changes** (tick 1178): +bribe
**Affordance changes** (tick 1180): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1228): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1276): -ask_witness
**Affordance changes** (tick 1278): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1284): +ask_witness, +bribe
**Affordance changes** (tick 1305): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1346): +bribe
**Affordance changes** (tick 1348): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1384): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1425): +harvest:Harvest Water, +toilet, -ask_witness, -harvest:Harvest Apples, -relieve_wilderness (at Riverside Camp)
**Affordance changes** (tick 1430): +drink, +drop_item, +put_down, +store_stock, +wash
**Final affordances** (tick 1432)

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
- unstage_stock (1 targets)
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

**Tick breakdown**: 1152 planning, 288 active-action, 0 dead
**Plan search outcomes**: 257 found, 8 frontier-exhausted, 6 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Apple, purpose: SelfConsume }, AcquireCommodity { commodity: Bread, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Apple }, ConsumeOwnedCommodity { commodity: Water }, ExploreLocation { target_place: EntityId { slot: 0, generation: 0 }, motivating_need: Dirtiness }, ExploreLocation { target_place: EntityId { slot: 1, generation: 0 }, motivating_need: Hunger }, ExploreLocation { target_place: EntityId { slot: 2, generation: 0 }, motivating_need: Dirtiness }, ExploreLocation { target_place: EntityId { slot: 3, generation: 0 }, motivating_need: Dirtiness }, ProduceCommodity { recipe_id: RecipeId(2) }, Relieve, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×4); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ACTIVE: travel — interrupt: NoInterrupt (×2); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=166500, total=166500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=222, weight=750, score=166500, recovery_relevant=true); Thirst(pressure=170, weight=700, score=119000, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0; ... and 13 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Relieve@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=117, weight=750, score=87750, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 9 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ACTIVE: toilet — interrupt: NoInterrupt (×7); ... and 15 more |
| 300–399 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×2); ACTIVE: toilet — interrupt: NoInterrupt (×7); ... and 18 more |
| 400–499 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=10, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ReachedCandidatePlanCap, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=136, weight=750, score=102000, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=141400, total=141400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=99, weight=750, score=74250, recovery_relevant=true); Thirst(pressure=202, weight=700, score=141400, recovery_relevant=true)], feasibility=Likely; ... and 9 more |
| 500–599 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×8); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=158200, total=158200, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=10, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=141, weight=750, score=105750, recovery_relevant=true); Thirst(pressure=226, weight=700, score=158200, recovery_relevant=true)], feasibility=Likely; ... and 9 more |
| 600–699 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e0g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140000, total=140000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=10, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=117, weight=750, score=87750, recovery_relevant=true); Thirst(pressure=200, weight=700, score=140000, recovery_relevant=true)], feasibility=Likely; ... and 13 more |
| 700–799 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ACTIVE: travel — interrupt: NoInterrupt; ACTIVE: wash — interrupt: NoInterrupt (×11); ... and 14 more |
| 800–899 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ... and 12 more |
| 900–999 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×8); ... and 11 more |
| 1000–1099 | ACTIVE: ask_witness — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0, frame=[resumed]; ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ... and 19 more |
| 1100–1199 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ACTIVE: travel — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=117, weight=750, score=87750, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 11 more |
| 1200–1299 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt; ... and 25 more |
| 1300–1399 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=140700, total=140700, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=10, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ReachedCandidatePlanCap, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=133, weight=750, score=99750, recovery_relevant=true); Thirst(pressure=201, weight=700, score=140700, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=165750, total=165750, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=221, weight=750, score=165750, recovery_relevant=true); Thirst(pressure=189, weight=700, score=132300, recovery_relevant=true)], feasibility=Likely; ... and 4 more |
| 1400–1499 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=141400, total=141400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=99, weight=750, score=74250, recovery_relevant=true); Thirst(pressure=202, weight=700, score=141400, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=154000, total=154000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, replacement=SameGoalSiblingReplaced, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=154, weight=750, score=115500, recovery_relevant=true); Thirst(pressure=220, weight=700, score=154000, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=ProgressBarrier[steps=1, next_index=Some(0), next_step=MoveCargo, path=MoveCargo, primary=151900, total=151900, side_benefits=0, search=expansions=640, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=151, weight=750, score=113250, recovery_relevant=true); Thirst(pressure=217, weight=700, score=151900, recovery_relevant=true)], feasibility=Likely; ... and 5 more |

**Failed plan attempts** (showing first 20 of 14)

| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |
|------|------|---------|------------|-----------|------------|----------|--------------------|
| 2 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 1376 | e0g0 | n/a |
| 151 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 3790 | e1g0 | n/a |
| 239 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 5446 | e1g0 | n/a |
| 341 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 231 | 9 | 2530 | e0g0 | n/a |
| 457 | ProduceCommodity { recipe_id: RecipeId(2) } | frontier-exhausted | 512 | 9 | 5805 | e1g0 | n/a |
| 457 | ProduceCommodity { recipe_id: RecipeId(2) } | frontier-exhausted | 512 | 9 | 5805 | e1g0 | n/a |
| 814 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 143 | 9 | 1881 | e2g0 | n/a |
| 1009 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 4194 | e1g0 | n/a |
| 1092 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 4681 | e2g0 | n/a |
| 1092 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 143 | 9 | 1111 | e2g0 | n/a |
| 1166 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 2570 | e1g0 | n/a |
| 1282 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 143 | 9 | 1001 | e2g0 | n/a |
| 1397 | ProduceCommodity { recipe_id: RecipeId(2) } | frontier-exhausted | 512 | 9 | 1707 | e1g0 | n/a |
| 1397 | ProduceCommodity { recipe_id: RecipeId(2) } | frontier-exhausted | 512 | 9 | 1707 | e1g0 | n/a |

### Failed Plan Frequency Breakdown
- frontier-exhausted: 8 / 14
- budget-exhausted: 6 / 14
- Max Depth = 0 (no operators available): 0 / 14
- Had Target Beliefs = false: 0 / 14

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

**Affordances after travel** (tick 248, arrived at Forest Clearing)

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

**Affordances after travel** (tick 253, arrived at Hillside Shelter)

- eat (1 targets)
- drink (1 targets)
- sleep
- toilet
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 260, arrived at Riverside Camp)

- eat (1 targets)
- drink (1 targets)
- sleep
- toilet
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

**Affordances after travel** (tick 344, arrived at Fertile Fields)

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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 769, arrived at Forest Clearing)

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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 816, arrived at Fertile Fields)

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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 1089, arrived at Forest Clearing)

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

**Affordances after travel** (tick 1094, arrived at Fertile Fields)

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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 1176, arrived at Riverside Camp)

- eat (1 targets)
- drink (1 targets)
- sleep
- toilet
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

**Affordances after travel** (tick 1182, arrived at Hillside Shelter)

- eat (1 targets)
- drink (1 targets)
- sleep
- toilet
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 1237, arrived at Forest Clearing)

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

**Affordances after travel** (tick 1284, arrived at Fertile Fields)

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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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
**Affordance changes** (tick 248): +harvest:Harvest Water, -ask_witness, -bribe, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 253): +toilet, -harvest:Harvest Water, -pick_up, -queue_for_facility_use, -relieve_wilderness, -steal (at Hillside Shelter)
**Affordance changes** (tick 260): +harvest:Harvest Water, +pick_up, +queue_for_facility_use, +steal (at Riverside Camp)
**Affordance changes** (tick 270): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 312): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 324): +ask_witness, +bribe
**Affordance changes** (tick 328): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 344): +harvest:Harvest Apples, +relieve_wilderness, -harvest:Harvest Water, -toilet (at Fertile Fields)
**Affordance changes** (tick 347): +bribe
**Affordance changes** (tick 349): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 414): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 460): +bribe
**Affordance changes** (tick 462): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 498): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 547): +bribe
**Affordance changes** (tick 549): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 576): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 617): +bribe
**Affordance changes** (tick 619): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 655): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 696): +bribe
**Affordance changes** (tick 698): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 725): -ask_witness, -bribe
**Affordance changes** (tick 743): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 746): +ask_witness
**Affordance changes** (tick 769): +harvest:Harvest Water, -ask_witness, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 776): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 791): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 816): +ask_witness, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 820): +bribe
**Affordance changes** (tick 822): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 887): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 934): +bribe
**Affordance changes** (tick 935): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 971): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1012): +bribe
**Affordance changes** (tick 1014): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1049): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1089): +harvest:Harvest Water, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 1094): +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 1097): +bribe
**Affordance changes** (tick 1099): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1128): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1169): +bribe
**Affordance changes** (tick 1170): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1176): +harvest:Harvest Water, +toilet, -ask_witness, -bribe, -harvest:Harvest Apples, -relieve_wilderness (at Riverside Camp)
**Affordance changes** (tick 1182): -harvest:Harvest Water, -pick_up, -queue_for_facility_use, -steal (at Hillside Shelter)
**Affordance changes** (tick 1209): +pick_up, +steal
**Affordance changes** (tick 1211): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1237): +harvest:Harvest Water, +queue_for_facility_use, +relieve_wilderness, -toilet (at Forest Clearing)
**Affordance changes** (tick 1249): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1264): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 1276): +ask_witness
**Affordance changes** (tick 1284): +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 1287): +bribe
**Affordance changes** (tick 1289): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1355): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1400): +bribe
**Affordance changes** (tick 1402): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1438): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

### Agent C (1440 decision ticks)

**Tick breakdown**: 1145 planning, 295 active-action, 0 dead
**Plan search outcomes**: 269 found, 5 frontier-exhausted, 8 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Apple, purpose: SelfConsume }, AcquireCommodity { commodity: Bread, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Apple }, ConsumeOwnedCommodity { commodity: Water }, ExploreLocation { target_place: EntityId { slot: 1, generation: 0 }, motivating_need: Hunger }, ProduceCommodity { recipe_id: RecipeId(2) }, Relieve, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×17); ACTIVE: travel — interrupt: NoInterrupt; ... and 15 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×11); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=133400, total=133400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=184, weight=725, score=133400, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 15 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=3, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=Feasibility AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ... and 11 more |
| 300–399 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, frame=[resumed]; ... and 21 more |
| 400–499 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=179200, total=179200, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=256, weight=700, score=179200, recovery_relevant=true); Thirst(pressure=88, weight=725, score=63800, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0; ... and 11 more |
| 500–599 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×3); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 13 more |
| 600–699 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ... and 21 more |
| 700–799 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ... and 18 more |
| 800–899 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=66, weight=700, score=46200, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ... and 11 more |
| 900–999 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×3); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=130500, total=130500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=58, weight=700, score=40600, recovery_relevant=true); Thirst(pressure=180, weight=725, score=130500, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ... and 9 more |
| 1000–1099 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, frame=[resumed]; ... and 16 more |
| 1100–1199 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ... and 16 more |
| 1200–1299 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e2g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×13); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, frame=[resumed]; ... and 14 more |
| 1300–1399 | ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ACTIVE: travel — interrupt: NoInterrupt; ... and 10 more |
| 1400–1499 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=133400, total=133400, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=5, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=44, weight=700, score=30800, recovery_relevant=true); Thirst(pressure=184, weight=725, score=133400, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(2) }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=150800, total=150800, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=60, weight=700, score=42000, recovery_relevant=true); Thirst(pressure=208, weight=725, score=150800, recovery_relevant=true)], feasibility=Likely; ... and 7 more |

**Failed plan attempts** (showing first 20 of 13)

| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |
|------|------|---------|------------|-----------|------------|----------|--------------------|
| 191 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 3740 | e1g0 | n/a |
| 374 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 143 | 9 | 1661 | e2g0 | n/a |
| 536 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 7442 | e1g0 | n/a |
| 595 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 7442 | e1g0 | n/a |
| 744 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 143 | 9 | 1771 | e2g0 | n/a |
| 869 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 6630 | e1g0 | n/a |
| 928 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 6224 | e1g0 | n/a |
| 987 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 5412 | e1g0 | n/a |
| 1111 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 143 | 9 | 1111 | e2g0 | n/a |
| 1273 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 2570 | e1g0 | n/a |
| 1274 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 231 | 9 | 902 | e1g0 | n/a |
| 1350 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 143 | 9 | 561 | e2g0 | n/a |
| 1431 | ProduceCommodity { recipe_id: RecipeId(2) } | budget-exhausted | 640 | 9 | 2212 | e1g0 | n/a |

### Failed Plan Frequency Breakdown
- frontier-exhausted: 5 / 13
- budget-exhausted: 8 / 13
- Max Depth = 0 (no operators available): 0 / 13
- Had Target Beliefs = false: 0 / 13

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

**Affordances after travel** (tick 310, arrived at Forest Clearing)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)

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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 656, arrived at Forest Clearing)

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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 746, arrived at Fertile Fields)

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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 1048, arrived at Forest Clearing)

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

**Affordances after travel** (tick 1138, arrived at Fertile Fields)

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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 1276, arrived at Forest Clearing)

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
- steal (1 targets)
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
- stage_stock_for_sale (1 targets)
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

**Affordances after travel** (tick 1352, arrived at Fertile Fields)

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
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
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
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
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
**Affordance changes** (tick 310): +harvest:Harvest Water, -collect_display_stock, -harvest:Harvest Apples, -pick_up, -stage_stock_for_sale, -steal, -unstage_stock (at Forest Clearing)
**Affordance changes** (tick 311): +collect_display_stock, +pick_up, +stage_stock_for_sale, +steal, +unstage_stock
**Affordance changes** (tick 314): +bribe
**Affordance changes** (tick 315): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 330): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 342): -ask_witness
**Affordance changes** (tick 379): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 400): +ask_witness, +bribe, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 405): +eat
**Affordance changes** (tick 462): -wash
**Affordance changes** (tick 508): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 539): +bribe
**Affordance changes** (tick 540): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 567): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 598): +bribe
**Affordance changes** (tick 600): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 626): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 656): +harvest:Harvest Water, -ask_witness, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 662): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 683): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 712): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 746): +ask_witness, +bribe, +harvest:Harvest Apples, -drop_item, -harvest:Harvest Water, -put_down, -store_stock (at Fertile Fields)
**Affordance changes** (tick 747): +drop_item, +put_down, +store_stock
**Affordance changes** (tick 752): +eat
**Affordance changes** (tick 795): -wash
**Affordance changes** (tick 841): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 872): +bribe
**Affordance changes** (tick 874): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 900): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 931): +bribe
**Affordance changes** (tick 933): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 959): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 990): +bribe
**Affordance changes** (tick 992): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1018): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1048): +harvest:Harvest Water, -ask_witness, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 1052): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1067): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 1089): +ask_witness
**Affordance changes** (tick 1093): -ask_witness
**Affordance changes** (tick 1115): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1138): +ask_witness, +bribe, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 1143): +eat
**Affordance changes** (tick 1199): -wash
**Affordance changes** (tick 1245): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1276): +harvest:Harvest Water, -ask_witness, -harvest:Harvest Apples (at Forest Clearing)
**Affordance changes** (tick 1279): +ask_witness, +bribe
**Affordance changes** (tick 1281): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 1290): -ask_witness, -bribe
**Affordance changes** (tick 1327): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 1352): +ask_witness, +harvest:Harvest Apples, -harvest:Harvest Water (at Fertile Fields)
**Affordance changes** (tick 1355): +bribe
**Affordance changes** (tick 1357): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 1410): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 1434): +bribe
**Affordance changes** (tick 1435): +drink, +drop_item, +eat, +put_down, +store_stock
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
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

## Section 8 — Budget Exhaustion Snapshots

4 unique budget-exhaustion signatures captured (deduplicated by agent+goal+location).

### Snapshot 1 — Agent B at tick 2

**Agent**: Agent B (e5g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 640
- Max depth reached: 9
- Total candidates generated: 1376

**Planner configuration**:
- max_node_expansions: 640
- max_plan_depth: 10
- max_candidates_per_expansion: 240
- max_prerequisite_locations: 4
- beam_width: 12
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=479, thirst=229, fatigue=146, bladder=182, dirtiness=143

**Agent inventory**:
- (empty)

**Beliefs** (3 known entities):
- (unknown): Riverside Camp
- Riverside Camp: Agent A, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 2 — Agent B at tick 151

**Agent**: Agent B (e5g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Fertile Fields (e1g0)

**Search metrics**:
- Expansions used: 640
- Max depth reached: 9
- Total candidates generated: 3790

**Planner configuration**:
- max_node_expansions: 640
- max_plan_depth: 10
- max_candidates_per_expansion: 240
- max_prerequisite_locations: 4
- beam_width: 12
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=123, thirst=206, fatigue=289, bladder=280, dirtiness=297

**Agent inventory**:
- (empty)

**Beliefs** (21 known entities):
- (unknown): Riverside Camp, Fertile Fields
- Fertile Fields: Agent A, Agent B, Agent C, OrchardRow, ItemLot#10, ItemLot#11, ItemLot#14, ItemLot#18, 1× Waste, ItemLot#21, ItemLot#23, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Apple, 1× Apple
- Riverside Camp: Well

**Current place contents**:
- Agent A (agent)
- Agent B (agent)
- Agent C (agent)
- OrchardRow (OrchardRow)
- 2× Apple
- 6× Waste

**Adjacent place contents**:
- Forest Clearing: Well (Well), 1× Waste
- Riverside Camp: Well (Well), 1× Waste

### Snapshot 3 — Agent C at tick 191

**Agent**: Agent C (e6g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Fertile Fields (e1g0)

**Search metrics**:
- Expansions used: 640
- Max depth reached: 9
- Total candidates generated: 3740

**Planner configuration**:
- max_node_expansions: 640
- max_plan_depth: 10
- max_candidates_per_expansion: 240
- max_prerequisite_locations: 4
- beam_width: 12
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=63, thirst=190, fatigue=289, bladder=88, dirtiness=309

**Agent inventory**:
- (empty)

**Beliefs** (24 known entities):
- (unknown): Fertile Fields, Forest Clearing
- Fertile Fields: Agent A, Agent B, Agent C, OrchardRow, ItemLot#10, ItemLot#11, ItemLot#18, 1× Waste, ItemLot#21, ItemLot#23, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Apple, ItemLot#31, ItemLot#34, 1× Waste, 1× Waste, 1× Waste
- Forest Clearing: Well

**Current place contents**:
- Agent A (agent)
- Agent B (agent)
- Agent C (agent)
- OrchardRow (OrchardRow)
- 1× Apple
- 9× Waste

**Adjacent place contents**:
- Forest Clearing: Well (Well), 1× Waste
- Riverside Camp: Well (Well), 1× Waste

### Snapshot 4 — Agent A at tick 496

**Agent**: Agent A (e4g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Fertile Fields (e1g0)

**Search metrics**:
- Expansions used: 640
- Max depth reached: 9
- Total candidates generated: 7620

**Planner configuration**:
- max_node_expansions: 640
- max_plan_depth: 10
- max_candidates_per_expansion: 240
- max_prerequisite_locations: 4
- beam_width: 12
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=99, thirst=231, fatigue=299, bladder=12, dirtiness=161

**Agent inventory**:
- (empty)

**Beliefs** (48 known entities):
- (unknown): Riverside Camp, Fertile Fields
- Fertile Fields: Agent A, Agent B, Agent C, OrchardRow, ItemLot#10, ItemLot#11, 1× Waste, ItemLot#23, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste, ItemLot#30, ItemLot#31, ItemLot#34, 1× Waste, 1× Waste, 1× Waste, ItemLot#39, 1× Waste, 1× Waste, ItemLot#43, ItemLot#44, ItemLot#46, 1× Waste, 1× Waste, 1× Waste, ItemLot#61, ItemLot#64, 1× Apple, 1× Waste, ItemLot#70, 1× Waste, 1× Waste, 1× Apple, 1× Waste, 1× Waste, 1× Waste
- Riverside Camp: Well, 1× Waste, 1× Waste, ItemLot#52, ItemLot#57, 1× Waste, 1× Waste

**Current place contents**:
- Agent A (agent)
- Agent B (agent)
- Agent C (agent)
- OrchardRow (OrchardRow)
- 2× Apple
- 20× Waste

**Adjacent place contents**:
- Forest Clearing: Well (Well), 3× Waste
- Riverside Camp: Well (Well), 4× Waste

