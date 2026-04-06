# Design: `scenario-designer` Skill

## Context

The project produces complex emergent behavior (302 golden tests, 131 scenario blocks), but the playable CLI scenarios are thin (3-5 places, 3-4 agents, partial profiles). This skill bridges the gap by designing rich, realistic RON scenarios from a user-provided theme, scanning the codebase for available types to ensure validity.

## Approach

Codebase-aware 3-phase design: scan for types, design scenario with user gate, write RON file. Handles both vague themes and specific requests.

## Output

RON file in `scenarios/<name>.ron` with comment header describing expected dynamics.

## Design Principles

1. Every agent has a reason to act
2. Natural tensions (scarcity, contested resources, info asymmetry)
3. Exercise 3+ AI systems per scenario
4. Realistic topology with meaningful travel times
5. One human-controlled agent
6. Profile diversity (P22)
7. Seed reproducibility
8. Don't over-provision — scarcity drives behavior
