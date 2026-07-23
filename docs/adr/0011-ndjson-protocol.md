---
type: ADR
title: The wire protocol — versioned NDJSON request/response, additive-only
status: Accepted
sprint: sprint-004
---

# Context

The design doc commits to newline-delimited JSON with a `v` field, additive-only
evolution, clients tolerating unknown fields, and a closed verb set with no mutation
verbs. The concrete shapes need fixing, because the protocol is a public API from its
first release: host adapters may live out-of-tree on their own release cadence.

# Decision

One JSON object per line, UTF-8, both directions. Requests:

```json
{"v": 1, "verb": "reveal", "path": "/abs/or/root-relative"}
{"v": 1, "verb": "get-path", "form": "rel"}
{"v": 1, "verb": "set", "setting": "hidden", "value": "toggle"}
```

Responses always carry `ok`:

```json
{"v": 1, "ok": true, "data": "src/main.rs"}
{"v": 1, "ok": false, "error": "no instance selection"}
```

- `data` is a string or absent; future verbs may add sibling fields — never repurpose
  existing ones (additive-only). Unknown fields in either direction are ignored.
- Verbs: `reveal`, `get-path` (`form`: `name` | `rel` | `abs`, default `rel`), `get-root`,
  `set`, `set-root`, `open`, `quit` — the closed set, verbatim from the design doc. No
  mutation verbs, ever.
- `set` settings: `hidden`, `ignored`, `noise`, `icons`, `compact`, `git`, `files-first`;
  values `on`/`off`/`true`/`false`/`1`/`0`/`toggle`.
- An unparseable line or unknown verb gets `ok: false` with an error; the connection
  stays open (one bad request must not kill a host's control channel).
- A request `v` greater than the server's is answered best-effort (the fields it
  understands), per clients-tolerate-unknowns applied symmetrically.

# Consequences

- serde_json already in the tree; the protocol module is shared by the server (birch) and
  the client (birch-ctl) so shapes cannot drift.
- Verb semantics are testable end-to-end over a real socket without a terminal.
