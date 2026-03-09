# Foundations Revision Design Document

**Date**: 2026-03-09
**Status**: Approved

## Motivation

The original 9 foundational principles define strong simulation integrity — no cheating, no scripts, traceable causality. However, analysis against emergence literature (Holland 1998, Epstein/Axtell 1996, Wolfram 2002, Mitchell 2009, Kauffman 1995) and practical simulation design (Dwarf Fortress, RimWorld, Caves of Qud) revealed that these principles are *necessary* but not *sufficient* conditions for rich emergent behavior.

The principles ensure the simulation is honest. They do not ensure it stays dynamic, that agents are meaningfully different, that information respects physical distance, or that systems compose without coupling.

## Gap Analysis

### HIGH PRIORITY

**GAP 1: Perturbation / Sustained Dynamism**
A causally perfect simulation can reach equilibrium and go inert. Kauffman, Prigogine, and Holland agree: emergent systems must be "far from equilibrium." The world needs ongoing sources of change.

**GAP 2: Dissipation / Entropy / Decay**
The complement to P7 (every action has cost). INACTION must also have cost. Food spoils, structures degrade, knowledge goes stale. Without decay, state accumulates until the system seizes up.

*Resolution*: These two gaps are addressed together by **Principle 8: Every Amplifying Loop Must Have a Physical Dampener**. Rather than adding abstract "perturbation" or "decay" principles, we mandate that every positive feedback loop has a concrete dampener — which naturally produces the dynamism and decay the literature calls for.

**GAP 3: Agent Heterogeneity**
Holland, Epstein, Kauffman, and Mitchell all identify diversity of goals/beliefs/capabilities as essential. P8 (Agent Symmetry) ensures identical rules but says nothing about agents being meaningfully DIFFERENT. Identical agents satisfy all 9 principles yet produce minimal emergence.

*Resolution*: **Principle 11: Agent Diversity Through Concrete Variation**.

### MEDIUM-HIGH PRIORITY

**GAP 4: Locality of Interaction and Information**
P9 mandates belief-only planning but no principle governs HOW information propagates. Simon's bounded rationality and Epstein's limited-vision agents show that information locality is a structural requirement for emergence, not just an implementation detail.

*Resolution*: **Principle 7: Locality of Interaction and Information**.

### MEDIUM PRIORITY

**GAP 5: System Interaction Topology**
Sylvester's insight from RimWorld: a system's value is proportional to its interaction surface with other systems. No principle governs how systems relate. Direct inter-system coupling limits emergence to only explicitly coded interactions.

*Resolution*: **Principle 12: Systems Interact Through State, Not Through Each Other**.

### Also addressed

**Principle 2 (No Magic Numbers) — Test Strengthened**
The existing test catches raw probability constants but not aggregate-threshold shortcuts that look data-derived but are actually magic numbers in disguise (e.g., `if stock < 50%, multiply price by 1.5`).

## Restructuring

The 9 original principles were a flat list. The 13 revised principles are organized into 4 categories that reflect their domain:

```
I. CAUSAL FOUNDATIONS (what makes the simulation honest)
   1. Maximal Emergence Through Causality
   2. No Magic Numbers [test strengthened]
   3. Concrete State Over Abstract Scores

II. WORLD DYNAMICS (how the world behaves)
   4. Simulate Carriers of Consequence
   5. World Runs Without Observers
   6. Every Action Has Physical Cost
   7. Locality of Interaction and Information [NEW]
   8. Every Amplifying Loop Has a Physical Dampener [NEW]

III. AGENT ARCHITECTURE (how agents work)
   9.  Agent Symmetry
   10. Intelligent Agency Over Behavioral Scripts
   11. Agent Diversity Through Concrete Variation [NEW]

IV. SYSTEM ARCHITECTURE (how code is organized)
   12. Systems Interact Through State, Not Each Other [NEW]
   13. No Backward Compatibility
```

## Renumbering Map

| Old # | Old Name | New # | Change |
|-------|----------|-------|--------|
| 1 | Maximal Emergence Through Causality | 1 | Unchanged |
| 2 | No Magic Numbers | 2 | Test strengthened |
| 6 | Concrete State Over Abstract Scores | 3 | Renumbered |
| 4 | Simulate Carriers of Consequence | 4 | Renumbered |
| 5 | World Runs Without Observers | 5 | Unchanged |
| 7 | Every Action Has Physical Cost | 6 | Renumbered |
| — | (new) | 7 | Locality of Interaction and Information |
| — | (new) | 8 | Every Amplifying Loop Has a Physical Dampener |
| 8 | Agent Symmetry | 9 | Renumbered |
| 9 | Intelligent Agency Over Behavioral Scripts | 10 | Renumbered |
| — | (new) | 11 | Agent Diversity Through Concrete Variation |
| — | (new) | 12 | Systems Interact Through State, Not Each Other |
| 3 | No Backward Compatibility | 13 | Renumbered |

## Literature References

- Holland, J.H. (1998). *Emergence: From Chaos to Order*
- Epstein, J.M. & Axtell, R. (1996). *Growing Artificial Societies*
- Wolfram, S. (2002). *A New Kind of Science*
- Mitchell, M. (2009). *Complexity: A Guided Tour*
- Kauffman, S.A. (1995). *At Home in the Universe*
- Simon, H.A. (1996). *The Sciences of the Artificial*
- Bratman, M.E. (1987). *Intention, Plans, and Practical Reason*
- Meadows, D.H. (2008). *Thinking in Systems*
- Sylvester, T. (2013). *Designing Games*
