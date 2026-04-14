# Soak Performance Profile Report

**Date**: 2026-04-14
**Branch**: implemented-spec-S102
**Seed**: 0
**Ticks profiled**: 500
**Total wall time**: 30.89s
**Agents**: 20
**Places**: 10

## Tick Timing Summary

| Metric | Value |
|--------|-------|
| Avg ms/tick | 61.76 |
| Min ms/tick | 5.34 |
| Max ms/tick | 208.39 |
| P50 ms/tick | 63.17 |
| P90 ms/tick | 89.69 |
| P99 ms/tick | 161.02 |

### Growth Analysis

| Period | Avg ms/tick |
|--------|-------------|
| First 50 ticks | 17.78 |
| Last 50 ticks | 75.16 |
| Growth ratio | 4.23x |

**WARNING**: Tick cost grew 4.2x over 500 ticks. This suggests O(n) or worse scaling with simulation age.

## Per-Tick Timing (sampled every 10 ticks)

| Tick | ms | EventLog | KnownEntities(all) | Claims(all) | Social(all) | Told(all) | Institutional(all) |
|------|-----|----------|---------------------|-------------|-------------|-----------|---------------------|
| 0 | 12.5 | 149 | 89 | 186 | 0 | 0 | 3 |
| 10 | 12.5 | 660 | 200 | 4345 | 187 | 55 | 3 |
| 20 | 17.6 | 1056 | 231 | 8378 | 335 | 93 | 3 |
| 30 | 23.6 | 1467 | 255 | 11763 | 392 | 109 | 3 |
| 40 | 20.6 | 1861 | 260 | 15185 | 450 | 120 | 3 |
| 50 | 25.2 | 2285 | 263 | 19003 | 526 | 130 | 3 |
| 60 | 30.0 | 2759 | 263 | 24642 | 773 | 124 | 3 |
| 70 | 40.0 | 3206 | 264 | 29002 | 909 | 121 | 3 |
| 80 | 46.6 | 3663 | 253 | 31371 | 1010 | 131 | 3 |
| 90 | 54.5 | 4130 | 261 | 30921 | 1129 | 136 | 3 |
| 100 | 43.9 | 4613 | 254 | 31611 | 1247 | 144 | 3 |
| 110 | 54.4 | 5294 | 278 | 33826 | 1473 | 136 | 3 |
| 120 | 51.4 | 5872 | 283 | 36224 | 1690 | 140 | 3 |
| 130 | 54.5 | 6360 | 294 | 36710 | 1834 | 143 | 3 |
| 140 | 45.0 | 6866 | 278 | 36817 | 1979 | 143 | 3 |
| 150 | 44.3 | 7436 | 280 | 38180 | 2153 | 145 | 3 |
| 160 | 49.4 | 8063 | 294 | 40447 | 2398 | 138 | 3 |
| 170 | 61.9 | 8641 | 298 | 42564 | 2625 | 138 | 3 |
| 180 | 68.7 | 9248 | 322 | 44933 | 2816 | 131 | 3 |
| 190 | 73.1 | 9646 | 403 | 48430 | 3053 | 129 | 3 |
| 200 | 63.2 | 10003 | 401 | 52333 | 3296 | 130 | 3 |
| 210 | 61.6 | 10382 | 407 | 55763 | 3506 | 125 | 3 |
| 220 | 88.0 | 10873 | 399 | 58331 | 3690 | 119 | 3 |
| 230 | 96.5 | 11396 | 407 | 60276 | 3862 | 122 | 3 |
| 240 | 83.7 | 11953 | 404 | 61410 | 4049 | 121 | 3 |
| 250 | 64.6 | 12717 | 407 | 61828 | 4197 | 121 | 3 |
| 260 | 64.0 | 13477 | 428 | 60744 | 4357 | 119 | 3 |
| 270 | 78.3 | 14219 | 436 | 57714 | 4489 | 117 | 3 |
| 280 | 74.0 | 14913 | 405 | 55366 | 4630 | 116 | 3 |
| 290 | 77.4 | 15514 | 389 | 53288 | 4680 | 114 | 3 |
| 300 | 120.2 | 16067 | 376 | 52886 | 4743 | 107 | 3 |
| 310 | 71.1 | 16605 | 381 | 52444 | 4836 | 99 | 3 |
| 320 | 146.3 | 17038 | 345 | 52376 | 4866 | 93 | 3 |
| 330 | 84.0 | 17279 | 342 | 52953 | 4889 | 81 | 3 |
| 340 | 141.7 | 17531 | 342 | 53047 | 4895 | 81 | 3 |
| 350 | 60.8 | 17744 | 342 | 52995 | 4895 | 81 | 3 |
| 360 | 128.0 | 18006 | 354 | 53581 | 4920 | 70 | 3 |
| 370 | 83.8 | 18350 | 456 | 55812 | 4953 | 41 | 3 |
| 380 | 144.4 | 18692 | 472 | 57702 | 4981 | 38 | 3 |
| 390 | 89.7 | 18972 | 473 | 58891 | 4987 | 38 | 3 |
| 400 | 176.7 | 19296 | 491 | 60784 | 4987 | 38 | 3 |
| 410 | 80.7 | 19594 | 505 | 62933 | 5000 | 29 | 3 |
| 420 | 167.1 | 19890 | 513 | 65421 | 5009 | 29 | 3 |
| 430 | 105.7 | 20239 | 522 | 67412 | 5022 | 26 | 3 |
| 440 | 208.4 | 20660 | 581 | 69405 | 5036 | 26 | 3 |
| 450 | 127.3 | 21046 | 573 | 69620 | 5078 | 32 | 3 |
| 460 | 175.4 | 21437 | 618 | 69740 | 5135 | 33 | 3 |
| 470 | 98.4 | 21775 | 623 | 69300 | 5151 | 33 | 3 |
| 480 | 161.0 | 22134 | 621 | 68421 | 5166 | 33 | 3 |
| 490 | 105.2 | 22561 | 620 | 67571 | 5190 | 34 | 3 |
| 499 | 80.4 | 22910 | 606 | 66553 | 5228 | 34 | 3 |

