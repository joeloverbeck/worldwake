# Plan Mode Awareness

If plan mode is active:

- **Steps 1-6** proceed normally (all read-only).
- **Step 6** includes the initial findings report and any question-resolution rounds.
- **If questions exist**: Present Step 6 findings in conversation → use AskUserQuestion for each question → incorporate resolved answers into the plan file. The conversational report + question resolutions constitute the decision record; the plan file is the condensed implementation reference.
- **After all questions are resolved**: Write a condensed summary to the plan file, then call ExitPlanMode. If question resolution produces new findings, the plan file reflects the final resolved state, not the initial report.
- **After plan approval**: Steps 7-8 execute. The user's approval covers both question resolutions and overall changes — no separate gate.
- **Pre-Apply Verification** runs after ExitPlanMode approval, before Step 7.
- If there are no questions, proceed from the Step 6 findings report directly to writing the plan file and calling ExitPlanMode.
- If the ExitPlanMode result contains user comments, treat them as binding modifications.
- **Delegated resolution in plan mode**: When a question is resolved via delegation (user says "you decide" or "decide based on FOUNDATIONS"), include the resolution rationale in the plan file alongside the resulting change. The ExitPlanMode approval then covers both the resolution and the change.

**Plan file structure** (condensed action list for Step 7 execution — not a re-presentation of the conversational report):
- **Context**: Which spec, why it's being reassessed, classification type
- **Approved Changes**: Compact action items organized by Issues Fixed / Improvements Applied / Additions Incorporated. Each item: severity tag + one-line summary of what to change + affected deliverable number. For question-resolved changes, append the resolution rationale in parentheses (e.g., "per Q1: pragmatic approximation chosen because agents still must physically travel"). Omit the full evidence and reasoning for non-question items — the conversational report (Step 6) carries that.
- **Critical Files**: Paths of files to be modified
- **Verification**: How to confirm the updated spec is correct after writing

The conversational report (Step 6) is the decision artifact. Present it as a normal message — do not write it to the plan file. The plan file is a separate condensed reference for implementation (Steps 7-8).
