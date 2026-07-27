---
type: Task
title: Encapsulate visual styles entirely in birch-tui
description: Remove ThemeId from birch-core; use a DI seam so the render layer carries its theme selection without core knowing theme identities.
status: Draft
priority: low
tags:
- tech-debt
---

Sprint 015 (ADR 0021) put a `ThemeId` enum in `birch-core` so the theme is a `Settings` / protocol /
config value reachable by the existing plumbing. `birch-core` stays ratatui-free, but it now knows
*which themes exist* — a render-layer concern leaking into core, a mild break of the real-tree /
render-layer boundary. Accepted for now to keep the theme selectable without new machinery.

Future cleanup: hold **all** visual-style knowledge in `birch-tui`. Core should carry the selected
theme **opaquely** — a string token or a small dependency-injection seam (e.g. core exposes an
extensible settings slot the render layer populates and validates) — without enumerating theme
identities itself. The render layer owns the id↔`Theme` mapping and id validation. The user-facing
surface (`--theme`, `birch ctl set theme`, the config `theme` key) stays unchanged. Internal
structure only, no user-visible change. Low priority.