## Detailed Per-Agent Snapshots

### Tick 0

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 3 | 2 | 3 | 0 | 0 | 0 | 0 | T30RulersHall |  |
| T30Claimant1 | 5 | 5 | 11 | 0 | 0 | 0 | 0 | T30Hub |  |
| T30Claimant2 | 5 | 5 | 11 | 0 | 0 | 0 | 0 | T30Hub |  |
| T30Merchant | 7 | 6 | 13 | 0 | 0 | 0 | 0 | T30Market |  |
| T30Carrier | 7 | 7 | 25 | 0 | 0 | 0 | 0 | T30Farm |  |
| T30Guard1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | ??? |  |
| T30Guard2 | 6 | 0 | 0 | 0 | 0 | 0 | 0 | ??? |  |
| T30Guard3 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | ??? |  |
| T30Bandit1 | 4 | 3 | 6 | 0 | 0 | 0 | 1 | T30BanditCamp |  |
| T30Bandit2 | 3 | 2 | 4 | 0 | 0 | 0 | 1 | T30BanditCamp |  |
| T30Bandit3 | 3 | 1 | 2 | 0 | 0 | 0 | 1 | T30BanditCamp |  |
| T30Thief1 | 7 | 5 | 11 | 0 | 0 | 0 | 0 | T30Market |  |
| T30Thief2 | 5 | 5 | 11 | 0 | 0 | 0 | 0 | T30Hub |  |
| T30Civ1 | 5 | 5 | 10 | 0 | 0 | 0 | 0 | T30Hub |  |
| T30Civ2 | 7 | 3 | 8 | 0 | 0 | 0 | 0 | T30Market |  |
| T30Civ3 | 5 | 4 | 9 | 0 | 0 | 0 | 0 | T30Farm |  |
| T30Civ4 | 5 | 4 | 13 | 0 | 0 | 0 | 0 | T30Forge |  |
| T30Worker1 | 6 | 6 | 25 | 0 | 0 | 0 | 0 | T30Farm |  |
| T30Worker2 | 5 | 5 | 21 | 0 | 0 | 0 | 0 | T30Forge |  |
| T30Worker3 | 1 | 1 | 3 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 50

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 3 | 3 | 123 | 0 | 0 | 0 | 0 | T30RulersHall |  |
| T30Claimant1 | 9 | 9 | 715 | 28 | 8 | 10 | 0 | T30Hub |  |
| T30Claimant2 | 9 | 9 | 747 | 25 | 8 | 10 | 0 | T30Hub |  |
| T30Merchant | 14 | 14 | 1522 | 43 | 9 | 10 | 0 | T30Market |  |
| T30Carrier | 12 | 12 | 1102 | 22 | 8 | 9 | 0 | T30Farm |  |
| T30Guard1 | 26 | 26 | 1561 | 50 | 7 | 8 | 0 | T30Market |  |
| T30Guard2 | 30 | 30 | 378 | 11 | 1 | 5 | 0 | T30Market |  |
| T30Guard3 | 29 | 29 | 800 | 15 | 8 | 5 | 0 | T30Market |  |
| T30Bandit1 | 5 | 5 | 380 | 11 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 5 | 5 | 367 | 9 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 5 | 5 | 369 | 9 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 14 | 14 | 1626 | 47 | 5 | 6 | 0 | T30Market |  |
| T30Thief2 | 19 | 19 | 1430 | 40 | 12 | 9 | 0 | T30Market |  |
| T30Civ1 | 9 | 9 | 694 | 26 | 9 | 8 | 0 | T30Hub |  |
| T30Civ2 | 14 | 14 | 1596 | 49 | 8 | 5 | 0 | T30Market |  |
| T30Civ3 | 12 | 12 | 1072 | 25 | 10 | 7 | 0 | T30Farm |  |
| T30Civ4 | 16 | 16 | 1590 | 44 | 9 | 8 | 0 | T30Market |  |
| T30Worker1 | 13 | 13 | 1136 | 24 | 7 | 9 | 0 | T30Farm |  |
| T30Worker2 | 16 | 16 | 1609 | 48 | 9 | 9 | 0 | T30Market |  |
| T30Worker3 | 3 | 3 | 186 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 100

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 16 | 16 | 498 | 8 | 1 | 7 | 0 | T30Farm |  |
| T30Claimant1 | 11 | 11 | 1393 | 96 | 12 | 12 | 0 | T30Hub |  |
| T30Claimant2 | 11 | 11 | 1443 | 92 | 11 | 12 | 0 | T30Hub |  |
| T30Merchant | 14 | 14 | 2169 | 77 | 9 | 10 | 0 | T30Market |  |
| T30Carrier | 15 | 15 | 1649 | 47 | 8 | 7 | 0 | T30Farm |  |
| T30Guard1 | 14 | 14 | 2309 | 79 | 6 | 7 | 0 | T30Market |  |
| T30Guard2 | 27 | 27 | 1923 | 97 | 12 | 12 | 0 | T30Hub |  |
| T30Guard3 | 19 | 19 | 2152 | 97 | 12 | 12 | 0 | T30Hub |  |
| T30Bandit1 | 5 | 5 | 506 | 19 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 5 | 5 | 500 | 19 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 5 | 5 | 498 | 20 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 14 | 14 | 2377 | 78 | 5 | 6 | 0 | T30Market |  |
| T30Thief2 | 14 | 14 | 2396 | 74 | 6 | 3 | 0 | T30Market |  |
| T30Civ1 | 9 | 9 | 1423 | 97 | 12 | 12 | 0 | T30Hub |  |
| T30Civ2 | 14 | 14 | 2230 | 79 | 7 | 6 | 0 | T30Market |  |
| T30Civ3 | 15 | 15 | 1602 | 56 | 9 | 6 | 0 | T30Farm |  |
| T30Civ4 | 14 | 14 | 2256 | 78 | 8 | 6 | 0 | T30Market |  |
| T30Worker1 | 15 | 15 | 1693 | 53 | 8 | 6 | 0 | T30Farm |  |
| T30Worker2 | 14 | 14 | 2290 | 81 | 6 | 8 | 0 | T30Market |  |
| T30Worker3 | 3 | 3 | 304 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 150

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 25 | 25 | 3031 | 112 | 12 | 12 | 0 | T30Farm |  |
| T30Claimant1 | 10 | 10 | 1376 | 144 | 12 | 12 | 0 | T30Hub |  |
| T30Claimant2 | 23 | 23 | 1820 | 155 | 12 | 12 | 0 | T30Farm |  |
| T30Merchant | 13 | 13 | 1745 | 98 | 6 | 5 | 0 | T30Road |  |
| T30Carrier | 21 | 21 | 3118 | 139 | 12 | 12 | 0 | T30Farm |  |
| T30Guard1 | 12 | 12 | 1846 | 99 | 2 | 5 | 0 | T30Market |  |
| T30Guard2 | 22 | 22 | 2953 | 190 | 12 | 12 | 0 | T30Farm |  |
| T30Guard3 | 9 | 9 | 1450 | 148 | 12 | 12 | 0 | T30Hub |  |
| T30Bandit1 | 5 | 5 | 517 | 26 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 5 | 5 | 508 | 29 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 5 | 5 | 513 | 31 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 12 | 12 | 1961 | 98 | 5 | 4 | 0 | T30Market |  |
| T30Thief2 | 12 | 12 | 1954 | 90 | 3 | 1 | 0 | T30Market |  |
| T30Civ1 | 23 | 23 | 3097 | 194 | 12 | 12 | 0 | T30Farm |  |
| T30Civ2 | 12 | 12 | 1902 | 100 | 4 | 4 | 0 | T30Market |  |
| T30Civ3 | 21 | 21 | 3072 | 151 | 12 | 12 | 0 | T30Farm |  |
| T30Civ4 | 12 | 12 | 1848 | 97 | 1 | 4 | 0 | T30Market |  |
| T30Worker1 | 21 | 21 | 3170 | 150 | 12 | 12 | 0 | T30Farm |  |
| T30Worker2 | 12 | 12 | 1878 | 102 | 4 | 3 | 0 | T30Market |  |
| T30Worker3 | 5 | 5 | 421 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 200

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 24 | 24 | 4880 | 233 | 12 | 12 | 0 | T30Farm |  |
| T30Claimant1 | 25 | 25 | 3086 | 250 | 12 | 12 | 0 | T30Farm |  |
| T30Claimant2 | 25 | 25 | 4027 | 277 | 12 | 12 | 0 | T30Farm |  |
| T30Merchant | 22 | 22 | 1528 | 112 | 5 | 2 | 0 | T30Road |  |
| T30Carrier | 24 | 24 | 4679 | 257 | 12 | 10 | 0 | T30Farm |  |
| T30Guard1 | 22 | 22 | 1818 | 117 | 7 | 4 | 0 | T30Road |  |
| T30Guard2 | 24 | 24 | 4775 | 316 | 9 | 12 | 0 | T30Farm |  |
| T30Guard3 | 25 | 25 | 1217 | 171 | 6 | 6 | 0 | T30Road |  |
| T30Bandit1 | 8 | 8 | 636 | 36 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 8 | 8 | 612 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 8 | 8 | 648 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 22 | 22 | 1963 | 116 | 4 | 4 | 0 | T30Road |  |
| T30Thief2 | 22 | 22 | 2023 | 108 | 0 | 3 | 0 | T30Road |  |
| T30Civ1 | 24 | 24 | 4703 | 311 | 12 | 12 | 0 | T30Farm |  |
| T30Civ2 | 22 | 22 | 1940 | 120 | 0 | 3 | 0 | T30Road |  |
| T30Civ3 | 24 | 24 | 4777 | 272 | 12 | 12 | 0 | T30Farm |  |
| T30Civ4 | 22 | 22 | 1883 | 114 | 0 | 3 | 0 | T30Road |  |
| T30Worker1 | 24 | 24 | 4738 | 282 | 12 | 9 | 0 | T30Farm |  |
| T30Worker2 | 22 | 22 | 1990 | 124 | 3 | 2 | 0 | T30Road |  |
| T30Worker3 | 4 | 4 | 410 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 250

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 26 | 26 | 5117 | 340 | 12 | 12 | 0 | T30Farm |  |
| T30Claimant1 | 26 | 26 | 5045 | 352 | 12 | 12 | 0 | T30Farm |  |
| T30Claimant2 | 26 | 26 | 5040 | 378 | 12 | 11 | 0 | T30Farm |  |
| T30Merchant | 21 | 21 | 2510 | 116 | 3 | 3 | 0 | T30Road |  |
| T30Carrier | 26 | 26 | 5002 | 359 | 12 | 10 | 0 | T30Farm |  |
| T30Guard1 | 22 | 22 | 1245 | 143 | 12 | 12 | 0 | T30Market |  |
| T30Guard2 | 26 | 26 | 5026 | 419 | 0 | 11 | 0 | T30Farm |  |
| T30Guard3 | 22 | 22 | 1642 | 197 | 12 | 12 | 0 | T30Market |  |
| T30Bandit1 | 8 | 8 | 816 | 36 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 8 | 8 | 806 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 8 | 8 | 860 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 21 | 21 | 2626 | 120 | 0 | 3 | 0 | T30Road |  |
| T30Thief2 | 21 | 21 | 2616 | 111 | 0 | 3 | 0 | T30Road |  |
| T30Civ1 | 26 | 26 | 5062 | 423 | 12 | 9 | 0 | T30Farm |  |
| T30Civ2 | 21 | 21 | 2573 | 124 | 0 | 2 | 0 | T30Road |  |
| T30Civ3 | 26 | 26 | 5051 | 365 | 10 | 11 | 0 | T30Farm |  |
| T30Civ4 | 21 | 21 | 2553 | 118 | 0 | 3 | 0 | T30Road |  |
| T30Worker1 | 26 | 26 | 5120 | 389 | 9 | 11 | 0 | T30Farm |  |
| T30Worker2 | 21 | 21 | 2674 | 127 | 3 | 2 | 0 | T30Road |  |
| T30Worker3 | 5 | 5 | 444 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 300

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 27 | 27 | 4210 | 397 | 9 | 11 | 0 | T30Farm |  |
| T30Claimant1 | 27 | 27 | 4207 | 406 | 12 | 9 | 0 | T30Farm |  |
| T30Claimant2 | 27 | 27 | 4190 | 432 | 9 | 10 | 0 | T30Farm |  |
| T30Merchant | 20 | 20 | 2413 | 118 | 3 | 0 | 0 | T30Road |  |
| T30Carrier | 27 | 27 | 4260 | 417 | 12 | 9 | 0 | T30Farm |  |
| T30Guard1 | 5 | 5 | 659 | 174 | 12 | 12 | 0 | T30Market |  |
| T30Guard2 | 27 | 27 | 4261 | 485 | 0 | 9 | 0 | T30Farm |  |
| T30Guard3 | 10 | 10 | 652 | 229 | 12 | 12 | 0 | T30Market |  |
| T30Bandit1 | 7 | 7 | 795 | 36 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 7 | 7 | 817 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 7 | 7 | 797 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 20 | 20 | 2514 | 122 | 0 | 1 | 0 | T30Road |  |
| T30Thief2 | 20 | 20 | 2501 | 112 | 0 | 1 | 0 | T30Road |  |
| T30Civ1 | 27 | 27 | 4208 | 484 | 8 | 4 | 0 | T30Farm |  |
| T30Civ2 | 20 | 20 | 2411 | 126 | 0 | 1 | 0 | T30Road |  |
| T30Civ3 | 27 | 27 | 4200 | 423 | 6 | 7 | 0 | T30Farm |  |
| T30Civ4 | 19 | 19 | 2435 | 121 | 0 | 3 | 0 | T30Road |  |
| T30Worker1 | 27 | 27 | 4317 | 451 | 9 | 10 | 0 | T30Farm |  |
| T30Worker2 | 19 | 19 | 2510 | 130 | 3 | 2 | 0 | T30Road |  |
| T30Worker3 | 6 | 6 | 529 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 350

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 24 | 24 | 3932 | 411 | 0 | 8 | 0 | T30Farm |  |
| T30Claimant1 | 24 | 24 | 4355 | 427 | 12 | 4 | 0 | T30Farm |  |
| T30Claimant2 | 24 | 24 | 4383 | 448 | 3 | 3 | 0 | T30Farm |  |
| T30Merchant | 20 | 20 | 2441 | 119 | 3 | 0 | 0 | T30Road |  |
| T30Carrier | 24 | 24 | 4399 | 435 | 9 | 9 | 0 | T30Farm |  |
| T30Guard1 | 3 | 3 | 283 | 174 | 12 | 12 | 0 | T30Market |  |
| T30Guard2 | 24 | 24 | 4393 | 505 | 0 | 5 | 0 | T30Farm |  |
| T30Guard3 | 3 | 3 | 273 | 229 | 12 | 12 | 0 | T30Market |  |
| T30Bandit1 | 7 | 7 | 793 | 36 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 7 | 7 | 799 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 7 | 7 | 769 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 20 | 20 | 2535 | 124 | 0 | 1 | 0 | T30Road |  |
| T30Thief2 | 20 | 20 | 2604 | 114 | 0 | 1 | 0 | T30Road |  |
| T30Civ1 | 24 | 24 | 4291 | 505 | 2 | 4 | 0 | T30Farm |  |
| T30Civ2 | 20 | 20 | 2492 | 128 | 0 | 1 | 0 | T30Road |  |
| T30Civ3 | 24 | 24 | 4301 | 438 | 6 | 2 | 0 | T30Farm |  |
| T30Civ4 | 19 | 19 | 2490 | 123 | 0 | 3 | 0 | T30Road |  |
| T30Worker1 | 24 | 24 | 4288 | 467 | 7 | 2 | 0 | T30Farm |  |
| T30Worker2 | 19 | 19 | 2617 | 132 | 3 | 2 | 0 | T30Road |  |
| T30Worker3 | 5 | 5 | 557 | 0 | 0 | 0 | 0 | T30Orchard |  |

