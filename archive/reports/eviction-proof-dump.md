**Status**: ✅ COMPLETED

# Simulation Observer Dump

## Section 1 — Run Metadata

- **Scenario**: `scenarios/cli-evaluation.ron`
- **Seed**: 7777
- **Ticks simulated**: 600
- **Total events**: 7843

### Agents

| Name | EntityId |
|------|----------|
| Kael | e5g0 |
| Merchant Vara | e6g0 |
| Forager Lina | e7g0 |
| Guard Theron | e8g0 |

### Places

| Name | EntityId |
|------|----------|
| Thornwall Village | e0g0 |
| Eldergrove Forest | e1g0 |
| Dusty Trail | e2g0 |
| Hearthstone Inn | e3g0 |
| Golden Fields | e4g0 |

## Section 2 — Per-Agent Summary

### Kael

**Actions** (total lifecycle events: 260)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 8 | 8 | 0 | 0 |
| eat | 5 | 5 | 0 | 0 |
| harvest:Harvest Water | 3 | 3 | 0 | 1 |
| pick_up | 3 | 3 | 0 | 0 |
| relieve_wilderness | 9 | 9 | 0 | 0 |
| sleep | 52 | 52 | 0 | 0 |
| tell | 31 | 31 | 0 | 23 |
| travel | 6 | 6 | 0 | 0 |
| wash | 1 | 1 | 0 | 0 |

**Perception**: 370 total observations, 356 passed, 26 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 2 | 508 | 174 |
| Thirst | 3 | 337 | 120 |
| Fatigue | 102 | 334 | 279 |
| Bladder | 4 | 508 | 181 |
| Dirtiness | 1 | 462 | 217 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 62 |
| e2g0 | 532 |

**Max consecutive idle ticks**: 22

### Merchant Vara

**Actions** (total lifecycle events: 328)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 5 | 5 | 0 | 0 |
| harvest:Harvest Water | 24 | 3 | 0 | 1 |
| pick_up | 3 | 3 | 0 | 0 |
| relieve_wilderness | 25 | 4 | 0 | 0 |
| sleep | 22 | 22 | 0 | 0 |
| tell | 62 | 28 | 0 | 22 |
| travel | 49 | 48 | 0 | 0 |
| wash | 1 | 1 | 0 | 0 |

**Perception**: 401 total observations, 335 passed, 22 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 202 | 1000 | 740 |
| Thirst | 3 | 819 | 248 |
| Fatigue | 102 | 995 | 447 |
| Bladder | 0 | 1000 | 339 |
| Dirtiness | 1 | 1000 | 391 |

**Ticks above 750‰**: hunger=331, thirst=21, fatigue=89, bladder=84, dirtiness=136

**Behavioral transition** at tick 400: action repertoire narrowed (8 types -> 3 types)
  Needs: hunger=1000, thirst=180, fatigue=492, bladder=832, dirtiness=67

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 215 |
| e2g0 | 336 |

**Max consecutive idle ticks**: 12

### Forager Lina

**Actions** (total lifecycle events: 135)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 5 | 5 | 0 | 0 |
| eat | 19 | 14 | 0 | 0 |
| harvest:Harvest Apples | 3 | 3 | 0 | 0 |
| pick_up | 5 | 5 | 0 | 0 |
| relieve_wilderness | 10 | 6 | 0 | 0 |
| sleep | 19 | 19 | 0 | 0 |
| tell | 1 | 1 | 0 | 0 |
| travel | 10 | 10 | 0 | 0 |

**Perception**: 122 total observations, 87 passed, 13 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 2 | 570 | 190 |
| Thirst | 5 | 1000 | 430 |
| Fatigue | 102 | 950 | 458 |
| Bladder | 0 | 1000 | 372 |
| Dirtiness | 101 | 1000 | 438 |

**Ticks above 750‰**: hunger=0, thirst=194, fatigue=100, bladder=84, dirtiness=63

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 4 |
| e1g0 | 477 |
| e2g0 | 108 |

**Max consecutive idle ticks**: 217

### Guard Theron

**Actions** (total lifecycle events: 444)

| Action | Started | Committed | Aborted | StartFailed |
|--------|---------|-----------|---------|-------------|
| drink | 7 | 7 | 0 | 0 |
| eat | 7 | 7 | 0 | 0 |
| harvest:Harvest Water | 5 | 5 | 0 | 0 |
| investigate | 30 | 9 | 1 | 0 |
| patrol | 22 | 4 | 0 | 0 |
| pick_up | 5 | 5 | 0 | 0 |
| relieve_wilderness | 7 | 7 | 0 | 0 |
| sleep | 51 | 51 | 0 | 0 |
| tell | 71 | 68 | 2 | 33 |
| travel | 19 | 19 | 0 | 0 |
| wash | 1 | 1 | 0 | 0 |

**Perception**: 443 total observations, 407 passed, 23 unique entities observed

**Needs trajectory** (‰)

| Need | Min | Max | Avg |
|------|-----|-----|-----|
| Hunger | 78 | 384 | 207 |
| Thirst | 3 | 394 | 152 |
| Fatigue | 102 | 448 | 299 |
| Bladder | 4 | 668 | 216 |
| Dirtiness | 1 | 509 | 238 |

**Ticks above 750‰**: hunger=0, thirst=0, fatigue=0, bladder=0, dirtiness=0

**Locations visited**

| Place | Ticks |
|-------|-------|
| e0g0 | 326 |
| e2g0 | 255 |

**Max consecutive idle ticks**: 10

## Section 3 — Anomaly Flags

29 anomalies detected:

### Anomaly 1 — REDUNDANT_PERCEPTION (Kael)

Observed entity e2g0 67 times (may indicate redundant perception if entity state unchanged)

### Anomaly 2 — REDUNDANT_PERCEPTION (Kael)

Observed entity e5g0 176 times (may indicate redundant perception if entity state unchanged)

### Anomaly 3 — REDUNDANT_PERCEPTION (Kael)

Observed entity e6g0 194 times (may indicate redundant perception if entity state unchanged)

### Anomaly 4 — REDUNDANT_PERCEPTION (Kael)

Observed entity e7g0 32 times (may indicate redundant perception if entity state unchanged)

### Anomaly 5 — REDUNDANT_PERCEPTION (Kael)

Observed entity e8g0 168 times (may indicate redundant perception if entity state unchanged)

### Anomaly 6 — STUCK_AGENT (Kael)

No actions for 22 consecutive ticks

### Anomaly 7 — REDUNDANT_PERCEPTION (Merchant Vara)

Observed entity e2g0 26 times (may indicate redundant perception if entity state unchanged)

### Anomaly 8 — REDUNDANT_PERCEPTION (Merchant Vara)

Observed entity e5g0 121 times (may indicate redundant perception if entity state unchanged)

### Anomaly 9 — REDUNDANT_PERCEPTION (Merchant Vara)

Observed entity e6g0 261 times (may indicate redundant perception if entity state unchanged)

### Anomaly 10 — REDUNDANT_PERCEPTION (Merchant Vara)

Observed entity e7g0 18 times (may indicate redundant perception if entity state unchanged)

### Anomaly 11 — REDUNDANT_PERCEPTION (Merchant Vara)

Observed entity e8g0 176 times (may indicate redundant perception if entity state unchanged)

### Anomaly 12 — REDUNDANT_PERCEPTION (Merchant Vara)

Observed entity e21g0 10 times (may indicate redundant perception if entity state unchanged)

### Anomaly 13 — SUSTAINED_CRITICAL_NEED (Merchant Vara)

hunger above 750‰ for 331 consecutive ticks (ticks 269–599)

Tick range: 269–599

### Anomaly 14 — SUSTAINED_CRITICAL_NEED (Merchant Vara)

dirtiness above 750‰ for 136 consecutive ticks (ticks 464–599)

Tick range: 464–599

### Anomaly 15 — REDUNDANT_PERCEPTION (Forager Lina)

Observed entity e2g0 24 times (may indicate redundant perception if entity state unchanged)

### Anomaly 16 — REDUNDANT_PERCEPTION (Forager Lina)

Observed entity e5g0 22 times (may indicate redundant perception if entity state unchanged)

### Anomaly 17 — REDUNDANT_PERCEPTION (Forager Lina)

Observed entity e6g0 28 times (may indicate redundant perception if entity state unchanged)

### Anomaly 18 — REDUNDANT_PERCEPTION (Forager Lina)

Observed entity e7g0 32 times (may indicate redundant perception if entity state unchanged)

### Anomaly 19 — REDUNDANT_PERCEPTION (Forager Lina)

Observed entity e8g0 42 times (may indicate redundant perception if entity state unchanged)

### Anomaly 20 — STUCK_AGENT (Forager Lina)

No actions for 217 consecutive ticks

### Anomaly 21 — SUSTAINED_CRITICAL_NEED (Forager Lina)

thirst above 750‰ for 194 consecutive ticks (ticks 406–599)

Tick range: 406–599

### Anomaly 22 — SUSTAINED_CRITICAL_NEED (Forager Lina)

fatigue above 750‰ for 100 consecutive ticks (ticks 500–599)

Tick range: 500–599

### Anomaly 23 — REDUNDANT_PERCEPTION (Guard Theron)

Observed entity e0g0 47 times (may indicate redundant perception if entity state unchanged)

### Anomaly 24 — REDUNDANT_PERCEPTION (Guard Theron)

Observed entity e2g0 59 times (may indicate redundant perception if entity state unchanged)

### Anomaly 25 — REDUNDANT_PERCEPTION (Guard Theron)

Observed entity e5g0 101 times (may indicate redundant perception if entity state unchanged)

### Anomaly 26 — REDUNDANT_PERCEPTION (Guard Theron)

Observed entity e6g0 198 times (may indicate redundant perception if entity state unchanged)

### Anomaly 27 — REDUNDANT_PERCEPTION (Guard Theron)

Observed entity e7g0 28 times (may indicate redundant perception if entity state unchanged)

### Anomaly 28 — REDUNDANT_PERCEPTION (Guard Theron)

Observed entity e8g0 302 times (may indicate redundant perception if entity state unchanged)

### Anomaly 29 — REDUNDANT_PERCEPTION (Guard Theron)

Observed entity e21g0 17 times (may indicate redundant perception if entity state unchanged)

## Section 4 — Raw Event Sample

### First 100 events

```
[0] tick=0 actor=None action=None place=None tags={} deltas=0
[1] tick=0 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[2] tick=0 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[3] tick=0 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[4] tick=0 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[5] tick=0 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[6] tick=0 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("patrol") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted} deltas=0
[7] tick=0 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("drink") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[8] tick=0 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[9] tick=0 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[10] tick=0 actor=None action=None place=None tags={System} deltas=0
[11] tick=1 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[12] tick=1 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[13] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[14] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[15] tick=1 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[16] tick=1 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[17] tick=1 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[18] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[19] tick=1 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[20] tick=1 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[21] tick=1 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("pick_up") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Inventory, Transfer, ActionCommitted} deltas=0
[22] tick=1 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[23] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[24] tick=1 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[25] tick=1 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[26] tick=1 actor=None action=None place=None tags={System} deltas=0
[27] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[28] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[29] tick=2 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[30] tick=2 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[31] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[32] tick=2 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[33] tick=2 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=0
[34] tick=2 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[35] tick=2 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[36] tick=2 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[37] tick=2 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[38] tick=2 actor=None action=None place=None tags={System} deltas=0
[39] tick=3 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[40] tick=3 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[41] tick=3 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[42] tick=3 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[43] tick=3 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[44] tick=3 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[45] tick=3 actor=None action=None place=None tags={System} deltas=0
[46] tick=4 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[47] tick=4 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[48] tick=4 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[49] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[50] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=0
[51] tick=4 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[52] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[53] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[54] tick=4 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[55] tick=4 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[56] tick=4 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[57] tick=4 actor=None action=None place=None tags={System} deltas=0
[58] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[59] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[60] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[61] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[62] tick=5 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[63] tick=5 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[64] tick=5 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[65] tick=5 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[66] tick=5 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[67] tick=5 actor=None action=None place=None tags={System} deltas=0
[68] tick=6 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[69] tick=6 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[70] tick=6 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[71] tick=6 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("patrol") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionCommitted, Patrol} deltas=0
[72] tick=6 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=0
[73] tick=6 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=0
[74] tick=6 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[75] tick=6 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[76] tick=6 actor=None action=None place=None tags={System} deltas=0
[77] tick=7 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[78] tick=7 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[79] tick=7 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[80] tick=7 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[81] tick=7 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[82] tick=7 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[83] tick=7 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[84] tick=7 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[85] tick=7 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[86] tick=7 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("travel") place=Some(EntityId { slot: 2, generation: 0 }) tags={ActionStarted, Travel} deltas=0
[87] tick=7 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=0
[88] tick=7 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[89] tick=7 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[90] tick=7 actor=None action=None place=None tags={System} deltas=0
[91] tick=8 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[92] tick=8 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=None tags={} deltas=0
[93] tick=8 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[94] tick=8 actor=Some(EntityId { slot: 7, generation: 0 }) action=Some("eat") place=Some(EntityId { slot: 1, generation: 0 }) tags={ActionStarted} deltas=0
[95] tick=8 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=0
[96] tick=8 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=0
[97] tick=8 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("travel") place=None tags={ActionCommitted, Travel} deltas=0
[98] tick=8 actor=None action=None place=None tags={WorldMutation, System} deltas=0
[99] tick=8 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 1, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
```

