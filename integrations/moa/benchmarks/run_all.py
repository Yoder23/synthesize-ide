#!/usr/bin/env python3
"""
run_all.py — Master Benchmark Runner
======================================
Runs all MoA benchmarks and produces:
  1. Console report
  2. benchmark_results.json  (machine-readable, for CI)
  3. BENCHMARK_RESULTS.md    (auto-generated, copy into BENCHMARKS.md)

Usage:
    python benchmarks/run_all.py
    python benchmarks/run_all.py --quick    # shorter N for fast iteration
"""

from __future__ import annotations

import sys
import json
import time
import argparse
import platform
import subprocess
from pathlib import Path
from datetime import datetime

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

# Import benchmark modules
from benchmarks.bench_safety   import run as run_safety, run_jailbreak_battery
from benchmarks.bench_memory   import run as run_memory
from benchmarks.bench_agent    import run as run_agent
from benchmarks.bench_jailbreak import run_jailbreak_suite
from benchmarks.bench_openclaw_comparison import run_comparison as run_claw_comparison


def get_platform_info() -> dict:
    info = {
        "python": platform.python_version(),
        "os": platform.system(),
        "machine": platform.machine(),
        "timestamp": datetime.utcnow().isoformat() + "Z",
    }
    try:
        result = subprocess.run(
            [sys.executable, "-c",
             "import torch; print(torch.__version__, torch.cuda.is_available())"],
            capture_output=True, text=True, timeout=5
        )
        if result.returncode == 0:
            parts = result.stdout.strip().split()
            info["torch"] = parts[0]
            info["cuda"] = parts[1] == "True"
    except Exception:
        info["torch"] = "not installed"
        info["cuda"] = False
    return info