### Tick 400

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 26 | 26 | 3772 | 418 | 0 | 4 | 0 | T30Farm |  |
| T30Claimant1 | 26 | 26 | 4453 | 435 | 0 | 2 | 0 | T30Farm |  |
| T30Claimant2 | 26 | 26 | 4468 | 456 | 0 | 1 | 0 | T30Farm |  |
| T30Merchant | 29 | 29 | 2952 | 123 | 4 | 0 | 0 | T30Road |  |
| T30Carrier | 26 | 26 | 4459 | 443 | 9 | 9 | 0 | T30Farm |  |
| T30Guard1 | 30 | 30 | 1906 | 178 | 0 | 1 | 0 | T30Road |  |
| T30Guard2 | 26 | 26 | 4458 | 512 | 0 | 1 | 0 | T30Farm |  |
| T30Guard3 | 30 | 30 | 1818 | 232 | 0 | 1 | 0 | T30Road |  |
| T30Bandit1 | 11 | 11 | 1059 | 36 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit2 | 11 | 11 | 1051 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Bandit3 | 11 | 11 | 1020 | 40 | 4 | 4 | 1 | T30BanditCamp |  |
| T30Thief1 | 29 | 29 | 3106 | 128 | 0 | 1 | 0 | T30Road |  |
| T30Thief2 | 29 | 29 | 3196 | 117 | 0 | 1 | 0 | T30Road |  |
| T30Civ1 | 26 | 26 | 4404 | 515 | 0 | 1 | 0 | T30Farm |  |
| T30Civ2 | 29 | 29 | 3082 | 130 | 0 | 1 | 0 | T30Road |  |
| T30Civ3 | 26 | 26 | 4386 | 448 | 4 | 1 | 0 | T30Farm |  |
| T30Civ4 | 29 | 29 | 3065 | 127 | 0 | 3 | 0 | T30Road |  |
| T30Worker1 | 26 | 26 | 4433 | 475 | 6 | 0 | 0 | T30Farm |  |
| T30Worker2 | 29 | 29 | 3156 | 134 | 3 | 2 | 0 | T30Road |  |
| T30Worker3 | 16 | 16 | 540 | 0 | 0 | 0 | 0 | T30BanditCamp |  |