### Last 100 events

```
[7743] tick=591 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=2
[7744] tick=591 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=2
[7745] tick=591 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7746] tick=591 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7747] tick=591 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7748] tick=591 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7749] tick=591 actor=None action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, System} deltas=1
[7750] tick=591 actor=None action=None place=None tags={System} deltas=0
[7751] tick=592 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=1
[7752] tick=592 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[7753] tick=592 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[7754] tick=592 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7755] tick=592 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7756] tick=592 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[7757] tick=592 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7758] tick=592 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7759] tick=592 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7760] tick=592 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7761] tick=592 actor=None action=None place=None tags={System} deltas=0
[7762] tick=593 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7763] tick=593 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=2
[7764] tick=593 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7765] tick=593 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7766] tick=593 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7767] tick=593 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7768] tick=593 actor=None action=None place=None tags={System} deltas=0
[7769] tick=594 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[7770] tick=594 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7771] tick=594 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7772] tick=594 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[7773] tick=594 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7774] tick=594 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7775] tick=594 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7776] tick=594 actor=None action=None place=None tags={System} deltas=0
[7777] tick=595 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[7778] tick=595 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=1
[7779] tick=595 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7780] tick=595 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("sleep") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[7781] tick=595 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=2
[7782] tick=595 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("sleep") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=1
[7783] tick=595 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7784] tick=595 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7785] tick=595 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7786] tick=595 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7787] tick=595 actor=None action=None place=None tags={System} deltas=0
[7788] tick=596 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[7789] tick=596 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=1
[7790] tick=596 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[7791] tick=596 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[7792] tick=596 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[7793] tick=596 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[7794] tick=596 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7795] tick=596 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7796] tick=596 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=1
[7797] tick=596 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[7798] tick=596 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7799] tick=596 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7800] tick=596 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7801] tick=596 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7802] tick=596 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=1
[7803] tick=596 actor=None action=None place=None tags={System} deltas=0
[7804] tick=597 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[7805] tick=597 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[7806] tick=597 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7807] tick=597 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=2
[7808] tick=597 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7809] tick=597 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7810] tick=597 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7811] tick=597 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7812] tick=597 actor=None action=None place=None tags={System} deltas=0
[7813] tick=598 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=0
[7814] tick=598 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7815] tick=598 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7816] tick=598 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[7817] tick=598 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=9
[7818] tick=598 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7819] tick=598 actor=None action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, System} deltas=1
[7820] tick=598 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7821] tick=598 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7822] tick=598 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7823] tick=598 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7824] tick=598 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7825] tick=598 actor=None action=None place=None tags={System} deltas=0
[7826] tick=599 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=0
[7827] tick=599 actor=Some(EntityId { slot: 5, generation: 0 }) action=None place=None tags={} deltas=1
[7828] tick=599 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=0
[7829] tick=599 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[7830] tick=599 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=None tags={} deltas=1
[7831] tick=599 actor=Some(EntityId { slot: 8, generation: 0 }) action=None place=None tags={} deltas=1
[7832] tick=599 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("sleep") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=0
[7833] tick=599 actor=Some(EntityId { slot: 6, generation: 0 }) action=Some("harvest:Harvest Water") place=Some(EntityId { slot: 0, generation: 0 }) tags={ActionStarted} deltas=1
[7834] tick=599 actor=Some(EntityId { slot: 8, generation: 0 }) action=Some("tell") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted, Social} deltas=2
[7835] tick=599 actor=Some(EntityId { slot: 5, generation: 0 }) action=Some("sleep") place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, ActionCommitted} deltas=1
[7836] tick=599 actor=None action=None place=None tags={WorldMutation, System} deltas=6
[7837] tick=599 actor=Some(EntityId { slot: 6, generation: 0 }) action=None place=Some(EntityId { slot: 0, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7838] tick=599 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7839] tick=599 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7840] tick=599 actor=Some(EntityId { slot: 7, generation: 0 }) action=None place=Some(EntityId { slot: 2, generation: 0 }) tags={WorldMutation, Discovery} deltas=0
[7841] tick=599 actor=None action=None place=None tags={WorldMutation, System} deltas=4
[7842] tick=599 actor=None action=None place=None tags={System} deltas=0
```

### Action Trace Summary

Total action trace events: 1167

#### Per-Agent Action Timeline (100-tick bins)

