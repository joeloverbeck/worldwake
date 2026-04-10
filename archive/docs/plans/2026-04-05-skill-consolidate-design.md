# Design: `skill-consolidate` Skill

## Context

Skills accumulate content through iterative skill-audit cycles. Each addition addresses a real gap, but structurally degrades: redundancies, fragmented topics, wall-of-text sections, scattered decision paths. This skill is the structural counterpart to skill-audit — where skill-audit grows, skill-consolidate prunes.

## Approach

Single-pass structural consolidation. One argument (skill path). Reads, analyzes, rewrites in-place, presents diff summary. No user gate — reviews via `git diff`.

## Output

Rewritten SKILL.md in-place + structured diff summary in conversation.

## Design Principles

1. Semantic preservation — every unique instruction survives
2. Four optimization axes: redundancy, regrouping, readability, decision paths
3. Tighten wording without changing meaning
4. No scope expansion — consolidation only, not improvement
5. Frontmatter untouched