### Tick 450

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 29 | 29 | 3527 | 420 | 0 | 2 | 0 | T30Farm |  |
| T30Claimant1 | 29 | 29 | 4440 | 437 | 0 | 2 | 0 | T30Farm |  |
| T30Claimant2 | 29 | 29 | 4462 | 457 | 0 | 1 | 0 | T30Farm |  |
| T30Merchant | 32 | 32 | 3707 | 132 | 5 | 1 | 0 | T30Road |  |
| T30Carrier | 29 | 29 | 4394 | 445 | 9 | 9 | 0 | T30Farm |  |
| T30Guard1 | 32 | 32 | 3906 | 185 | 0 | 2 | 0 | T30Road |  |
| T30Guard2 | 30 | 30 | 4290 | 514 | 0 | 1 | 0 | ??? |  |
| T30Guard3 | 32 | 32 | 3949 | 241 | 0 | 2 | 0 | T30Road |  |
| T30Bandit1 | 12 | 12 | 1278 | 39 | 0 | 1 | 1 | T30BanditCamp |  |
| T30Bandit2 | 12 | 12 | 1269 | 42 | 0 | 1 | 1 | T30BanditCamp |  |
| T30Bandit3 | 12 | 12 | 1268 | 42 | 0 | 1 | 1 | T30BanditCamp |  |
| T30Thief1 | 32 | 32 | 3811 | 135 | 0 | 2 | 0 | T30Road |  |
| T30Thief2 | 32 | 32 | 3880 | 124 | 0 | 2 | 0 | T30Road |  |
| T30Civ1 | 30 | 30 | 3856 | 518 | 0 | 1 | 0 | ??? |  |
| T30Civ2 | 32 | 32 | 3813 | 136 | 0 | 1 | 0 | T30Road |  |
| T30Civ3 | 29 | 29 | 4277 | 450 | 4 | 1 | 0 | T30Farm |  |
| T30Civ4 | 31 | 31 | 3807 | 136 | 0 | 3 | 0 | T30Road |  |
| T30Worker1 | 29 | 29 | 4452 | 478 | 3 | 0 | 0 | T30Farm |  |
| T30Worker2 | 31 | 31 | 3843 | 139 | 3 | 2 | 0 | T30Road |  |
| T30Worker3 | 49 | 49 | 1391 | 8 | 8 | 1 | 0 | T30Road |  |

