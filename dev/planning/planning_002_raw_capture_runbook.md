002 - Raw Capture Runbook Planning

## Goal
Standardize how raw REST/WS responses are captured, saved, and reviewed before any parsing or modeling work.

## Scope
- Define a repeatable, safe workflow for raw captures (REST + WS)
- Clarify required environment variables and output locations
- Emphasize manual inspection before implementation

## Out of Scope
- Implementing parsers or business logic
- Changing existing documentation structure

## Risks
- Accidental trading calls without explicit consent
- Storing sensitive data without masking

## Mitigations
- Require BLUEFIN_ALLOW_TRADING=1 for create/cancel scripts
- Save raw outputs under data/raw (gitignored)
- Provide explicit masking guidance before sharing logs

## Deliverables
- Task list document with a concrete, step-by-step runbook
