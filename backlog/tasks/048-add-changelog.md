---
type: Task
title: Add a changelog and issue templates
description: A CHANGELOG.md and optional GitHub issue/PR templates.
status: Designed
priority: low
---

Add a `CHANGELOG.md` (Keep a Changelog style) tracking user-visible changes per release,
and optionally GitHub issue templates under `.github/`. Low priority; most valuable once
releases exist (`automate-releases`).

## Design

Add `CHANGELOG.md` (Keep a Changelog format) with a `[0.1.0]` entry for the initial release.
cargo-dist reads the changelog for GitHub Release notes, so this also improves release output going
forward. Optional: `.github/ISSUE_TEMPLATE/` (bug / feature). Low ceremony.

**Changelog-maintenance rules live in `CONTRIBUTING.md`** (the tech-docs approach — coordinates
with `049`): what a changelog entry is, when to add one (user-visible changes), and the Keep a
Changelog sections (`Added` / `Changed` / `Fixed` / …). `CHANGELOG.md` itself just holds the
entries.
