# Ratatui Design Intelligence 2.1.0

## Purpose

This module converts a reference project into evidence-backed Ratatui and Herdr
integration intelligence. It owns research, classification, stack translation,
and a validated handoff; it never authorizes Herdr product code changes.

## Inputs

- An immutable reference-project revision and license evidence.
- A fresh Herdr architecture baseline, obtained through Codebase Memory when a
  phase requires graph evidence.
- Explicit feature, page, layout, design, behavior, and failure goals.

## Versions

The module and canonical pipeline are version `2.1.0`. Its compatibility family
remains `reference-project-intelligence-v2`, so the v2 phase IDs, statuses, and
artifact names remain additive and stable.

## P0-P14

P0-P7 freeze, index, map, verify, classify, publish, and audit source
intelligence. P8-P14 compare current Herdr, classify behavior and stack gaps,
trace data authority, specify terminal-cell fidelity, bind components, derive
TDD slices, and audit the handoff. A P7 result is source intelligence only.

## Outputs

The canonical artifacts remain required. Version 2.1.0 additionally requires
`stack-adaptation-map.json`, whose records distinguish direct API use,
structural adaptation, behavior reimplementation, and rejection.

## Commands

Run all module tests and both validators from the repository root:

```bash
PYTHONDONTWRITEBYTECODE=1 python -m unittest discover \
  -s .codex/skills/ratatui-design-intelligence/tests -p 'test_*.py'
python .codex/skills/ratatui-design-intelligence/scripts/validate_reference_pipeline.py \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-pipeline-v2.json \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-run-template.json
python .codex/skills/ratatui-design-intelligence/scripts/validate_v21_module.py \
  .codex/skills/ratatui-design-intelligence
```

## Statuses

`queued`, `running`, `passed`, `partial`, `blocked`, `failed`, and `skipped`
remain canonical pipeline statuses. Stack records also allow `rejected`. A run
cannot be promoted from partial or blocked without new evidence and fresh
validation.

## MCP Blocking

Codebase Memory evidence is mandatory for graph-dependent work. If its MCP
tools are unavailable, mark the affected phase blocked; never invent symbols,
edges, snippets, architecture, or index results. Filesystem evidence may only
cover a phase whose contract explicitly permits it.

## Product Isolation

This module cannot edit product code or operate the stable runtime. TDD applies
to module contracts, schemas, validators, and evals here. Any future Herdr
implementation belongs to the sibling `herdr-change-pipeline` after explicit
approval and must preserve the runtime/client and pure-render boundaries.

## Handoff

The P14 adapter is a reviewed input to `herdr-change-pipeline`, not execution
authority. Use targeted staging and atomic commits. CyPack publication, shared
checkout fast-forward, push, and graph refresh occur only at their approved
delivery gate.
