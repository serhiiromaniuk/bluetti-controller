#!/usr/bin/env python3
"""Measure the v0.4 acceptance gate (RULES.md) from a collector database.

Usage: python3 -I scripts/acceptance_v04.py <db> <from_ts> <to_ts> <interval_s>
                                            [--min-pct 95] [--max-mb 10]

Reads the SQLite file read-only (stdlib only) and never touches a station, so it
is safe to run while `serve` is writing. Per station: scheduled polls in
[from_ts, to_ts), samples, misses, success % (PASS at >= min-pct). DB size:
(db + WAL bytes) / stations / days (PASS below max-mb), with bytes per table so
an overage names its table. Lists the grid_event rows overlapping the window.
Exits 1 on any FAIL.
"""

import argparse
import os
import sqlite3
import sys


def table_bytes(con):
    """Bytes per table: dbstat when compiled in, else rows x average row length."""
    try:
        rows = con.execute(
            "SELECT name, sum(pgsize) FROM dbstat GROUP BY name ORDER BY 2 DESC"
        ).fetchall()
        return "dbstat", rows
    except sqlite3.OperationalError:
        pass
    out = []
    tables = [r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type = 'table'")]
    for t in tables:
        cols = [r[1] for r in con.execute(f'PRAGMA table_info("{t}")')]
        width = " + ".join(f'coalesce(length("{c}"), 0)' for c in cols)
        n, avg = con.execute(f'SELECT count(*), avg({width}) FROM "{t}"').fetchone()
        out.append((t, int(n * (avg or 0))))
    return "estimate (rows x avg row length)", sorted(out, key=lambda r: -r[1])


def main(argv):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("db")
    p.add_argument("from_ts", type=int)
    p.add_argument("to_ts", type=int)
    p.add_argument("interval_s", type=int)
    p.add_argument("--min-pct", type=float, default=95.0)
    p.add_argument("--max-mb", type=float, default=10.0)
    a = p.parse_args(argv)
    if a.to_ts <= a.from_ts or a.interval_s <= 0:
        p.error("need to_ts > from_ts and interval_s > 0")
    if not os.path.isfile(a.db):
        p.error(f"no such file: {a.db}")

    con = sqlite3.connect(f"file:{a.db}?mode=ro", uri=True)
    fail = False
    window = (a.from_ts, a.to_ts)
    scheduled = (a.to_ts - a.from_ts) // a.interval_s
    print(f"window [{a.from_ts}, {a.to_ts}) = {a.to_ts - a.from_ts} s, interval {a.interval_s} s, "
          f"scheduled {scheduled} polls per station")

    print("\n== polls per station ==")
    stations = con.execute("SELECT id, name, model FROM station ORDER BY id").fetchall()
    for sid, name, model in stations:
        (samples,) = con.execute(
            "SELECT count(*) FROM sample WHERE station_id = ? AND ts >= ? AND ts < ?", (sid, *window)
        ).fetchone()
        failures = con.execute(
            "SELECT stage, count(*) FROM poll_failure WHERE station_id = ? AND ts >= ? AND ts < ?"
            " GROUP BY stage ORDER BY stage", (sid, *window)
        ).fetchall()
        # A window not aligned to the tick grid can hold one extra sample; cap at 100 %.
        ok = min(samples, scheduled)
        pct = 100.0 * ok / scheduled if scheduled else 0.0
        verdict = "PASS" if scheduled and pct >= a.min_pct else "FAIL"
        fail |= verdict == "FAIL"
        stages = ", ".join(f"{s} {n}" for s, n in failures) or "none"
        print(f"{name} ({model}): samples {samples}, misses {scheduled - ok}, "
              f"success {pct:.2f}% (min {a.min_pct}%) {verdict} · failures: {stages}")

    print("\n== database size ==")
    db_b, wal_b = (os.path.getsize(f) if os.path.exists(f) else 0 for f in (a.db, a.db + "-wal"))
    size = db_b + wal_b
    days = (a.to_ts - a.from_ts) / 86400
    per = size / 1e6 / max(len(stations), 1) / days
    verdict = "PASS" if per < a.max_mb else "FAIL"
    fail |= verdict == "FAIL"
    print(f"db {db_b} B + wal {wal_b} B = {size} B, {len(stations)} stations, {days:.4f} days: "
          f"{per:.3f} MB per station-day (max {a.max_mb}) {verdict}")
    method, rows = table_bytes(con)
    # Logical pages only: a WAL-driven overage shows in the line above, not here.
    print(f"bytes per table ({method}):")
    for name, b in rows:
        print(f"  {name}: {b}")

    print("\n== grid_event rows overlapping the window ==")
    events = con.execute(
        "SELECT id, scope, kind, started_at, ended_at, precision_s FROM grid_event"
        " WHERE started_at < ? AND (ended_at IS NULL OR ended_at >= ?) ORDER BY started_at",
        (a.to_ts, a.from_ts),
    ).fetchall()
    for e in events:
        print(f"  id {e[0]} {e[1]} {e[2]} started {e[3]} ended {e[4]} precision {e[5]} s")
    if not events:
        print("  none")

    print(f"\nRESULT: {'FAIL' if fail else 'PASS'}")
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
