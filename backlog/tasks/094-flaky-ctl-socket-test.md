---
type: Task
title: stale_socket_is_reclaimed_but_files_are_not is intermittently flaky
description: The ctl socket test fails roughly one run in five, and passes in isolation and on retry.
status: Draft
priority: low
---

`ctl::tests::stale_socket_is_reclaimed_but_files_are_not` failed three times in roughly fifteen full
`cargo test` runs during sprint 021, and passed every time it was run alone or immediately again. The
sprint changed nothing in `crates/birch/src/ctl.rs` — `git diff main..sprint/021 -- crates/birch/src/ctl.rs`
is empty — so it is pre-existing rather than introduced.

A flake in a *test* is a nuisance; a flake in **this** test is worth more than that, because the code
under it decides whether birch deletes a file at a socket path. The reclaim logic reads correctly:
a path that is not a socket returns `AlreadyExists` and is never removed, and the test asserts the
file's contents survive. So the likely fault is in the test's own sequencing rather than in `serve`
— but "likely" is not where this should be left.

## The failing assertion, captured

Caught during sprint-022's gate runs, on a run that passed on the immediate retry:

```
---- ctl::tests::stale_socket_is_reclaimed_but_files_are_not stdout ----
thread 'ctl::tests::stale_socket_is_reclaimed_but_files_are_not' panicked at crates/birch/src/ctl.rs:279:14:
dead socket is reclaimed: Custom { kind: AddrInUse, error: "…/birch-ctl-test-stale-55139.sock is in use by a live instance" }
```

So the reclaim path found the socket **live** where the test had just made it dead. That points at
the second hypothesis below rather than the first: `serve` decides liveness by connecting, and a
previous accept thread still holding the bound listener answers. Nothing about the file is wrong —
the process behind it has not let go yet.

## What a fix has to answer

- ~~**Which assertion fails.**~~ Answered above.
- **Whether the accept thread outlives `cleanup`.** `ServeHandle::cleanup` unlinks the socket
  synchronously but nothing joins the accept thread, which still holds the bound listener. If any
  ordering between that thread and the test's next `serve` matters, the test is racing a thread it
  cannot see.
- **Whether `serve` should be sequenced at all.** If a caller cannot rely on the socket being gone
  when `cleanup` returns, that is a product fact worth stating rather than a test to paper over.

## Related

- ADR 0010 — socket addressing and most-recent-wins, which this test guards.