def fmt_rate(n: float, unit: str = "/s") -> str:
    if n >= 1_000_000:
        return f"{n/1_000_000:.1f}M{unit}"
    if n >= 1_000:
        return f"{n/1_000:.1f}K{unit}"
    return f"{n:.0f}{unit}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--quick", action="store_true",
                        help="Run with reduced N (faster, less precise)")
    parser.add_argument("--out-dir", default=".",
                        help="Directory for output files")
    args = parser.parse_args()

    out_dir = Path(args.out_dir)

    print()
    print("╔══════════════════════════════════════════════════╗")
    print("║          MoA Benchmark Suite                     ║")
    print("╚══════════════════════════════════════════════════╝")

    platform_info = get_platform_info()
    print(f"  Python {platform_info['python']} | {platform_info['os']} {platform_info['machine']}")
    print(f"  torch: {platform_info.get('torch','N/A')} | "
          f"cuda: {platform_info.get('cuda', False)}")
    print(f"  {platform_info['timestamp']}")
    print()

    results = {"platform": platform_info, "benchmarks": {}}

    # ─── Safety Gate ─────────────────────────────────────────────────────────
    print("── Safety Gate ─────────────────────────────────────")
    t0 = time.perf_counter()
    s = run_safety(n_throughput=5_000 if args.quick else 10_000)
    elapsed = time.perf_counter() - t0
    print(f"  Throughput (approved):   {fmt_rate(s.throughput_approved)}")
    print(f"  Throughput (rejected):   {fmt_rate(s.throughput_rejected)}")
    print(f"  Latency  p50/p95/p99:    {s.latency_p50_us:.1f} / {s.latency_p95_us:.1f} / {s.latency_p99_us:.1f} µs")
    print(f"  Jailbreak battery:       {s.jailbreak_blocked}/{s.jailbreak_total}  "
          f"({s.jailbreak_block_rate*100:.0f}%)")
    print(f"  False positive rate:     {s.fp_incorrectly_blocked}/{s.fp_total}  "
          f"({s.fp_rate*100:.0f}%)")
    print(f"  Boundary tests:          {s.boundary_passed}/{s.boundary_tests}")
    print(f"  [{elapsed:.1f}s]")
    results["benchmarks"]["safety"] = {
        "throughput_approved_per_s": round(s.throughput_approved),
        "throughput_rejected_per_s": round(s.throughput_rejected),
        "latency_p50_us":  round(s.latency_p50_us, 2),
        "latency_p95_us":  round(s.latency_p95_us, 2),
        "latency_p99_us":  round(s.latency_p99_us, 2),
        "latency_max_us":  round(s.latency_max_us, 2),
        "jailbreak_total": s.jailbreak_total,
        "jailbreak_blocked": s.jailbreak_blocked,
        "jailbreak_block_rate": round(s.jailbreak_block_rate, 4),
        "fp_total": s.fp_total,
        "fp_blocked": s.fp_incorrectly_blocked,
        "fp_rate": round(s.fp_rate, 4),
        "boundary_tests": s.boundary_tests,
        "boundary_passed": s.boundary_passed,
    }

    # ─── Jailbreak Suite ──────────────────────────────────────────────────────
    print()
    print("── Adversarial Resistance ───────────────────────────")
    t0 = time.perf_counter()
    j = run_jailbreak_suite()
    elapsed = time.perf_counter() - t0
    by_cat = j.by_category()
    for cat, (total, blocked) in sorted(by_cat.items()):
        status = "✓" if blocked == total else f"✗ {total-blocked} BYPASSED"
        print(f"  {cat:<30} {blocked:>3}/{total:<3}  {status}")
    print(f"  Total: {j.blocked}/{j.total}  ({j.block_rate*100:.0f}% resistance)  [{elapsed:.1f}s]")
    results["benchmarks"]["jailbreak"] = {
        "total": j.total,
        "blocked": j.blocked,
        "block_rate": round(j.block_rate, 4),
        "bypassed": j.bypassed,
        "by_category": {k: {"total": v[0], "blocked": v[1]} for k, v in by_cat.items()},
    }

    # ─── Memory ───────────────────────────────────────────────────────────────
    print()
    print("── Memory Layer ─────────────────────────────────────")
    t0 = time.perf_counter()
    m = run_memory()
    elapsed = time.perf_counter() - t0
    print(f"  EventLog write:          {fmt_rate(m.event_log_write_tps)}")
    print(f"  EpisodicBuffer write:    {fmt_rate(m.episode_write_tps)}")
    print(f"  EpisodicBuffer recall:   {fmt_rate(m.episode_kw_recall_tps)}")
    print(f"  FactGraph add:           {fmt_rate(m.fact_add_tps)}")
    print(f"  AgentMemory recall:      {m.recall_latency_ms:.3f} ms  ({m.recall_with_n} episodes)")
    print(f"  Footprint (100ep):       {m.footprint_100ep_kb:.1f} KB")
    print(f"  Footprint (1000ep):      {m.footprint_1000ep_kb:.1f} KB")
    print(f"  [{elapsed:.1f}s]")
    results["benchmarks"]["memory"] = {
        "event_log_write_tps": round(m.event_log_write_tps),
        "episode_write_tps": round(m.episode_write_tps),
        "episode_recall_tps": round(m.episode_kw_recall_tps),
        "fact_add_tps": round(m.fact_add_tps),
        "recall_latency_ms": round(m.recall_latency_ms, 3),
        "footprint_100ep_kb": round(m.footprint_100ep_kb, 1),
        "footprint_1000ep_kb": round(m.footprint_1000ep_kb, 1),
    }

    # ─── Agent OODA ───────────────────────────────────────────────────────────
    print()
    print("── OODA Agent Loop ──────────────────────────────────")
    t0 = time.perf_counter()
    a = run_agent()
    elapsed = time.perf_counter() - t0
    print(f"  Turns/s (MockBackend):   {fmt_rate(a.turns_per_sec)}")
    print(f"  Turn latency p50/p95/p99:{a.latency_p50_ms:.2f} / {a.latency_p95_ms:.2f} / {a.latency_p99_ms:.2f} ms")
    print(f"  Memory scaling:")
    print(f"    10 eps:   {a.latency_ms_10ep:.3f} ms/turn")
    print(f"   100 eps:   {a.latency_ms_100ep:.3f} ms/turn")
    print(f"   500 eps:   {a.latency_ms_500ep:.3f} ms/turn")
    print(f"  Gate overhead:           {a.overhead_us:.2f} µs/eval")
    print(f"  50-turn p50/p99:         {a.multiturn_p50_ms:.3f} / {a.multiturn_p99_ms:.3f} ms")
    print(f"  [{elapsed:.1f}s]")
    results["benchmarks"]["agent"] = {
        "turns_per_sec": round(a.turns_per_sec),
        "latency_p50_ms": round(a.latency_p50_ms, 3),
        "latency_p95_ms": round(a.latency_p95_ms, 3),
        "latency_p99_ms": round(a.latency_p99_ms, 3),
        "latency_ms_10ep": round(a.latency_ms_10ep, 3),
        "latency_ms_100ep": round(a.latency_ms_100ep, 3),
        "latency_ms_500ep": round(a.latency_ms_500ep, 3),
        "gate_overhead_us": round(a.overhead_us, 2),
        "multiturn_p50_ms": round(a.multiturn_p50_ms, 3),
        "multiturn_p99_ms": round(a.multiturn_p99_ms, 3),
    }

    # ─── Claw Family Comparison ───────────────────────────────────────────────
    print()
    print("── Claw Family Architecture Comparison ─────────────")
    t0 = time.perf_counter()
    claw = run_claw_comparison()
    elapsed = time.perf_counter() - t0
    tp = claw["throughput"]
    h2h = claw["head_to_head"]
    total = h2h["total"]
    print(f"  MoA gate (approved):     {fmt_rate(tp['approved_tps'])}")
    print(f"  MoA gate (rejected):     {fmt_rate(tp['rejected_tps'])}")
    print(f"  MoA block rate:          {h2h['moa_blocked']}/{total} vectors")
    print(f"  OpenClaw block rate:     {total - h2h['oc_main_approved']}/{total} (no action-type gate by design)")
    print(f"  ZeroClaw supervised:     {h2h['zc_supervised_blocked']}/{total} (TOML threshold)")
    print(f"  ZeroClaw YOLO:           {h2h['zc_yolo_blocked']}/{total} (all gates bypassed)")
    print(f"  [{elapsed:.1f}s]")
    results["benchmarks"]["claw_comparison"] = claw

    # ─── Pass/Fail Summary ────────────────────────────────────────────────────
    print()
    print("── Summary ──────────────────────────────────────────")
    critical_pass = (
        s.jailbreak_block_rate == 1.0 and
        s.fp_rate == 0.0 and
        j.block_rate == 1.0 and
        s.boundary_passed == s.boundary_tests
    )
    if critical_pass:
        print("  ✓ 100% jailbreak resistance (safety is code, not prompt)")
        print("  ✓ 0% false positive rate on legitimate actions")
        print("  ✓ All boundary conditions correct")
        print("  ✓ All adversarial categories blocked")
        results["critical_pass"] = True
    else:
        print("  *** CRITICAL FAILURES DETECTED ***")
        results["critical_pass"] = False

    # ─── Write outputs ────────────────────────────────────────────────────────
    json_path = out_dir / "benchmark_results.json"
    with open(json_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\n  Results written to: {json_path}")

    # Generate markdown
    md = _generate_markdown(results)
    md_path = out_dir / "BENCHMARK_RESULTS.md"
    with open(md_path, "w", encoding="utf-8") as f:
        f.write(md)
    print(f"  Markdown written to: {md_path}")
    print()


def _generate_markdown(r: dict) -> str:
    s = r["benchmarks"]["safety"]
    j = r["benchmarks"]["jailbreak"]
    m = r["benchmarks"]["memory"]
    a = r["benchmarks"]["agent"]
    p = r["platform"]

    lines = [
        "<!-- AUTO-GENERATED by benchmarks/run_all.py — do not edit manually -->",
        f"<!-- Generated: {p['timestamp']} | Python {p['python']} | {p['os']} {p['machine']} -->",
        "",
        "## Benchmark Results",
        "",
        f"> Python {p['python']} | {p['os']} {p['machine']} | "
        f"torch: {p.get('torch','N/A')} | CUDA: {p.get('cuda',False)}",
        "",
        "### Safety Gate",
        "",
        "| Metric | Value |",
        "|---|---|",
        f"| Throughput (approved path) | {s['throughput_approved_per_s']:,} decisions/s |",
        f"| Throughput (rejected path) | {s['throughput_rejected_per_s']:,} decisions/s |",
        f"| Latency p50 | {s['latency_p50_us']} µs |",
        f"| Latency p95 | {s['latency_p95_us']} µs |",
        f"| Latency p99 | {s['latency_p99_us']} µs |",
        f"| Jailbreak battery | **{s['jailbreak_blocked']}/{s['jailbreak_total']} blocked** ({s['jailbreak_block_rate']*100:.0f}%) |",
        f"| False positive rate | **{s['fp_blocked']}/{s['fp_total']}** ({s['fp_rate']*100:.0f}%) |",
        f"| Boundary tests | {s['boundary_passed']}/{s['boundary_tests']} |",
        "",
        "### Adversarial Resistance (100 attacks)",
        "",
        "| Attack Category | Blocked | Result |",
        "|---|---|---|",
    ]
    for cat, d in sorted(j["by_category"].items()):
        status = "✅ 100%" if d["blocked"] == d["total"] else f"❌ {d['blocked']}/{d['total']}"
        lines.append(f"| {cat} | {d['blocked']}/{d['total']} | {status} |")
    lines += [
        f"| **Total** | **{j['blocked']}/{j['total']}** | "
        f"{'✅ 100% resistance' if j['block_rate']==1.0 else '❌ BYPASSED'} |",
        "",
        "### Memory Layer",
        "",
        "| Operation | Throughput |",
        "|---|---|",
        f"| EventLog write | {m['event_log_write_tps']:,} entries/s |",
        f"| EpisodicBuffer write | {m['episode_write_tps']:,} episodes/s |",
        f"| EpisodicBuffer recall | {m['episode_recall_tps']:,} queries/s |",
        f"| FactGraph add | {m['fact_add_tps']:,} facts/s |",
        f"| AgentMemory recall (500 eps) | {m['recall_latency_ms']} ms/query |",
        f"| Memory footprint (100 ep) | {m['footprint_100ep_kb']} KB |",
        f"| Memory footprint (1000 ep) | {m['footprint_1000ep_kb']} KB |",
        "",
        "### OODA Loop (MockBackend)",
        "",
        "| Metric | Value |",
        "|---|---|",
        f"| Throughput | {a['turns_per_sec']:,} turns/s |",
        f"| Latency p50 | {a['latency_p50_ms']} ms |",
        f"| Latency p99 | {a['latency_p99_ms']} ms |",
        f"| Latency (10 episodes stored) | {a['latency_ms_10ep']} ms |",
        f"| Latency (100 episodes stored) | {a['latency_ms_100ep']} ms |",
        f"| Latency (500 episodes stored) | {a['latency_ms_500ep']} ms |",
        f"| Safety gate overhead | {a['gate_overhead_us']} µs/eval |",
        f"| 50-turn p99 (no drift) | {a['multiturn_p99_ms']} ms |",
        "",
    ]
    return "\n".join(lines)


if __name__ == "__main__":
    main()