### Tick 499

| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |
|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|
| T30Ruler | 29 | 29 | 2852 | 425 | 0 | 5 | 0 | T30Farm |  |
| T30Claimant1 | 30 | 30 | 2719 | 440 | 0 | 2 | 0 | T30Farm |  |
| T30Claimant2 | 29 | 29 | 3895 | 464 | 0 | 1 | 0 | T30Farm |  |
| T30Merchant | 33 | 33 | 4221 | 147 | 7 | 2 | 0 | T30Road |  |
| T30Carrier | 29 | 29 | 3794 | 451 | 9 | 9 | 0 | T30Farm |  |
| T30Guard1 | 33 | 33 | 4401 | 195 | 0 | 3 | 0 | T30Road |  |
| T30Guard2 | 68 | 68 | 1886 | 516 | 0 | 1 | 0 | T30Orchard |  |
| T30Guard3 | 33 | 33 | 4489 | 256 | 0 | 2 | 0 | T30Road |  |
| T30Bandit1 | 10 | 10 | 1187 | 39 | 0 | 1 | 1 | T30BanditCamp |  |
| T30Bandit2 | 10 | 10 | 1164 | 42 | 0 | 1 | 1 | T30BanditCamp |  |
| T30Bandit3 | 10 | 10 | 1197 | 42 | 0 | 1 | 1 | T30BanditCamp |  |
| T30Thief1 | 33 | 33 | 4440 | 149 | 0 | 2 | 0 | T30Road |  |
| T30Thief2 | 33 | 33 | 4375 | 139 | 0 | 2 | 0 | T30Road |  |
| T30Civ1 | 30 | 30 | 2063 | 520 | 0 | 1 | 0 | T30Hub |  |
| T30Civ2 | 33 | 33 | 4258 | 146 | 0 | 1 | 0 | T30Road | YES |
| T30Civ3 | 29 | 29 | 3802 | 458 | 4 | 1 | 0 | T30Farm |  |
| T30Civ4 | 33 | 33 | 3960 | 141 | 0 | 1 | 0 | T30Road | YES |
| T30Worker1 | 29 | 29 | 3821 | 486 | 5 | 0 | 0 | T30Farm |  |
| T30Worker2 | 33 | 33 | 4376 | 151 | 0 | 1 | 0 | T30Road |  |
| T30Worker3 | 39 | 39 | 3653 | 21 | 9 | 1 | 0 | T30Road |  |

