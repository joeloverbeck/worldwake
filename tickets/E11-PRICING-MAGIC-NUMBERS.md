# E11 Pricing Formula Violates Revised Principle 2

**Epic**: E11 (Trade & Economy)
**Priority**: Medium (blocks no current work; E11 is not yet implemented)
**Type**: Design revision

## Problem

The E11 spec (`specs/E11-trade-economy.md`) contains a pricing formula with hardcoded threshold-multiplier pairs:

```
Abundant (>150% typical stock): price * 0.7
Normal (50-150%): base price
Scarce (<50%): price * 1.5
Critical (<10%): price * 3.0
```

This violates revised Principle 2 (No Magic Numbers), specifically the new test:

> If a system computes an aggregate and applies threshold-based multipliers to produce an outcome, verify that both the aggregate and the thresholds derive from concrete world properties. A pricing formula that says "if stock < 50%, multiply price by 1.5" is a magic number unless the 50% and 1.5 arise from traceable world state. Shortcutting with lookup tables what should emerge from agent interactions violates this principle.

## Required Change

Replace the lookup-table pricing with agent-to-agent negotiation where prices emerge from:
- Individual agent need urgency (how badly does the buyer want it)
- Seller's assessment of remaining stock relative to expected future demand
- Competing offers from other buyers/sellers present at the same location
- Each agent's personal risk tolerance and utility weights (Principle 11)

The "price" is the outcome of a negotiation action between two co-located agents, not a formula applied to aggregate stock levels.

## When to Address

Before implementing E11. Update the spec's pricing section during E11 ticket creation.
