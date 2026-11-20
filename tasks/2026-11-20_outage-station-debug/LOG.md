# Log

## Task opened 2026-11-20 — outage and missing station

- User requested both problems investigated and fixes deployed under repo rules.
- Plan review: 2 HIGH fixed before work: oracle-readable questions and explicit release/deployment checks.
- Read-only evidence: EB3A input dropped 19 Nov 20:49:48; AC180 simultaneously measured mains but does not vote. Elite last sampled 17 Nov 19:34:54; subsequent discovery fails.
- The false house event persists on restart and overlaps distinct AC180 input-loss intervals. No production changes or historical edits made.
- Human-only boundary and physical status limit live verification; questions requested in chat.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-20T15:30:41Z

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