**Kael (e5g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | tell×9, drink×2, eat×2, relieve_wilderness×2, pick_up×1, travel×1 |
| 100–199 | sleep×10, tell×6, drink×1, eat×1, relieve_wilderness×1 |
| 200–299 | sleep×10, tell×5, drink×2, relieve_wilderness×2, eat×1 |
| 300–399 | sleep×11, tell×9, travel×2, drink×1, eat×1, harvest:Harvest Water×1, pick_up×1, relieve_wilderness×1, wash×1 |
| 400–499 | tell×12, sleep×10, relieve_wilderness×2, travel×2, drink×1, harvest:Harvest Water×1, pick_up×1 |
| 500–599 | tell×13, sleep×11, harvest:Harvest Water×2, drink×1, relieve_wilderness×1, travel×1 |

**Merchant Vara (e6g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | tell×21, drink×2, relieve_wilderness×2, harvest:Harvest Water×1, pick_up×1, sleep×1, travel×1 |
| 100–199 | tell×19, sleep×5, travel×2, drink×1, harvest:Harvest Water×1, pick_up×1, relieve_wilderness×1 |
| 200–299 | tell×33, sleep×15, drink×1, relieve_wilderness×1 |
| 300–399 | travel×20, tell×11, relieve_wilderness×10, drink×1, harvest:Harvest Water×1, pick_up×1, sleep×1, wash×1 |
| 400–499 | travel×21, relieve_wilderness×9, harvest:Harvest Water×6 |
| 500–599 | harvest:Harvest Water×16, travel×5, relieve_wilderness×2 |

**Forager Lina (e7g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | eat×8, pick_up×2, relieve_wilderness×2, drink×1 |
| 100–199 | sleep×10, drink×4, relieve_wilderness×3, eat×1, harvest:Harvest Apples×1, pick_up×1 |
| 200–299 | sleep×9, eat×3, travel×3, harvest:Harvest Apples×2, pick_up×1, relieve_wilderness×1, tell×1 |
| 500–599 | eat×7, travel×7, relieve_wilderness×4, pick_up×1 |

**Guard Theron (e8g0)**

| Ticks | Actions |
|-------|---------|
| 0–99 | tell×22, investigate×6, travel×5, drink×2, eat×2, patrol×2, pick_up×2, relieve_wilderness×2, harvest:Harvest Water×1, sleep×1 |
| 100–199 | tell×15, sleep×7, travel×5, investigate×4, patrol×4, drink×1, eat×1, harvest:Harvest Water×1, pick_up×1, relieve_wilderness×1 |
| 200–299 | sleep×13, patrol×7, tell×3, travel×3, investigate×2, drink×1, eat×1, relieve_wilderness×1 |
| 300–399 | tell×27, sleep×10, drink×1, eat×1, harvest:Harvest Water×1, patrol×1, pick_up×1, relieve_wilderness×1, travel×1, wash×1 |
| 400–499 | tell×25, sleep×11, investigate×4, travel×3, patrol×2, drink×1, eat×1, harvest:Harvest Water×1, pick_up×1, relieve_wilderness×1 |
| 500–599 | investigate×14, tell×12, sleep×9, patrol×6, travel×2, drink×1, eat×1, harvest:Harvest Water×1, relieve_wilderness×1 |

#### Raw Action Trace (last 50 events)

```
tick 570 seq 2: e5g0 committed 'sleep' (instance ai576, 0 materializations)
tick 571 seq 0: e8g0 started 'sleep' targeting []
tick 571 seq 1: e6g0 committed 'travel' (instance ai577, 0 materializations)
tick 571 seq 2: e8g0 committed 'sleep' (instance ai578, 0 materializations)
tick 572 seq 0: e6g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 21, generation: 0 }]
tick 572 seq 1: e8g0 started 'patrol' targeting [EntityId { slot: 2, generation: 0 }]
tick 574 seq 0: e8g0 started 'investigate' targeting [EntityId { slot: 2, generation: 0 }] [investigate violation 14]
tick 575 seq 0: e6g0 started 'travel' targeting [EntityId { slot: 2, generation: 0 }]
tick 576 seq 0: e8g0 started 'investigate' targeting [EntityId { slot: 2, generation: 0 }] [investigate violation 13]
tick 578 seq 0: e6g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 21, generation: 0 }]
tick 578 seq 1: e8g0 started 'patrol' targeting [EntityId { slot: 2, generation: 0 }]
tick 580 seq 0: e5g0 started 'sleep' targeting []
tick 580 seq 1: e8g0 started 'sleep' targeting []
tick 580 seq 2: e5g0 committed 'sleep' (instance ai586, 0 materializations)
tick 580 seq 3: e8g0 committed 'sleep' (instance ai587, 0 materializations)
tick 581 seq 0: e8g0 started 'investigate' targeting [EntityId { slot: 2, generation: 0 }] [investigate violation 14]
tick 583 seq 0: e8g0 started 'travel' targeting [EntityId { slot: 0, generation: 0 }]
tick 584 seq 0: e5g0 failed to start 'tell' (request#667, AiPlan, ReproducedAffordance, reason: PreconditionFailed("TargetAtActorPlace(0)")) [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 8, generation: 0 } }]
tick 584 seq 1: e6g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 21, generation: 0 }]
tick 584 seq 2: e8g0 committed 'travel' (instance ai589, 0 materializations)
tick 585 seq 0: e5g0 started 'travel' targeting [EntityId { slot: 0, generation: 0 }]
tick 585 seq 1: e8g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 21, generation: 0 }]
tick 586 seq 0: e5g0 committed 'travel' (instance ai591, 0 materializations)
tick 587 seq 0: e5g0 failed to start 'harvest:Harvest Water' (request#671, AiPlan, ReproducedAffordance, reason: ReservationUnavailable(EntityId { slot: 21, generation: 0 }))
tick 587 seq 1: e8g0 committed 'harvest:Harvest Water' (instance ai592, 0 materializations)
tick 588 seq 0: e5g0 started 'tell' targeting [EntityId { slot: 6, generation: 0 }] [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }]
tick 588 seq 1: e8g0 started 'tell' targeting [EntityId { slot: 5, generation: 0 }] [tell listener e5g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }]
tick 589 seq 0: e5g0 committed 'tell' (instance ai593, 0 materializations) [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }] <tell result=AlreadyHeldEqualOrNewer disposition=AlreadyHeldEqualOrNewer changed=false delta=no_change>
tick 589 seq 1: e8g0 committed 'tell' (instance ai594, 0 materializations) [tell listener e5g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }] <tell result=NotInternalized disposition=NotInternalized changed=false delta=no_change>
tick 590 seq 0: e5g0 started 'tell' targeting [EntityId { slot: 8, generation: 0 }] [tell listener e8g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }]
tick 590 seq 1: e6g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 21, generation: 0 }]
tick 590 seq 2: e8g0 started 'tell' targeting [EntityId { slot: 6, generation: 0 }] [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }]
tick 591 seq 0: e5g0 committed 'tell' (instance ai595, 0 materializations) [tell listener e8g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }] <tell result=AlreadyHeldEqualOrNewer disposition=AlreadyHeldEqualOrNewer changed=false delta=no_change>
tick 591 seq 1: e8g0 committed 'tell' (instance ai597, 0 materializations) [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 0, generation: 0 } }] <tell result=AlreadyHeldEqualOrNewer disposition=AlreadyHeldEqualOrNewer changed=false delta=no_change>
tick 592 seq 0: e8g0 started 'tell' targeting [EntityId { slot: 5, generation: 0 }] [tell listener e5g0 topic EntityBelief { subject: EntityId { slot: 19, generation: 0 } }]
tick 593 seq 0: e8g0 committed 'tell' (instance ai598, 0 materializations) [tell listener e5g0 topic EntityBelief { subject: EntityId { slot: 19, generation: 0 } }] <tell result=AlreadyHeldEqualOrNewer disposition=AlreadyHeldEqualOrNewer changed=false delta=no_change>
tick 594 seq 0: e8g0 started 'tell' targeting [EntityId { slot: 6, generation: 0 }] [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 19, generation: 0 } }]
tick 595 seq 0: e5g0 started 'sleep' targeting []
tick 595 seq 1: e8g0 committed 'tell' (instance ai599, 0 materializations) [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 19, generation: 0 } }] <tell result=AlreadyHeldEqualOrNewer disposition=AlreadyHeldEqualOrNewer changed=false delta=no_change>
tick 595 seq 2: e5g0 committed 'sleep' (instance ai600, 0 materializations)
tick 596 seq 0: e5g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 21, generation: 0 }]
tick 596 seq 1: e6g0 failed to start 'harvest:Harvest Water' (request#681, AiPlan, ReproducedAffordance, reason: ReservationUnavailable(EntityId { slot: 21, generation: 0 }))
tick 596 seq 2: e8g0 started 'tell' targeting [EntityId { slot: 5, generation: 0 }] [tell listener e5g0 topic EntityBelief { subject: EntityId { slot: 20, generation: 0 } }]
tick 597 seq 0: e8g0 committed 'tell' (instance ai602, 0 materializations) [tell listener e5g0 topic EntityBelief { subject: EntityId { slot: 20, generation: 0 } }] <tell result=NotInternalized disposition=NotInternalized changed=false delta=no_change>
tick 598 seq 0: e8g0 started 'tell' targeting [EntityId { slot: 6, generation: 0 }] [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 20, generation: 0 } }]
tick 598 seq 1: e5g0 committed 'harvest:Harvest Water' (instance ai601, 0 materializations)
tick 599 seq 0: e5g0 started 'sleep' targeting []
tick 599 seq 1: e6g0 started 'harvest:Harvest Water' targeting [EntityId { slot: 21, generation: 0 }]
tick 599 seq 2: e8g0 committed 'tell' (instance ai603, 0 materializations) [tell listener e6g0 topic EntityBelief { subject: EntityId { slot: 20, generation: 0 } }] <tell result=NotInternalized disposition=NotInternalized changed=false delta=no_change>
tick 599 seq 3: e5g0 committed 'sleep' (instance ai604, 0 materializations)
```

### Perception Trace Summary

Total perception trace events: 1336

**Kael (e5g0)** — 370 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 72 | 2 | 11 |
| 100–199 | 41 | 3 | 7 |
| 200–299 | 63 | 1 | 10 |
| 300–399 | 51 | 5 | 6 |
| 400–499 | 37 | 0 | 7 |
| 500–599 | 92 | 3 | 8 |

**Merchant Vara (e6g0)** — 401 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 68 | 11 | 12 |
| 100–199 | 78 | 9 | 7 |
| 200–299 | 57 | 7 | 10 |
| 300–399 | 73 | 9 | 6 |
| 400–499 | 40 | 15 | 6 |
| 500–599 | 19 | 15 | 5 |

**Forager Lina (e7g0)** — 122 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 2 | 2 | 4 |
| 100–199 | 3 | 5 | 4 |
| 200–299 | 37 | 8 | 9 |
| 500–599 | 45 | 20 | 6 |

**Guard Theron (e8g0)** — 443 observations

| Ticks | Passed | Failed | Entities Observed |
|-------|--------|--------|-------------------|
| 0–99 | 49 | 5 | 9 |
| 100–199 | 80 | 7 | 9 |
| 200–299 | 52 | 2 | 9 |
| 300–399 | 88 | 9 | 8 |
| 400–499 | 51 | 5 | 9 |
| 500–599 | 87 | 8 | 8 |

#### Raw Perception Trace (last 50 events)

```
tick 588 seq 4: e6g0 observed ev7702 (passed @ 632‰), 2 entities, 0 institutional claims
tick 588 seq 5: e8g0 observed ev7702 (passed @ 950‰), 2 entities, 0 institutional claims
tick 589 seq 0: e5g0 observed ev7712 (passed @ 950‰), 2 entities, 0 institutional claims
tick 589 seq 1: e6g0 observed ev7712 (passed @ 630‰), 2 entities, 0 institutional claims
tick 589 seq 2: e8g0 observed ev7712 (passed @ 950‰), 2 entities, 0 institutional claims
tick 589 seq 3: e5g0 observed ev7713 (passed @ 950‰), 2 entities, 0 institutional claims
tick 589 seq 4: e6g0 observed ev7713 (FAILED @ 630‰), 0 entities, 0 institutional claims
tick 589 seq 5: e8g0 observed ev7713 (passed @ 950‰), 2 entities, 0 institutional claims
tick 590 seq 0: e5g0 observed ev7728 (passed @ 950‰), 2 entities, 0 institutional claims
tick 590 seq 1: e6g0 observed ev7728 (FAILED @ 501‰), 0 entities, 0 institutional claims
tick 590 seq 2: e8g0 observed ev7728 (passed @ 950‰), 2 entities, 0 institutional claims
tick 590 seq 3: e5g0 observed ev7730 (passed @ 950‰), 2 entities, 0 institutional claims
tick 590 seq 4: e6g0 observed ev7730 (passed @ 501‰), 2 entities, 0 institutional claims
tick 590 seq 5: e8g0 observed ev7730 (passed @ 950‰), 2 entities, 0 institutional claims
tick 590 seq 6: e5g0 observed ev7732 (passed @ 950‰), 1 entities, 0 institutional claims
tick 590 seq 7: e6g0 observed ev7732 (FAILED @ 501‰), 0 entities, 0 institutional claims
tick 590 seq 8: e8g0 observed ev7732 (passed @ 950‰), 1 entities, 0 institutional claims
tick 591 seq 0: e5g0 observed ev7743 (passed @ 950‰), 2 entities, 0 institutional claims
tick 591 seq 1: e6g0 observed ev7743 (passed @ 626‰), 2 entities, 0 institutional claims
tick 591 seq 2: e8g0 observed ev7743 (passed @ 950‰), 2 entities, 0 institutional claims
tick 591 seq 3: e5g0 observed ev7744 (passed @ 950‰), 2 entities, 0 institutional claims
tick 591 seq 4: e6g0 observed ev7744 (FAILED @ 626‰), 0 entities, 0 institutional claims
tick 591 seq 5: e8g0 observed ev7744 (passed @ 950‰), 2 entities, 0 institutional claims
tick 592 seq 0: e5g0 observed ev7756 (passed @ 950‰), 2 entities, 0 institutional claims
tick 592 seq 1: e6g0 observed ev7756 (passed @ 625‰), 2 entities, 0 institutional claims
tick 592 seq 2: e8g0 observed ev7756 (passed @ 950‰), 2 entities, 0 institutional claims
tick 593 seq 0: e5g0 observed ev7763 (passed @ 950‰), 2 entities, 0 institutional claims
tick 593 seq 1: e6g0 observed ev7763 (FAILED @ 624‰), 0 entities, 0 institutional claims
tick 593 seq 2: e8g0 observed ev7763 (passed @ 950‰), 2 entities, 0 institutional claims
tick 594 seq 0: e5g0 observed ev7772 (passed @ 950‰), 2 entities, 0 institutional claims
tick 594 seq 1: e6g0 observed ev7772 (passed @ 623‰), 2 entities, 0 institutional claims
tick 594 seq 2: e8g0 observed ev7772 (passed @ 950‰), 2 entities, 0 institutional claims
tick 595 seq 0: e5g0 observed ev7781 (passed @ 950‰), 2 entities, 0 institutional claims
tick 595 seq 1: e6g0 observed ev7781 (FAILED @ 622‰), 0 entities, 0 institutional claims
tick 595 seq 2: e8g0 observed ev7781 (passed @ 950‰), 2 entities, 0 institutional claims
tick 596 seq 0: e5g0 observed ev7797 (passed @ 760‰), 2 entities, 0 institutional claims
tick 596 seq 1: e6g0 observed ev7797 (passed @ 621‰), 2 entities, 0 institutional claims
tick 596 seq 2: e8g0 observed ev7797 (passed @ 950‰), 2 entities, 0 institutional claims
tick 597 seq 0: e5g0 observed ev7807 (passed @ 760‰), 2 entities, 0 institutional claims
tick 597 seq 1: e6g0 observed ev7807 (passed @ 620‰), 2 entities, 0 institutional claims
tick 597 seq 2: e8g0 observed ev7807 (passed @ 950‰), 2 entities, 0 institutional claims
tick 598 seq 0: e5g0 observed ev7816 (passed @ 950‰), 2 entities, 0 institutional claims
tick 598 seq 1: e6g0 observed ev7816 (FAILED @ 619‰), 0 entities, 0 institutional claims
tick 598 seq 2: e8g0 observed ev7816 (passed @ 950‰), 2 entities, 0 institutional claims
tick 598 seq 3: e5g0 observed ev7819 (passed @ 950‰), 1 entities, 0 institutional claims
tick 598 seq 4: e6g0 observed ev7819 (FAILED @ 619‰), 0 entities, 0 institutional claims
tick 598 seq 5: e8g0 observed ev7819 (passed @ 950‰), 1 entities, 0 institutional claims
tick 599 seq 0: e5g0 observed ev7834 (passed @ 950‰), 2 entities, 0 institutional claims
tick 599 seq 1: e6g0 observed ev7834 (FAILED @ 492‰), 0 entities, 0 institutional claims
tick 599 seq 2: e8g0 observed ev7834 (passed @ 950‰), 2 entities, 0 institutional claims
```

## Section 5 — Per-Agent Belief Summary

### Kael

**Known entities**: 16
- Agents: 4
- Places: 2
- Items: 7
- Other: 3

**Believed entity locations**:
- (place entity — no parent location): Thornwall Village, Dusty Trail
- Thornwall Village: Kael, Merchant Vara, Guard Theron, Mill, Loom, Well, 1× Bow, 20× Coin, 3× Grain, 1× Sword, 4× Water
- Dusty Trail: Forager Lina, 1× Waste

**Social observations**: 16
**Told beliefs**: 5 (counterparties: Merchant Vara, Forager Lina, Guard Theron)
**Heard beliefs**: 4
**Institutional beliefs**: 0

### Merchant Vara

**Known entities**: 12
- Agents: 4
- Places: 2
- Items: 3
- Other: 3

**Believed entity locations**:
- (place entity — no parent location): Thornwall Village, Dusty Trail
- Thornwall Village: Kael, Merchant Vara, Guard Theron, Mill, Loom, Well, 1× Bow, 1× Sword, 2× Water
- Dusty Trail: Forager Lina

**Social observations**: 6
**Told beliefs**: 0
**Heard beliefs**: 6
**Institutional beliefs**: 0

### Forager Lina

**Known entities**: 12
- Agents: 4
- Places: 3
- Items: 0
- Other: 5

**Believed entity locations**:
- (place entity — no parent location): Thornwall Village, Eldergrove Forest, Dusty Trail
- Thornwall Village: Mill, Loom, Well
- Eldergrove Forest: ChoppingBlock, OrchardRow
- Dusty Trail: Kael, Merchant Vara, Forager Lina, Guard Theron

**Social observations**: 3
**Told beliefs**: 0
**Heard beliefs**: 2
**Institutional beliefs**: 0

### Guard Theron

**Known entities**: 16
- Agents: 4
- Places: 2
- Items: 7
- Other: 3

**Believed entity locations**:
- (place entity — no parent location): Thornwall Village, Dusty Trail
- Thornwall Village: Kael, Merchant Vara, Guard Theron, Mill, Loom, Well, 1× Bow, 20× Coin, 3× Grain, 1× Sword, 4× Water
- Dusty Trail: Forager Lina, 1× Waste

**Social observations**: 16
**Told beliefs**: 9 (counterparties: Kael, Merchant Vara, Forager Lina)
**Heard beliefs**: 2
**Institutional beliefs**: 0

## Section 6 — End-State Inventory & Resources

### Agent Inventories

**Kael**: 20× Coin

**Merchant Vara**: (empty)

**Forager Lina**: (empty)

**Guard Theron**: 1× Bow, 3× Grain, 1× Sword

### Place Contents

**Thornwall Village (e0g0)**: Kael (agent), Merchant Vara (agent), Guard Theron (agent), Mill (Mill), Loom (Loom), Well (Well), 1× Bow, 20× Coin, 3× Grain, 1× Sword, 4× Water

**Eldergrove Forest (e1g0)**: ChoppingBlock (ChoppingBlock), OrchardRow (OrchardRow), 5× Waste

**Dusty Trail (e2g0)**: Forager Lina (agent), 22× Waste

**Hearthstone Inn (e3g0)**: Forge (Forge), WashBasin (WashBasin), 3× Firewood, 2× Medicine

**Golden Fields (e4g0)**: FieldPlot (FieldPlot), GravePlot (GravePlot)

## Section 7 — Per-Agent Decision Summary

### Kael (600 decision ticks)

**Tick breakdown**: 473 planning, 127 active-action, 0 dead
**Plan search outcomes**: 139 found, 54 frontier-exhausted, 2 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Bread, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Bread }, ConsumeOwnedCommodity { commodity: Water }, ProduceCommodity { recipe_id: RecipeId(2) }, Relieve, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 12, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 7, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 8, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 6, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 6, generation: 0 } }, communication_class: Testimony }, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 15, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×5); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>Relieve@none (×4); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>Relieve@none (×5); ... and 32 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>Relieve@none (×7); ACTIVE: tell — interrupt: NoInterrupt (×4); PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=134000, total=134000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=7, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=268, weight=500, score=134000, recovery_relevant=true)], feasibility=Likely, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=100500, total=100500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=7, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, stop=ExhaustedAdmittedOpportunities, replacement=GoalChanged, drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=201, weight=500, score=100500, recovery_relevant=true)], feasibility=Likely, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0; ... and 10 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>Relieve@none (×14); ACTIVE: tell — interrupt: NoInterrupt (×4); PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=125000, total=125000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=10, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=250, weight=500, score=125000, recovery_relevant=true)], feasibility=Likely, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=100500, total=100500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=10, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Water }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=201, weight=500, score=100500, recovery_relevant=true)], feasibility=Likely, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0; ... and 17 more |
| 300–399 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Bread }]@none (×2); ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>Relieve@none (×5); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>Relieve@none (×2); ACTIVE: tell — interrupt: NoInterrupt (×2); ... and 45 more |
| 400–499 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>Relieve@none (×6); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>Relieve@none (×6); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>Relieve@none, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 6, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>Relieve@none; ... and 34 more |
| 500–599 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>Relieve@none; ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>Relieve@none (×6); ACTIVE: tell — interrupt: NoInterrupt (×7); ... and 37 more |

