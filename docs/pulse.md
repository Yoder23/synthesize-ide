# Pulse monitoring

Pulse is an advisory monitoring subsystem. Symbolic detectors are authoritative descriptions of recorded facts; intervention proposals still require the orchestrator or human control to act.

Production detectors cover intent, requirement, constraint, architecture, assumption, scope, and value drift; repeated failures; churn; regressions; deleted or weakened tests; repeated ADR violations; belief divergence; unresolved questions; budget pressure; stagnation; oscillation; unauthorized paths; and evidence regression. Findings include source, severity, trend, primary factors, related requirements, supporting events, and a recommended intervention.

The production temporal observer is deterministic and rule-based. It applies elapsed-time exponential decay to named risk features and supports versioned snapshot/restore. Event order alone is not treated as elapsed time.

The liquid observer is experimental and shadow-only. It validates schema version, dimensions, normalization, checksum, and calibration metadata before loading weights. Uncalibrated weights yield no findings; random weights are never silently substituted. Its exact update is a bounded leaky state update over elapsed time followed by a calibrated linear readout. It has no transition, write, merge, or truth authority, and experimental findings cannot route interventions. The experimental liquid observer does not establish truth.

Shadow results can be exported as JSONL for offline comparison. The application remains fully useful without learned weights.