## Belief Store Composition (Final Tick)

### T30Ruler

| EntityKind | Count |
|------------|-------|
| Agent | 8 |
| ItemLot | 18 |
| Other | 2 |
| Place | 1 |

### T30Claimant1

| EntityKind | Count |
|------------|-------|
| Agent | 8 |
| ItemLot | 18 |
| Other | 2 |
| Place | 2 |

### T30Claimant2

| EntityKind | Count |
|------------|-------|
| Agent | 8 |
| ItemLot | 18 |
| Other | 2 |
| Place | 1 |

### T30Merchant

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Carrier

| EntityKind | Count |
|------------|-------|
| Agent | 8 |
| ItemLot | 18 |
| Other | 2 |
| Place | 1 |

### T30Guard1

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Guard2

| EntityKind | Count |
|------------|-------|
| Agent | 17 |
| ItemLot | 42 |
| Other | 3 |
| Place | 6 |

### T30Guard3

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Bandit1

| EntityKind | Count |
|------------|-------|
| Agent | 2 |
| ItemLot | 7 |
| Place | 1 |

### T30Bandit2

| EntityKind | Count |
|------------|-------|
| Agent | 2 |
| ItemLot | 7 |
| Place | 1 |

### T30Bandit3

| EntityKind | Count |
|------------|-------|
| Agent | 2 |
| ItemLot | 7 |
| Place | 1 |

