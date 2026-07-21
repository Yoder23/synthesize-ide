# Test plan

Unit tests cover serialization, enum/state compatibility, migration order/idempotence, role/artifact permissions, spec binding, mandates/budgets, requirement evidence gates, verdict routing, prototype validation, context redaction/order/budget, symbolic detectors, belief divergence, temporal observer snapshot/restore/checksum/calibration, worktree path/repository/base protections, no-progress, and proof reports.

Integration tests use temporary SQLite databases and Git repositories. They exercise Studio discovery/scope approval, deterministic Builder/Verifier/Reviewer paths, revision/replan/block/pass routing, immutable spec versions, Dream rejection without mutation, approved isolated prototypes, active-branch integrity, budget stops, restart/resume, forged binding rejection, and Assist schema compatibility.

Frontend tests cover pure workspace reducers and prototype interactions without requiring a heavyweight browser harness. Production typecheck/build remains part of every release gate. Manual QA covers actual Tauri rendering, keyboard/focus behavior, restart recovery, Git worktree review, and real configured model runtimes.

