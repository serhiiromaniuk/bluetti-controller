# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers and change only when the gate verdict changes, a
> decision is made, a carry-forward moves, or a task opens/closes. `loop/where.sh`
> computes step position from `PLAN.md` and the last result from this file's newest
> `## ` heading — so restating an entry in a pointer file is prose every later
> iteration pays to re-read.
>
> **Budget: ≤40 lines per entry** (`make loop-hygiene` warns). Bullets, not
> narrative. Cite a spec/doc section, never re-quote it. Record the decision and
> the evidence, not the reasoning that produced them. The shape below is the shape.

## 2026-11-11 — Task opened

- Opened from the ledger: no in-progress row; next `todo` in `tasks/INDEX.md` order (before `r3-control-hardening`).
- BRIEF.md written 2026-11-07 at the ble-reliability close; unchanged.
- `loop/STATE.md` carry-forwards: none name this task.
- PLAN.md is a placeholder with no steps, so `where.sh` reports `needs_plan`.

**reviewer:** batched (ledger row + task docs only)
**Next:** planner writes PLAN.md, plan-reviewer audits it.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T13:22:09Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## 2026-11-11 15:30 — Deferred by the owner

- Owner, 2026-11-11, recorded at the owner's request: "skip elite, do
  hardening first + deploy". The R3 control hardening task
  (`2026-11-10_r3-control-hardening`) runs first, then the deploy.
- `tasks/INDEX.md`: this task back to `todo`, hardening to `in-progress`.
  No plan was written; the next open of this task starts at `needs_plan`.