### T30Thief1

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Thief2

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Civ1

| EntityKind | Count |
|------------|-------|
| Agent | 8 |
| ItemLot | 18 |
| Other | 2 |
| Place | 2 |

### T30Civ2

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Civ3

| EntityKind | Count |
|------------|-------|
| Agent | 8 |
| ItemLot | 18 |
| Other | 2 |
| Place | 1 |

### T30Civ4

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Worker1

| EntityKind | Count |
|------------|-------|
| Agent | 8 |
| ItemLot | 18 |
| Other | 2 |
| Place | 1 |

### T30Worker2

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 22 |
| Place | 1 |

### T30Worker3

| EntityKind | Count |
|------------|-------|
| Agent | 10 |
| ItemLot | 25 |
| Other | 1 |
| Place | 3 |

## Top 20 Slowest Ticks

| Tick | ms | KnownEntities | Claims |
|------|----|---------------|--------|
| 440 | 208.4 | 581 | 69405 |
| 400 | 176.7 | 491 | 60784 |
| 460 | 175.4 | 618 | 69740 |
| 420 | 167.1 | 513 | 65421 |
| 480 | 161.0 | 621 | 68421 |
| 320 | 146.3 | 345 | 52376 |
| 380 | 144.4 | 472 | 57702 |
| 340 | 141.7 | 342 | 53047 |
| 360 | 128.0 | 354 | 53581 |
| 450 | 127.3 | 573 | 69620 |
| 401 | 123.8 | 497 | 61048 |
| 300 | 120.2 | 376 | 52886 |
| 368 | 118.4 | 456 | 55341 |
| 441 | 107.9 | 581 | 69684 |
| 430 | 105.7 | 522 | 67412 |
| 490 | 105.2 | 620 | 67571 |
| 427 | 104.5 | 521 | 66971 |
| 175 | 104.0 | 305 | 43649 |
| 358 | 102.7 | 353 | 53394 |
| 334 | 101.1 | 342 | 52905 |

## Extrapolation to Full Soak (10080 ticks)

Linear regression: ms/tick = 31.56 + 0.1211 * tick
Estimated ms/tick at tick 10080: 1251.9
Estimated total time for 10080 ticks: 6468s (107.8 min)

