# Agent-readiness — bluetti-controller
**Grade: A — Exemplary (91 / 100)**  ·  ▲ +86.75 since 2026-10-12T21:47:06Z (baseline F, 4.5)  ·  pre-initial-commit · 2026-10-12T21:55:59Z

> The repo is set up for long-running agentic work before any product code exists;
> the one real blocker is that `make check` cannot run on the dev host yet.

| Pillar | Level | Grade | Δ | Top gap |
|---|---|---|---|---|
| Rules & context | ●●●● (4) | A | ▲ | golden rules await owner confirmation |
| Filesystem-as-memory | ●●●● (4) | A | ▲ | — |
| Long-running loop | ●●●● (4) | A | ▲ | not yet exercised by a real iteration (accepted) |
| Verification gates | ●●●○ (3) | B | ▲ | rustfmt + clippy missing on the dev host; coverage not wired (M1) |
| Role-specialized subagents | ●●●● (4) | A | ▲ | — |
| Autonomy boundaries | ●●●○ (3) | B | ▲ | no `.claude/settings.json` permission scoping |
| Change hygiene | ●●●○ (3) | B | ▲ | no history yet to prove the convention (accepted) |

**Strengths:** project-specific golden rules and human-only list (hardware, keys,
control); computed loop position with `spec §` citations keeps the 50 KB ROADMAP
out of each iteration's read set; acceptance thresholds carry their arithmetic.
**Do first:** install `rustfmt` + `clippy` so `make check` runs (P4); owner confirms
RULES.md golden rules (P1).

This is an **adoption-maturity** scale for agentic work, not a code-quality score.
Weights: default profile. CI/CD is intentionally absent (owner's decision) and is
not counted as a gap.

_The scan was read-only: nothing in the repo was modified or run by the audit, and no
secrets were read. The audit wrote only `.agent-readiness/score.json` and this report;
the scaffold itself was installed separately in uplift mode._
