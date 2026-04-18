# Triage Checkpoint (Step 3)

After reading Sections 1, 2, and 3, evaluate whether any agent has any need above 750 permille for 100+ consecutive ticks (from Section 2 "Ticks above 750 permille" and Section 3 Smell 5/6 flags).

**If NO agent meets this threshold** (healthy scenario):

1. Run the lightweight Section 7 extraction — follow the Step 2 dump-reading protocol with these deltas:
   - Use `-A 5` in place of `-A 30` (step 3) and `-A 10` (step 4).
   - Step 5: grep only `Final affordances` with `-A 15` (skip `Affordances available` and `Affordances after travel`).
   - Skip step 6 (specific row reads).
2. Additionally: read Sections 5 and 6 in full. Optionally scan Section 4 last events — if Discovery events dominate (>50% of the last 100), note perception bloat from ground-item accumulation (Waste, consumed remnants), name the affected location(s), and cross-reference Section 6 place contents.
3. Run Layer 1 (smells unlikely to be severe) and Layer 3 (always runs). **Skip Layer 2 entirely**.
4. Use the Healthy Scenario Report Template.

**If ANY agent meets the threshold**:

1. Continue full Section 7 extraction (and Section 8 if present). Read Sections 5 and 6 in full.
2. Run all three layers.
3. Use the Standard Report Template.