**Failed plan attempts** (showing first 20 of 56)

| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |
|------|------|---------|------------|-----------|------------|----------|--------------------|
| 11 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e0g0 | true |
| 11 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e0g0 | true |
| 12 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e0g0 | true |
| 12 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e0g0 | true |
| 13 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e0g0 | true |
| 29 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 15, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 29 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 18, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 30 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 48 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 48 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 49 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 49 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 55 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 57 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 59 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 96 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 250 | ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 250 | ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 251 | ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 322 | ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |

### Failed Plan Frequency Breakdown
- frontier-exhausted: 20 / 20
- budget-exhausted: 0 / 20
- Max Depth = 0 (no operators available): 20 / 20
- Had Target Beliefs = false: 0 / 20

**Fully blocked desires** (goal generated but all opportunities blocked)

| Goal | Times Blocked |
|------|---------------|
| ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 45 |
| ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 20 |
| ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 12 |

**Affordances available at tick 0** (at e0g0)

- drink (1 targets)
- sleep
- wash (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)

**Affordances after travel** (tick 15, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 337, arrived at Thornwall Village)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
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
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
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
- store_stock (1 targets)
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

**Affordances after travel** (tick 366, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
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
- store_stock (1 targets)
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

**Affordances after travel** (tick 442, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
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

**Affordances after travel** (tick 450, arrived at Dusty Trail)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 587, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
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
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
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

**Affordance changes** (tick 1): +ask_witness, +bribe, +collect_display_stock, +harvest:Harvest Water, +pick_up, +queue_for_facility_use, +stage_stock_for_sale, +steal, +tell, +unstage_stock
**Affordance changes** (tick 2): +eat
**Affordance changes** (tick 15): +relieve_wilderness, -ask_witness, -bribe, -harvest:Harvest Water, -pick_up, -queue_for_facility_use, -steal, -tell (at Dusty Trail)
**Affordance changes** (tick 23): +ask_witness, +bribe, +pick_up, +steal, +tell
**Affordance changes** (tick 269): -drink, -wash
**Affordance changes** (tick 337): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 341): +steal
**Affordance changes** (tick 342): +pick_up
**Affordance changes** (tick 349): +drink, +wash, -pick_up
**Affordance changes** (tick 361): -steal
**Affordance changes** (tick 366): +pick_up, +relieve_wilderness, +steal, -ask_witness, -bribe, -harvest:Harvest Water, -queue_for_facility_use, -tell (at Dusty Trail)
**Affordance changes** (tick 374): +ask_witness, +bribe, +tell
**Affordance changes** (tick 380): -drink, -wash
**Affordance changes** (tick 392): -eat
**Affordance changes** (tick 442): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -bribe, -pick_up, -relieve_wilderness, -steal, -tell (at Thornwall Village)
**Affordance changes** (tick 445): +pick_up, +steal
**Affordance changes** (tick 447): +drink, +wash, -pick_up, -steal
**Affordance changes** (tick 450): +ask_witness, +bribe, +pick_up, +relieve_wilderness, +steal, +tell, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 519): -drink, -wash
**Affordance changes** (tick 587): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 588): +steal
**Affordance changes** (tick 599): +pick_up
**Final affordances** (tick 599)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
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

### Merchant Vara (600 decision ticks)

**Tick breakdown**: 403 planning, 197 active-action, 0 dead
**Plan search outcomes**: 219 found, 10 frontier-exhausted, 192 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Water }, ProduceCommodity { recipe_id: RecipeId(2) }, Relieve, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 8, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony }, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }]@entity:e8g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×7); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>Relieve@none (×6); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>Relieve@none, frame=[resumed]; ACTIVE: tell — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e8g0; ... and 39 more |
| 100–199 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>Relieve@none (×6); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>Relieve@none, frame=[resumed]; ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }; ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e8g0 (×4); ... and 42 more |
| 200–299 | ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>Relieve@none (×7); ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e5g0 (×8); ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e7g0; ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e7g0 (×5); ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e8g0 (×2); ... and 48 more |
| 300–399 | ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×2); ACTIVE: relieve_wilderness — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>Relieve@none, frame=[resumed] (×10); ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e5g0; ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e5g0; ACTIVE: tell — interrupt: InterruptForReplan { trigger: HigherPriorityGoal }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e8g0 (×3); ... and 52 more |
| 400–499 | ACTIVE: harvest:Harvest Water — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×4); ACTIVE: harvest:Harvest Water — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: InterruptForReplan { trigger: CriticalSurvival }, frame=[resumed] (×5); ACTIVE: relieve_wilderness — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>Relieve@none, frame=[resumed] (×4); ACTIVE: travel — interrupt: NoInterrupt (×21); ... and 33 more |
| 500–599 | ACTIVE: harvest:Harvest Water — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×10); ACTIVE: harvest:Harvest Water — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e2g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0, frame=[resumed]; ACTIVE: harvest:Harvest Water — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×3); ACTIVE: harvest:Harvest Water — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=PriorityClass AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Bread, purpose: SelfConsume }]@place:e0g0>Relieve@none, frame=[resumed] (×2); ... and 37 more |

**Failed plan attempts** (showing first 20 of 202)

| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |
|------|------|---------|------------|-----------|------------|----------|--------------------|
| 11 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 5 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 5 | e0g0 | true |
| 11 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 5 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 5 | e0g0 | true |
| 11 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 5 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 5 | e0g0 | true |
| 35 | AcquireCommodity { commodity: Bread, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 35 | AcquireCommodity { commodity: Apple, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 35 | AcquireCommodity { commodity: Grain, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 39 | AcquireCommodity { commodity: Bread, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 39 | AcquireCommodity { commodity: Apple, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 39 | AcquireCommodity { commodity: Grain, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 54 | AcquireCommodity { commodity: Bread, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 54 | AcquireCommodity { commodity: Apple, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 54 | AcquireCommodity { commodity: Grain, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 693 | e2g0 | n/a |
| 73 | AcquireCommodity { commodity: Bread, purpose: SelfConsume } | budget-exhausted | 150 | 9 | 356 | e2g0 | n/a |
| 73 | AcquireCommodity { commodity: Apple, purpose: SelfConsume } | budget-exhausted | 150 | 9 | 356 | e2g0 | n/a |
| 73 | AcquireCommodity { commodity: Grain, purpose: SelfConsume } | budget-exhausted | 150 | 9 | 356 | e2g0 | n/a |
| 106 | AcquireCommodity { commodity: Bread, purpose: SelfConsume } | budget-exhausted | 75 | 8 | 178 | e2g0 | n/a |
| 106 | AcquireCommodity { commodity: Apple, purpose: SelfConsume } | budget-exhausted | 75 | 8 | 178 | e2g0 | n/a |
| 106 | AcquireCommodity { commodity: Grain, purpose: SelfConsume } | budget-exhausted | 75 | 8 | 178 | e2g0 | n/a |
| 155 | AcquireCommodity { commodity: Bread, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 705 | e0g0 | n/a |
| 155 | AcquireCommodity { commodity: Apple, purpose: SelfConsume } | budget-exhausted | 300 | 9 | 705 | e0g0 | n/a |

### Failed Plan Frequency Breakdown
- frontier-exhausted: 3 / 20
- budget-exhausted: 17 / 20
- Max Depth = 0 (no operators available): 3 / 20
- Had Target Beliefs = false: 0 / 20

**Fully blocked desires** (goal generated but all opportunities blocked)

| Goal | Times Blocked |
|------|---------------|
| AcquireCommodity { commodity: Water, purpose: SelfConsume } | 2 |
| ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | 2 |
| ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 28 |
| ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | 5 |
| ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | 2 |
| ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 30 |
| ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | 30 |
| ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | 9 |
| ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 38 |
| ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | 35 |

**Affordances available at tick 0** (at e0g0)

- sleep
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 21, arrived at Dusty Trail)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- declare_support
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
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 155, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 180, arrived at Dusty Trail)

- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- declare_support
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
- fine (1 targets)
- fine (1 targets)
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

**Affordances after travel** (tick 309, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
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

**Affordances after travel** (tick 344, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 348, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 350, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 354, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 356, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 360, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 362, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
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

**Affordances after travel** (tick 366, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 368, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 372, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 374, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 378, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 380, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 384, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 386, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 390, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 392, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 396, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 398, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 402, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 404, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 408, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 410, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
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

**Affordances after travel** (tick 415, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
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

**Affordances after travel** (tick 417, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
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

**Affordances after travel** (tick 422, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
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

**Affordances after travel** (tick 424, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
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

**Affordances after travel** (tick 429, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 431, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
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

**Affordances after travel** (tick 436, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
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

**Affordances after travel** (tick 438, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
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

**Affordances after travel** (tick 443, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 445, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 450, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 452, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 457, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 459, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 464, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 466, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 469, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
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

**Affordances after travel** (tick 557, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
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
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 562, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 567, arrived at Dusty Trail)

- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 572, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordance changes** (tick 1): +ask_witness, +collect_display_stock, +harvest:Harvest Water, +pick_up, +queue_for_facility_use, +stage_stock_for_sale, +steal, +tell, +unstage_stock
**Affordance changes** (tick 14): +bribe
**Affordance changes** (tick 18): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 21): +relieve_wilderness, -drop_item, -harvest:Harvest Water, -pick_up, -put_down, -queue_for_facility_use, -steal, -store_stock (at Dusty Trail)
**Affordance changes** (tick 29): +drop_item, +pick_up, +put_down, +steal, +store_stock
**Affordance changes** (tick 35): -tell
**Affordance changes** (tick 46): +tell
**Affordance changes** (tick 50): -pick_up, -steal
**Affordance changes** (tick 52): +pick_up, +steal
**Affordance changes** (tick 54): -tell
**Affordance changes** (tick 61): +tell
**Affordance changes** (tick 63): -tell
**Affordance changes** (tick 67): +tell
**Affordance changes** (tick 73): -tell
**Affordance changes** (tick 75): +tell
**Affordance changes** (tick 86): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 155): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness (at Thornwall Village)
**Affordance changes** (tick 168): +bribe, +pick_up
**Affordance changes** (tick 173): +drink, +drop_item, +put_down, +store_stock, +wash, -pick_up
**Affordance changes** (tick 180): +pick_up, +relieve_wilderness, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 241): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 309): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 318): +bribe, +pick_up, +steal
**Affordance changes** (tick 323): +drink, +drop_item, +put_down, +store_stock, +wash, -pick_up, -steal
**Affordance changes** (tick 339): +steal
**Affordance changes** (tick 342): -bribe, -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 344): +pick_up, +relieve_wilderness, -ask_witness, -harvest:Harvest Water, -queue_for_facility_use, -tell (at Dusty Trail)
**Affordance changes** (tick 348): +ask_witness, +harvest:Harvest Water, +queue_for_facility_use, +tell, -pick_up, -relieve_wilderness (at Thornwall Village)
**Affordance changes** (tick 350): +pick_up, +relieve_wilderness, -ask_witness, -harvest:Harvest Water, -queue_for_facility_use, -tell (at Dusty Trail)
**Affordance changes** (tick 354): +ask_witness, +harvest:Harvest Water, +queue_for_facility_use, +tell, -pick_up, -relieve_wilderness (at Thornwall Village)
**Affordance changes** (tick 356): +pick_up, +relieve_wilderness, -ask_witness, -harvest:Harvest Water, -queue_for_facility_use, -tell (at Dusty Trail)
**Affordance changes** (tick 360): +ask_witness, +harvest:Harvest Water, +queue_for_facility_use, +tell, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 362): +pick_up, +relieve_wilderness, +steal, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 366): +harvest:Harvest Water, +queue_for_facility_use, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 368): +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 372): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 374): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 378): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 380): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 384): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 386): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 390): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 392): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 396): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 398): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 402): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 404): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 408): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 410): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 415): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 417): +pick_up, +relieve_wilderness, +steal, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 422): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 424): +pick_up, +relieve_wilderness, +steal, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 429): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 431): +pick_up, +relieve_wilderness, +steal, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 436): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 438): +pick_up, +relieve_wilderness, +steal, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 443): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -pick_up, -relieve_wilderness, -steal, -tell (at Thornwall Village)
**Affordance changes** (tick 445): +ask_witness, +pick_up, +relieve_wilderness, +steal, +tell, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 450): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 452): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 457): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 459): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 464): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 466): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 469): +harvest:Harvest Water, +queue_for_facility_use, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 502): -ask_witness, -tell
**Affordance changes** (tick 503): +ask_witness, +tell
**Affordance changes** (tick 514): +pick_up, +steal
**Affordance changes** (tick 557): +relieve_wilderness, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 562): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 567): +ask_witness, +collect_display_stock, +pick_up, +relieve_wilderness, +stage_stock_for_sale, +steal, +tell, +unstage_stock, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 572): +harvest:Harvest Water, +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 586): +ask_witness, +collect_display_stock, +stage_stock_for_sale, +tell, +unstage_stock
**Affordance changes** (tick 588): +steal
**Affordance changes** (tick 594): -steal
**Affordance changes** (tick 595): +steal
**Affordance changes** (tick 597): -steal
**Affordance changes** (tick 598): +steal
**Final affordances** (tick 599)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

### Forager Lina (600 decision ticks)

**Tick breakdown**: 514 planning, 86 active-action, 0 dead
**Plan search outcomes**: 421 found, 13 frontier-exhausted, 10 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Apple, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Apple }, ConsumeOwnedCommodity { commodity: Water }, ExploreLocation { target_place: EntityId { slot: 2, generation: 0 }, motivating_need: Dirtiness }, Relieve, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, Sleep

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt (×8); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=MoveCargo, path=MoveCargo, primary=423500, total=423500, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ExhaustedAdmittedOpportunities, drive=base=High final=High adjustment=none motive_inputs=[Hunger(pressure=202, weight=600, score=121200, recovery_relevant=true); Thirst(pressure=605, weight=700, score=423500, recovery_relevant=true)], feasibility=Likely; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=MoveCargo, path=MoveCargo, primary=77000, total=77000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=2, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]), drive=base=Low final=Low adjustment=none motive_inputs=[Thirst(pressure=110, weight=700, score=77000, recovery_relevant=true)], feasibility=Likely, ranking=OpportunityStrength AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e1g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0; PLAN (dirty: CLEAN): selected=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }], selected_opportunity=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Consume, path=Consume, primary=119000, total=119000, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=1, plans_found=1, same_goal=trigger=ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none, stop=ExhaustedAdmittedOpportunities, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=2, weight=600, score=1200, recovery_relevant=true); Thirst(pressure=170, weight=700, score=119000, recovery_relevant=true)], feasibility=Likely; ... and 9 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt (×2); ACTIVE: relieve_wilderness — interrupt: NoInterrupt (×14); ACTIVE: relieve_wilderness — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>Relieve@none (×5); PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=1, next_index=Some(0), next_step=Harvest, path=Harvest, primary=147600, total=147600, side_benefits=0, search=expansions=1, root_remaining=0, selected_root_travel=none, pruning=none], candidates=2, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=246, weight=600, score=147600, recovery_relevant=true); Thirst(pressure=100, weight=700, score=70000, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0; ... and 10 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0 (×2); ACTIVE: harvest:Harvest Apples — interrupt: NoInterrupt, ranking=MotiveScore Sleep@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, frame=[resumed]; ... and 40 more |
| 300–399 | PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=153300, total=153300, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), replacement=SameGoalBranchRefreshed, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=92, weight=600, score=55200, recovery_relevant=true); Thirst(pressure=219, weight=700, score=153300, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=156800, total=156800, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), replacement=SameGoalBranchRefreshed, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=94, weight=600, score=56400, recovery_relevant=true); Thirst(pressure=224, weight=700, score=156800, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=160300, total=160300, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), replacement=SameGoalBranchRefreshed, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=96, weight=600, score=57600, recovery_relevant=true); Thirst(pressure=229, weight=700, score=160300, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=163800, total=163800, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), replacement=SameGoalBranchRefreshed, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=98, weight=600, score=58800, recovery_relevant=true); Thirst(pressure=234, weight=700, score=163800, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=167300, total=167300, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=4, plans_found=1, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=EncounteredDifferentGoal(ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]), replacement=SameGoalBranchRefreshed, drive=base=Low final=Low adjustment=none motive_inputs=[Hunger(pressure=100, weight=600, score=60000, recovery_relevant=true); Thirst(pressure=239, weight=700, score=167300, recovery_relevant=true)], feasibility=Likely, ranking=GoalKindOrder AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0>ProduceCommodity [ProduceCommodity { recipe_id: RecipeId(0) }]@place:e1g0, frame=[resumed]; ... and 95 more |
| 400–499 | PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=503300, total=503300, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=6, plans_found=2, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ReachedCandidatePlanCap, replacement=SameGoalBranchRefreshed, drive=base=High final=High adjustment=none motive_inputs=[Hunger(pressure=292, weight=600, score=175200, recovery_relevant=true); Thirst(pressure=719, weight=700, score=503300, recovery_relevant=true)], feasibility=Likely, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=506800, total=506800, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=6, plans_found=2, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ReachedCandidatePlanCap, replacement=SameGoalBranchRefreshed, drive=base=High final=High adjustment=none motive_inputs=[Hunger(pressure=294, weight=600, score=176400, recovery_relevant=true); Thirst(pressure=724, weight=700, score=506800, recovery_relevant=true)], feasibility=Likely, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=510300, total=510300, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=6, plans_found=2, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ReachedCandidatePlanCap, replacement=SameGoalBranchRefreshed, drive=base=High final=High adjustment=none motive_inputs=[Hunger(pressure=296, weight=600, score=177600, recovery_relevant=true); Thirst(pressure=729, weight=700, score=510300, recovery_relevant=true)], feasibility=Likely, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=513800, total=513800, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=6, plans_found=2, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ReachedCandidatePlanCap, replacement=SameGoalBranchRefreshed, drive=base=High final=High adjustment=none motive_inputs=[Hunger(pressure=298, weight=600, score=178800, recovery_relevant=true); Thirst(pressure=734, weight=700, score=513800, recovery_relevant=true)], feasibility=Likely, frame=[resumed]; PLAN (dirty: CLEAN): selected=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }], selected_opportunity=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, source=SearchSelection, selected_plan=GoalSatisfied[steps=0, next_index=None, next_step=none, path=, primary=517300, total=517300, side_benefits=0, search=expansions=0, root_remaining=0, selected_root_travel=none, pruning=none], candidates=6, plans_found=2, same_goal=trigger=AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Apple, purpose: SelfConsume }]@place:e1g0, stop=ReachedCandidatePlanCap, replacement=SameGoalBranchRefreshed, drive=base=High final=High adjustment=none motive_inputs=[Hunger(pressure=300, weight=600, score=180000, recovery_relevant=true); Thirst(pressure=739, weight=700, score=517300, recovery_relevant=true)], feasibility=Likely, frame=[resumed]; ... and 95 more |
| 500–599 | ACTIVE: eat — interrupt: InterruptForReplan { trigger: CriticalSurvival } (×4); ACTIVE: eat — interrupt: InterruptForReplan { trigger: CriticalSurvival }, frame=[resumed]; ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed] (×2); ACTIVE: relieve_wilderness — interrupt: InterruptForReplan { trigger: CriticalSurvival }, frame=[resumed]; ACTIVE: relieve_wilderness — interrupt: InterruptForReplan { trigger: CriticalSurvival }, ranking=MotiveScore ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Apple }]@none>Relieve@none; ... and 25 more |

**Failed plan attempts** (showing first 20 of 23)

| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |
|------|------|---------|------------|-----------|------------|----------|--------------------|
| 250 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 250 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 251 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 27, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 251 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 27, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 252 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 27, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 252 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 24, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 253 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 24, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 253 | ShareBelief { listener: EntityId { slot: 8, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 24, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 254 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 1, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 277 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 19 | 7 | 25 | e2g0 | n/a |
| 542 | ProduceCommodity { recipe_id: RecipeId(0) } | frontier-exhausted | 19 | 7 | 25 | e2g0 | n/a |
| 542 | AcquireCommodity { commodity: Water, purpose: SelfConsume } | frontier-exhausted | 2 | 1 | 2 | e2g0 | n/a |
| 543 | AcquireCommodity { commodity: Apple, purpose: SelfConsume } | frontier-exhausted | 2 | 1 | 2 | e2g0 | n/a |
| 543 | Sleep | budget-exhausted | 224 | 7 | 726 | e2g0 | n/a |
| 547 | Sleep | budget-exhausted | 224 | 7 | 726 | e2g0 | n/a |
| 555 | Sleep | budget-exhausted | 224 | 7 | 726 | e2g0 | n/a |
| 571 | Sleep | budget-exhausted | 112 | 7 | 369 | e2g0 | n/a |
| 585 | Sleep | budget-exhausted | 224 | 7 | 726 | e2g0 | n/a |
| 586 | TreatWounds { patient: EntityId { slot: 7, generation: 0 } } | budget-exhausted | 224 | 7 | 758 | e2g0 | true |
| 589 | Sleep | budget-exhausted | 224 | 7 | 726 | e2g0 | n/a |

### Failed Plan Frequency Breakdown
- frontier-exhausted: 13 / 20
- budget-exhausted: 7 / 20
- Max Depth = 0 (no operators available): 9 / 20
- Had Target Beliefs = false: 0 / 20

**Fully blocked desires** (goal generated but all opportunities blocked)

| Goal | Times Blocked |
|------|---------------|
| ConsumeOwnedCommodity { commodity: Apple } | 5 |

**Affordances available at tick 0** (at e1g0)

- sleep
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 248, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
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

**Affordances after travel** (tick 279, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- attack (1 targets)
- defend
- fine (1 targets)
- exile (1 targets)

**Affordances after travel** (tick 282, arrived at Eldergrove Forest)

- sleep
- relieve_wilderness
- queue_for_facility_use (1 targets)
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

**Affordances after travel** (tick 514, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- declare_support
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

**Affordances after travel** (tick 520, arrived at Thornwall Village)

- eat (1 targets)
- drink (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
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

**Affordances after travel** (tick 522, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
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

**Affordances after travel** (tick 528, arrived at Thornwall Village)

- eat (1 targets)
- drink (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
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

**Affordances after travel** (tick 530, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
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

**Affordances after travel** (tick 536, arrived at Thornwall Village)

- eat (1 targets)
- drink (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
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

**Affordances after travel** (tick 538, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- declare_support
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

**Affordance changes** (tick 1): +collect_display_stock, +harvest:Harvest Apples, +pick_up, +queue_for_facility_use, +stage_stock_for_sale, +steal, +unstage_stock
**Affordance changes** (tick 2): +drink, +drop_item, +eat, +put_down, +store_stock, -pick_up, -steal
**Affordance changes** (tick 4): +pick_up, +steal
**Affordance changes** (tick 65): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 87): +drink, +drop_item, +put_down, +store_stock, +wash
**Affordance changes** (tick 168): -drink, -drop_item, -put_down, -store_stock, -wash
**Affordance changes** (tick 191): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 213): -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 237): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 248): +ask_witness, +bribe, +tell, -harvest:Harvest Apples, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 258): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 279): +queue_for_facility_use, -ask_witness, -collect_display_stock, -pick_up, -relieve_wilderness, -stage_stock_for_sale, -steal, -tell, -unstage_stock (at Thornwall Village)
**Affordance changes** (tick 282): +harvest:Harvest Apples, +relieve_wilderness (at Eldergrove Forest)
**Affordance changes** (tick 502): +collect_display_stock, +pick_up, +stage_stock_for_sale, +steal, +unstage_stock
**Affordance changes** (tick 503): +drink, +drop_item, +eat, +put_down, +store_stock
**Affordance changes** (tick 505): -pick_up, -steal
**Affordance changes** (tick 510): -collect_display_stock, -stage_stock_for_sale, -unstage_stock
**Affordance changes** (tick 514): +ask_witness, +bribe, +tell, -harvest:Harvest Apples, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 520): +queue_for_facility_use, -relieve_wilderness (at Thornwall Village)
**Affordance changes** (tick 522): +relieve_wilderness, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 528): +queue_for_facility_use, -relieve_wilderness (at Thornwall Village)
**Affordance changes** (tick 530): +relieve_wilderness, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 536): +queue_for_facility_use, -relieve_wilderness (at Thornwall Village)
**Affordance changes** (tick 538): +relieve_wilderness, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 542): -bribe, -drink, -drop_item, -eat, -put_down, -store_stock
**Affordance changes** (tick 586): +queue_for_care_target
**Final affordances** (tick 599)

- sleep
- relieve_wilderness
- queue_for_care_target (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- declare_support
- travel (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)

### Guard Theron (600 decision ticks)

**Tick breakdown**: 304 planning, 296 active-action, 0 dead
**Plan search outcomes**: 276 found, 65 frontier-exhausted, 2 budget-exhausted, 0 unsupported
**Goals selected**: AcquireCommodity { commodity: Grain, purpose: SelfConsume }, AcquireCommodity { commodity: Water, purpose: SelfConsume }, ConsumeOwnedCommodity { commodity: Grain }, ConsumeOwnedCommodity { commodity: Water }, InvestigateViolation { violation_id: ViolationId(0), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(1), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(10), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(11), place: EntityId { slot: 2, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(12), place: EntityId { slot: 2, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(13), place: EntityId { slot: 2, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(14), place: EntityId { slot: 2, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(2), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(3), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(4), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(5), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(6), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(7), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(8), place: EntityId { slot: 0, generation: 0 } }, InvestigateViolation { violation_id: ViolationId(9), place: EntityId { slot: 0, generation: 0 } }, Patrol { place: EntityId { slot: 0, generation: 0 } }, Patrol { place: EntityId { slot: 2, generation: 0 } }, Relieve, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 6, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: SocialObservation { observation: SocialObservation { detail: WitnessedAbsence { missing_entity: EntityId { slot: 5, generation: 0 }, expected_place: EntityId { slot: 2, generation: 0 } }, place: EntityId { slot: 2, generation: 0 }, observed_tick: Tick(543), source: DirectObservation } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 49, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 5, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 52, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 53, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 58, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 7, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 77, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 96, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 97, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 99, generation: 0 } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: SocialObservation { observation: SocialObservation { detail: WitnessedAbsence { missing_entity: EntityId { slot: 61, generation: 0 }, expected_place: EntityId { slot: 0, generation: 0 } }, place: EntityId { slot: 0, generation: 0 }, observed_tick: Tick(273), source: DirectObservation } }, communication_class: Testimony }, ShareBelief { listener: EntityId { slot: 7, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }, Sleep, Wash

**Decision timeline** (100-tick bins)

| Ticks | Decisions |
|-------|-----------|
| 0–99 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none; ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none (×2); ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ... and 59 more |
| 100–199 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0, frame=[resumed]; ACTIVE: investigate — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore Sleep@none>InvestigateViolation [InvestigateViolation { violation_id: ViolationId(6), place: EntityId { slot: 0, generation: 0 } }]@place:e0g0 (×2); ACTIVE: investigate — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore Sleep@none>InvestigateViolation [InvestigateViolation { violation_id: ViolationId(6), place: EntityId { slot: 0, generation: 0 } }]@place:e0g0, frame=[resumed]; ... and 54 more |
| 200–299 | ACTIVE: eat — interrupt: NoInterrupt (×3); ACTIVE: investigate — interrupt: NoInterrupt (×3); ACTIVE: investigate — interrupt: NoInterrupt, frame=[resumed]; ACTIVE: patrol — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none>Patrol [Patrol { place: EntityId { slot: 0, generation: 0 } }]@place:e0g0; ACTIVE: patrol — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore Sleep@none>Patrol [Patrol { place: EntityId { slot: 0, generation: 0 } }]@place:e0g0 (×4); ... and 31 more |
| 300–399 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: patrol — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>Patrol [Patrol { place: EntityId { slot: 0, generation: 0 } }]@place:e0g0; ... and 61 more |
| 400–499 | ACTIVE: eat — interrupt: NoInterrupt (×2); ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0 (×2); ACTIVE: investigate — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>InvestigateViolation [InvestigateViolation { violation_id: ViolationId(9), place: EntityId { slot: 0, generation: 0 } }]@place:e0g0, frame=[resumed]; ACTIVE: investigate — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>InvestigateViolation [InvestigateViolation { violation_id: ViolationId(11), place: EntityId { slot: 2, generation: 0 } }]@place:e2g0; ... and 67 more |
| 500–599 | ACTIVE: eat — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>ConsumeOwnedCommodity [ConsumeOwnedCommodity { commodity: Grain }]@none (×3); ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0; ACTIVE: harvest:Harvest Water — interrupt: NoInterrupt, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e6g0>AcquireCommodity(SelfConsume) [AcquireCommodity { commodity: Water, purpose: SelfConsume }]@place:e0g0, frame=[resumed]; ACTIVE: investigate — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>InvestigateViolation [InvestigateViolation { violation_id: ViolationId(12), place: EntityId { slot: 2, generation: 0 } }]@place:e2g0 (×2); ACTIVE: investigate — interrupt: InterruptForReplan { trigger: SuperiorSameClassPlan }, ranking=MotiveScore ShareBelief(Testimony) [ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony }]@entity:e5g0>InvestigateViolation [InvestigateViolation { violation_id: ViolationId(13), place: EntityId { slot: 2, generation: 0 } }]@place:e2g0 (×2); ... and 67 more |

**Failed plan attempts** (showing first 20 of 67)

| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |
|------|------|---------|------------|-----------|------------|----------|--------------------|
| 20 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e0g0 | true |
| 22 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e0g0 | true |
| 58 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 58 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 59 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 59 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 60 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 92 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 92 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 93 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 93 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 94 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 94 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 125 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 125 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 126 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 126 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 127 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 127 | ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |
| 146 | ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | frontier-exhausted at depth 0: 3 candidates generated, all skipped (build_successor returned None) | 1 | 0 | 3 | e2g0 | true |

### Failed Plan Frequency Breakdown
- frontier-exhausted: 20 / 20
- budget-exhausted: 0 / 20
- Max Depth = 0 (no operators available): 20 / 20
- Had Target Beliefs = false: 0 / 20

**Fully blocked desires** (goal generated but all opportunities blocked)

| Goal | Times Blocked |
|------|---------------|
| InvestigateViolation { violation_id: ViolationId(11), place: EntityId { slot: 2, generation: 0 } } | 3 |
| InvestigateViolation { violation_id: ViolationId(12), place: EntityId { slot: 2, generation: 0 } } | 10 |
| InvestigateViolation { violation_id: ViolationId(13), place: EntityId { slot: 2, generation: 0 } } | 19 |
| InvestigateViolation { violation_id: ViolationId(14), place: EntityId { slot: 2, generation: 0 } } | 7 |
| InvestigateViolation { violation_id: ViolationId(4), place: EntityId { slot: 0, generation: 0 } } | 2 |
| InvestigateViolation { violation_id: ViolationId(6), place: EntityId { slot: 0, generation: 0 } } | 8 |
| InvestigateViolation { violation_id: ViolationId(9), place: EntityId { slot: 0, generation: 0 } } | 4 |
| Patrol { place: EntityId { slot: 0, generation: 0 } } | 31 |
| Patrol { place: EntityId { slot: 2, generation: 0 } } | 18 |
| ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | 19 |
| ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 5 |
| ShareBelief { listener: EntityId { slot: 5, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | 17 |
| ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 0, generation: 0 } }, communication_class: Testimony } | 11 |
| ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 19, generation: 0 } }, communication_class: Testimony } | 10 |
| ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 2, generation: 0 } }, communication_class: Testimony } | 4 |
| ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 20, generation: 0 } }, communication_class: Testimony } | 21 |
| ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: EntityBelief { subject: EntityId { slot: 21, generation: 0 } }, communication_class: Testimony } | 17 |
| ShareBelief { listener: EntityId { slot: 6, generation: 0 }, topic: SocialObservation { observation: SocialObservation { detail: WitnessedAbsence { missing_entity: EntityId { slot: 61, generation: 0 }, expected_place: EntityId { slot: 0, generation: 0 } }, place: EntityId { slot: 0, generation: 0 }, observed_tick: Tick(273), source: DirectObservation } }, communication_class: Testimony } | 2 |

