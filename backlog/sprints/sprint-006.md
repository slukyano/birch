---
type: Sprint
title: Search feel — match what is displayed
status: Done
branch: sprint/006
tasks:
- 014-refine-search-matching
---

# Scope rationale

Maintainer feedback after using MVP search: path matching surfaced hits the tree could
not explain, and matched characters were invisible. One task: name-first matching (with
the `/` escape hatch), identical in both modes, plus per-character highlighting. Scope
and design approved by the maintainer in discussion.

# Checklist

- [x] refine-search-matching

# Open questions

None.

# Sprint summary

- **refine-search-matching** (major for search feel): the corpus is now the simple name
  (a `/` in the query switches to full relative paths), identical in tree and picker;
  matched characters highlight in amber — in names, in chain labels at each hit member's
  segment, and in the picker's relative paths (name indices shifted by the dir prefix);
  path-mode tree hits fall back to whole-name bold. Camel humps anchor via smart-case.
  ADR 0013 supersedes ADR 0009's corpus sentence; the design doc's search section now
  states the policy.
- **Independent review**: one substantial catch — nucleo's match positions are not char
  indices for non-ASCII names (NFD accents, the macOS normal form, match over raw
  bytes; other Unicode over grapheme clusters). `to_char_indices()` maps all three
  segmentations onto char positions, pinned by NFD and Cyrillic tests. Also fixed:
  mid-chain hits now light their own segment; the stale spec sentence; missing tests
  for chain offsets, the picker shift, and span run-grouping.
- PTY-verified: `sm` no longer cross-matches `src/main.rs`, `s/m` does (path mode),
  matched characters render in the accent color, and the picker picks names-first.

# Session log

- Sprint created from maintainer feedback; design approved with scope.
- Implemented, PTY-verified, review fixes (Unicode index mapping), closed.