**Affordances available at tick 0** (at e2g0)

- sleep
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)

**Affordances after travel** (tick 9, arrived at Thornwall Village)

- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- patrol (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
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
- store_stock (1 targets)
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

**Affordances after travel** (tick 46, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 62, arrived at Thornwall Village)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- investigate (1 targets)
- investigate (1 targets)
- investigate (1 targets)
- investigate (1 targets)
- investigate (1 targets)
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 84, arrived at Dusty Trail)

- eat (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 98, arrived at Thornwall Village)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- investigate (1 targets)
- investigate (1 targets)
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 119, arrived at Dusty Trail)

- eat (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- patrol (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
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

**Affordances after travel** (tick 133, arrived at Thornwall Village)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 139, arrived at Dusty Trail)

- eat (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- patrol (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
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

**Affordances after travel** (tick 152, arrived at Thornwall Village)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 187, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 206, arrived at Thornwall Village)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 248, arrived at Dusty Trail)

- eat (1 targets)
- sleep
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 270, arrived at Thornwall Village)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 360, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
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
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 411, arrived at Thornwall Village)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- investigate (1 targets)
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordances after travel** (tick 439, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
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
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 465, arrived at Thornwall Village)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- patrol (1 targets)
- fine (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 503, arrived at Dusty Trail)

- eat (1 targets)
- drink (1 targets)
- sleep
- wash (1 targets)
- relieve_wilderness
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- defend
- investigate (1 targets)
- patrol (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
- ask_witness (1 targets)
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
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

**Affordances after travel** (tick 585, arrived at Thornwall Village)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- collect_display_stock (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- stage_stock_for_sale (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)
- unstage_stock (1 targets)

**Affordance changes** (tick 7): +collect_display_stock, +stage_stock_for_sale, +unstage_stock, -patrol
**Affordance changes** (tick 9): +ask_witness, +bribe, +harvest:Harvest Water, +patrol, +pick_up, +queue_for_facility_use, +steal, +tell, +threaten, -relieve_wilderness (at Thornwall Village)
**Affordance changes** (tick 15): +investigate
**Affordance changes** (tick 30): +drink, +wash
**Affordance changes** (tick 36): +eat, -pick_up, -steal
**Affordance changes** (tick 46): +pick_up, +relieve_wilderness, +steal, -harvest:Harvest Water, -investigate, -patrol, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 62): +harvest:Harvest Water, +investigate, +patrol, +queue_for_facility_use, -ask_witness, -bribe, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 73): +pick_up, +steal, -drink, -wash
**Affordance changes** (tick 84): +ask_witness, +bribe, +relieve_wilderness, +tell, +threaten, -harvest:Harvest Water, -investigate, -patrol, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 98): +harvest:Harvest Water, +investigate, +patrol, +queue_for_facility_use, -ask_witness, -bribe, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 114): -patrol
**Affordance changes** (tick 119): +ask_witness, +bribe, +patrol, +pick_up, +relieve_wilderness, +steal, +tell, +threaten, -harvest:Harvest Water, -investigate, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 133): +harvest:Harvest Water, +investigate, +queue_for_facility_use, -ask_witness, -bribe, -patrol, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 139): +ask_witness, +bribe, +patrol, +pick_up, +relieve_wilderness, +steal, +tell, +threaten, -harvest:Harvest Water, -investigate, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 146): -patrol
**Affordance changes** (tick 152): +harvest:Harvest Water, +investigate, +patrol, +queue_for_facility_use, -ask_witness, -bribe, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 155): +ask_witness, +bribe, +pick_up, +steal, +tell, +threaten
**Affordance changes** (tick 173): -investigate
**Affordance changes** (tick 179): -ask_witness, -bribe, -tell, -threaten
**Affordance changes** (tick 180): +drink, +investigate, +wash, -pick_up, -steal
**Affordance changes** (tick 187): +ask_witness, +bribe, +pick_up, +relieve_wilderness, +steal, +tell, +threaten, -harvest:Harvest Water, -investigate, -patrol, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 206): +harvest:Harvest Water, +investigate, +patrol, +queue_for_facility_use, -ask_witness, -bribe, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 209): -investigate
**Affordance changes** (tick 246): +pick_up, +steal, -drink, -wash
**Affordance changes** (tick 248): +ask_witness, +bribe, +relieve_wilderness, +tell, +threaten, -harvest:Harvest Water, -patrol, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 270): +harvest:Harvest Water, +investigate, +patrol, +queue_for_facility_use, -ask_witness, -bribe, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 274): -investigate
**Affordance changes** (tick 310): +ask_witness, +bribe, +tell, +threaten
**Affordance changes** (tick 318): +steal
**Affordance changes** (tick 324): -steal
**Affordance changes** (tick 339): +pick_up, +steal
**Affordance changes** (tick 343): +investigate
**Affordance changes** (tick 355): +drink, +wash, -pick_up, -steal
**Affordance changes** (tick 360): +pick_up, +relieve_wilderness, +steal, -ask_witness, -bribe, -harvest:Harvest Water, -investigate, -patrol, -queue_for_facility_use, -tell, -threaten (at Dusty Trail)
**Affordance changes** (tick 368): +ask_witness, +bribe, +investigate, +tell, +threaten
**Affordance changes** (tick 404): -drink, -wash
**Affordance changes** (tick 411): +harvest:Harvest Water, +patrol, +queue_for_facility_use, -ask_witness, -bribe, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 416): +ask_witness, +bribe, +tell, +threaten
**Affordance changes** (tick 423): -ask_witness, -bribe, -tell, -threaten
**Affordance changes** (tick 424): +ask_witness, +bribe, +tell, +threaten
**Affordance changes** (tick 430): +pick_up, +steal
**Affordance changes** (tick 434): +drink, +wash, -pick_up, -steal
**Affordance changes** (tick 437): -ask_witness, -bribe, -tell, -threaten
**Affordance changes** (tick 439): +ask_witness, +bribe, +pick_up, +relieve_wilderness, +steal, +tell, +threaten, -harvest:Harvest Water, -patrol, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 465): +harvest:Harvest Water, +patrol, +queue_for_facility_use, -ask_witness, -bribe, -investigate, -pick_up, -relieve_wilderness, -steal, -tell, -threaten (at Thornwall Village)
**Affordance changes** (tick 470): +ask_witness, +bribe, +tell, +threaten
**Affordance changes** (tick 490): -tell
**Affordance changes** (tick 500): -patrol
**Affordance changes** (tick 503): +investigate, +patrol, +pick_up, +relieve_wilderness, +steal, +tell, -harvest:Harvest Water, -queue_for_facility_use (at Dusty Trail)
**Affordance changes** (tick 514): -drink, -wash
**Affordance changes** (tick 585): +harvest:Harvest Water, +queue_for_facility_use, -investigate, -patrol, -pick_up, -relieve_wilderness, -steal (at Thornwall Village)
**Affordance changes** (tick 588): +pick_up, +steal
**Final affordances** (tick 598)

- eat (1 targets)
- sleep
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- queue_for_facility_use (1 targets)
- harvest:Harvest Water (1 targets)
- staff_market
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- tell (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- bribe (1 targets)
- threaten (1 targets)
- threaten (1 targets)
- declare_support
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- travel (1 targets)
- pick_up (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- put_down (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- drop_item (1 targets)
- steal (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
- attack (1 targets)
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
- fine (1 targets)
- fine (1 targets)
- fine (1 targets)
- exile (1 targets)
- exile (1 targets)
- exile (1 targets)
- store_stock (1 targets)
- store_stock (1 targets)
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

## Section 8 — Budget Exhaustion Snapshots

19 unique budget-exhaustion signatures captured (deduplicated by agent+goal+location).

### Snapshot 1 — Merchant Vara at tick 35

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Bread, purpose: SelfConsume }`
**Location**: Dusty Trail (e2g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 693

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=278, thirst=54, fatigue=182, bladder=32, dirtiness=138

**Agent inventory**:
- 1× Water

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael, Merchant Vara, 4× Water, 3× Bread, 1× Waste, 1× Waste
- Thornwall Village: Guard Theron, Mill, Loom, Well

**Current place contents**:
- Kael (agent)
- Merchant Vara (agent)
- 3× Bread
- 20× Coin
- 2× Waste
- 5× Water

**Adjacent place contents**:
- Thornwall Village: Guard Theron (agent), Mill (Mill), Loom (Loom), Well (Well), 1× Bow, 10× Grain, 1× Sword, 1× Water

### Snapshot 2 — Merchant Vara at tick 35

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`
**Location**: Dusty Trail (e2g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 693

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=278, thirst=54, fatigue=182, bladder=32, dirtiness=138

**Agent inventory**:
- 1× Water

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael, Merchant Vara, 4× Water, 3× Bread, 1× Waste, 1× Waste
- Thornwall Village: Guard Theron, Mill, Loom, Well

**Current place contents**:
- Kael (agent)
- Merchant Vara (agent)
- 3× Bread
- 20× Coin
- 2× Waste
- 5× Water

**Adjacent place contents**:
- Thornwall Village: Guard Theron (agent), Mill (Mill), Loom (Loom), Well (Well), 1× Bow, 10× Grain, 1× Sword, 1× Water

### Snapshot 3 — Merchant Vara at tick 35

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Grain, purpose: SelfConsume }`
**Location**: Dusty Trail (e2g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 693

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=278, thirst=54, fatigue=182, bladder=32, dirtiness=138

**Agent inventory**:
- 1× Water

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael, Merchant Vara, 4× Water, 3× Bread, 1× Waste, 1× Waste
- Thornwall Village: Guard Theron, Mill, Loom, Well

**Current place contents**:
- Kael (agent)
- Merchant Vara (agent)
- 3× Bread
- 20× Coin
- 2× Waste
- 5× Water

**Adjacent place contents**:
- Thornwall Village: Guard Theron (agent), Mill (Mill), Loom (Loom), Well (Well), 1× Bow, 10× Grain, 1× Sword, 1× Water

### Snapshot 4 — Guard Theron at tick 149

**Agent**: Guard Theron (e8g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Dusty Trail (e2g0)

**Search metrics**:
- Expansions used: 224
- Max depth reached: 7
- Total candidates generated: 519

**Planner configuration**:
- max_node_expansions: 224
- max_plan_depth: 8
- max_candidates_per_expansion: 200
- max_prerequisite_locations: 3
- beam_width: 8
- preferred_operator_boost: 2

**Agent needs** (‰):
- hunger=246, thirst=234, fatigue=290, bladder=236, dirtiness=252

**Agent inventory**:
- 1× Bow
- 8× Grain
- 1× Sword

**Beliefs** (16 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael, Merchant Vara, Guard Theron, 1× Bow, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste
- Thornwall Village: Mill, Loom, Well

**Current place contents**:
- Kael (agent)
- Merchant Vara (agent)
- Guard Theron (agent)
- 1× Bow
- 2× Bread
- 20× Coin
- 8× Grain
- 1× Sword
- 7× Waste
- 2× Water

**Adjacent place contents**:
- Thornwall Village: Mill (Mill), Loom (Loom), Well (Well)

### Snapshot 5 — Merchant Vara at tick 155

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Bread, purpose: SelfConsume }`
**Location**: Thornwall Village (e0g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 705

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=518, thirst=213, fatigue=302, bladder=252, dirtiness=258

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael
- Thornwall Village: Merchant Vara, Guard Theron, 8× Grain, 1× Sword, 1× Bow, Mill, Loom, Well, 2× Water

**Current place contents**:
- Merchant Vara (agent)
- Guard Theron (agent)
- Mill (Mill)
- Loom (Loom)
- Well (Well)
- 1× Bow
- 8× Grain
- 1× Sword
- 2× Water

**Adjacent place contents**:
- Dusty Trail: Kael (agent), 2× Bread, 20× Coin, 7× Waste, 2× Water
- Eldergrove Forest: Forager Lina (agent), ChoppingBlock (ChoppingBlock), OrchardRow (OrchardRow), 4× Waste, 1× Water
- Golden Fields: FieldPlot (FieldPlot), GravePlot (GravePlot)
- Hearthstone Inn: Forge (Forge), WashBasin (WashBasin), 3× Firewood, 2× Medicine

### Snapshot 6 — Merchant Vara at tick 155

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`
**Location**: Thornwall Village (e0g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 705

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=518, thirst=213, fatigue=302, bladder=252, dirtiness=258

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael
- Thornwall Village: Merchant Vara, Guard Theron, 8× Grain, 1× Sword, 1× Bow, Mill, Loom, Well, 2× Water

**Current place contents**:
- Merchant Vara (agent)
- Guard Theron (agent)
- Mill (Mill)
- Loom (Loom)
- Well (Well)
- 1× Bow
- 8× Grain
- 1× Sword
- 2× Water

**Adjacent place contents**:
- Dusty Trail: Kael (agent), 2× Bread, 20× Coin, 7× Waste, 2× Water
- Eldergrove Forest: Forager Lina (agent), ChoppingBlock (ChoppingBlock), OrchardRow (OrchardRow), 4× Waste, 1× Water
- Golden Fields: FieldPlot (FieldPlot), GravePlot (GravePlot)
- Hearthstone Inn: Forge (Forge), WashBasin (WashBasin), 3× Firewood, 2× Medicine

### Snapshot 7 — Merchant Vara at tick 155

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Grain, purpose: SelfConsume }`
**Location**: Thornwall Village (e0g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 705

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=518, thirst=213, fatigue=302, bladder=252, dirtiness=258

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael
- Thornwall Village: Merchant Vara, Guard Theron, 8× Grain, 1× Sword, 1× Bow, Mill, Loom, Well, 2× Water

**Current place contents**:
- Merchant Vara (agent)
- Guard Theron (agent)
- Mill (Mill)
- Loom (Loom)
- Well (Well)
- 1× Bow
- 8× Grain
- 1× Sword
- 2× Water

**Adjacent place contents**:
- Dusty Trail: Kael (agent), 2× Bread, 20× Coin, 7× Waste, 2× Water
- Eldergrove Forest: Forager Lina (agent), ChoppingBlock (ChoppingBlock), OrchardRow (OrchardRow), 4× Waste, 1× Water
- Golden Fields: FieldPlot (FieldPlot), GravePlot (GravePlot)
- Hearthstone Inn: Forge (Forge), WashBasin (WashBasin), 3× Firewood, 2× Medicine

### Snapshot 8 — Merchant Vara at tick 346

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Bread, purpose: SelfConsume }`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 693

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=912, thirst=18, fatigue=384, bladder=616, dirtiness=13

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Merchant Vara, 1× Waste, 1× Waste, 1× Waste
- Thornwall Village: Kael, Guard Theron, Mill, Loom, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 9 — Merchant Vara at tick 346

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 693

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=912, thirst=18, fatigue=384, bladder=616, dirtiness=13

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Merchant Vara, 1× Waste, 1× Waste, 1× Waste
- Thornwall Village: Kael, Guard Theron, Mill, Loom, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 10 — Merchant Vara at tick 346

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Grain, purpose: SelfConsume }`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 693

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=912, thirst=18, fatigue=384, bladder=616, dirtiness=13

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Merchant Vara, 1× Waste, 1× Waste, 1× Waste
- Thornwall Village: Kael, Guard Theron, Mill, Loom, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 11 — Kael at tick 440

**Agent**: Kael (e5g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 224
- Max depth reached: 7
- Total candidates generated: 519

**Planner configuration**:
- max_node_expansions: 224
- max_plan_depth: 8
- max_candidates_per_expansion: 200
- max_prerequisite_locations: 3
- beam_width: 8
- preferred_operator_boost: 2

**Agent needs** (‰):
- hunger=100, thirst=203, fatigue=292, bladder=132, dirtiness=81

**Agent inventory**:
- 20× Coin

**Beliefs** (16 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Kael, Merchant Vara, Guard Theron, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Waste, 1× Water
- Thornwall Village: Mill, Loom, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 12 — Merchant Vara at tick 467

**Agent**: Merchant Vara (e6g0)
**Goal**: `Sleep`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 977

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=1000, thirst=381, fatigue=626, bladder=12, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Kael, Merchant Vara, Guard Theron, 1× Waste, 1× Water, 1× Waste
- Thornwall Village: Mill, Loom, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 13 — Merchant Vara at tick 467

**Agent**: Merchant Vara (e6g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 694

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=1000, thirst=381, fatigue=626, bladder=12, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Kael, Merchant Vara, Guard Theron, 1× Waste, 1× Water, 1× Waste
- Thornwall Village: Mill, Loom, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 14 — Merchant Vara at tick 472

**Agent**: Merchant Vara (e6g0)
**Goal**: `Sleep`
**Location**: Thornwall Village (e0g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 8
- Total candidates generated: 1013

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=1000, thirst=398, fatigue=641, bladder=32, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Kael
- Thornwall Village: Merchant Vara, Guard Theron, 1× Sword, 1× Bow, Mill, Loom, Well, 1× Water

**Current place contents**:
- Merchant Vara (agent)
- Guard Theron (agent)
- Mill (Mill)
- Loom (Loom)
- Well (Well)
- 1× Bow
- 4× Grain
- 1× Sword
- 1× Water

**Adjacent place contents**:
- Dusty Trail: Kael (agent), 20× Coin, 19× Waste, 1× Water
- Eldergrove Forest: Forager Lina (agent), ChoppingBlock (ChoppingBlock), OrchardRow (OrchardRow), 2× Apple, 5× Waste
- Golden Fields: FieldPlot (FieldPlot), GravePlot (GravePlot)
- Hearthstone Inn: Forge (Forge), WashBasin (WashBasin), 3× Firewood, 2× Medicine

### Snapshot 15 — Merchant Vara at tick 472

**Agent**: Merchant Vara (e6g0)
**Goal**: `ProduceCommodity { recipe_id: RecipeId(2) }`
**Location**: Thornwall Village (e0g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 757

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=1000, thirst=398, fatigue=641, bladder=32, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Kael
- Thornwall Village: Merchant Vara, Guard Theron, 1× Sword, 1× Bow, Mill, Loom, Well, 1× Water

**Current place contents**:
- Merchant Vara (agent)
- Guard Theron (agent)
- Mill (Mill)
- Loom (Loom)
- Well (Well)
- 1× Bow
- 4× Grain
- 1× Sword
- 1× Water

**Adjacent place contents**:
- Dusty Trail: Kael (agent), 20× Coin, 19× Waste, 1× Water
- Eldergrove Forest: Forager Lina (agent), ChoppingBlock (ChoppingBlock), OrchardRow (OrchardRow), 2× Apple, 5× Waste
- Golden Fields: FieldPlot (FieldPlot), GravePlot (GravePlot)
- Hearthstone Inn: Forge (Forge), WashBasin (WashBasin), 3× Firewood, 2× Medicine

### Snapshot 16 — Merchant Vara at tick 472

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Water, purpose: SelfConsume }`
**Location**: Thornwall Village (e0g0)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 757

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=1000, thirst=398, fatigue=641, bladder=32, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail, Forager Lina
- Dusty Trail: Kael
- Thornwall Village: Merchant Vara, Guard Theron, 1× Sword, 1× Bow, Mill, Loom, Well, 1× Water

**Current place contents**:
- Merchant Vara (agent)
- Guard Theron (agent)
- Mill (Mill)
- Loom (Loom)
- Well (Well)
- 1× Bow
- 4× Grain
- 1× Sword
- 1× Water

**Adjacent place contents**:
- Dusty Trail: Kael (agent), 20× Coin, 19× Waste, 1× Water
- Eldergrove Forest: Forager Lina (agent), ChoppingBlock (ChoppingBlock), OrchardRow (OrchardRow), 2× Apple, 5× Waste
- Golden Fields: FieldPlot (FieldPlot), GravePlot (GravePlot)
- Hearthstone Inn: Forge (Forge), WashBasin (WashBasin), 3× Firewood, 2× Medicine

### Snapshot 17 — Forager Lina at tick 543

**Agent**: Forager Lina (e7g0)
**Goal**: `Sleep`
**Location**: Dusty Trail (e2g0)

**Search metrics**:
- Expansions used: 224
- Max depth reached: 7
- Total candidates generated: 726

**Planner configuration**:
- max_node_expansions: 224
- max_plan_depth: 8
- max_candidates_per_expansion: 200
- max_prerequisite_locations: 3
- beam_width: 8
- preferred_operator_boost: 2

**Agent needs** (‰):
- hunger=140, thirst=785, fatigue=838, bladder=104, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Eldergrove Forest, Dusty Trail
- Dusty Trail: Kael, Forager Lina, Guard Theron
- Eldergrove Forest: ChoppingBlock, OrchardRow
- Thornwall Village: Merchant Vara, Mill, Loom, Well

**Current place contents**:
- Kael (agent)
- Forager Lina (agent)
- Guard Theron (agent)
- 1× Bow
- 20× Coin
- 3× Grain
- 1× Sword
- 22× Waste

**Adjacent place contents**:
- Thornwall Village: Merchant Vara (agent), Mill (Mill), Loom (Loom), Well (Well)

### Snapshot 18 — Merchant Vara at tick 565

**Agent**: Merchant Vara (e6g0)
**Goal**: `AcquireCommodity { commodity: Water, purpose: SelfConsume }`
**Location**: Unknown#4294967295 (e4294967295g4294967295)

**Search metrics**:
- Expansions used: 300
- Max depth reached: 9
- Total candidates generated: 757

**Planner configuration**:
- max_node_expansions: 300
- max_plan_depth: 10
- max_candidates_per_expansion: 150
- max_prerequisite_locations: 3
- beam_width: 10
- preferred_operator_boost: 3

**Agent needs** (‰):
- hunger=1000, thirst=707, fatigue=902, bladder=404, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Dusty Trail
- Dusty Trail: Kael, Merchant Vara, Forager Lina, Guard Theron, 1× Waste, 1× Waste, 1× Waste
- Thornwall Village: Mill, Loom, Well

**Current place contents**:
- (empty)

**Adjacent place contents**:
- (none)

### Snapshot 19 — Forager Lina at tick 586

**Agent**: Forager Lina (e7g0)
**Goal**: `TreatWounds { patient: EntityId { slot: 7, generation: 0 } }`
**Location**: Dusty Trail (e2g0)

**Search metrics**:
- Expansions used: 224
- Max depth reached: 7
- Total candidates generated: 758

**Planner configuration**:
- max_node_expansions: 224
- max_plan_depth: 8
- max_candidates_per_expansion: 200
- max_prerequisite_locations: 3
- beam_width: 8
- preferred_operator_boost: 2

**Agent needs** (‰):
- hunger=226, thirst=1000, fatigue=924, bladder=276, dirtiness=1000

**Agent inventory**:
- (empty)

**Beliefs** (12 known entities):
- (unknown): Thornwall Village, Eldergrove Forest, Dusty Trail
- Dusty Trail: Kael, Merchant Vara, Forager Lina, Guard Theron
- Eldergrove Forest: ChoppingBlock, OrchardRow
- Thornwall Village: Mill, Loom, Well

**Current place contents**:
- Forager Lina (agent)
- 22× Waste

**Adjacent place contents**:
- Thornwall Village: Kael (agent), Merchant Vara (agent), Guard Theron (agent), Mill (Mill), Loom (Loom), Well (Well), 1× Bow, 20× Coin, 3× Grain, 1× Sword

