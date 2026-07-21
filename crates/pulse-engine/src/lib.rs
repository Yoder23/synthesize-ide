use intent_ledger::{OrchestrationEvent, Role};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error;

pub const FEATURE_SCHEMA_VERSION: i64 = 1;
pub const FEATURE_NAMES: [&str; 9] = [
    "intent_alignment_risk",
    "architecture_risk",
    "coordination_deterioration",
    "stagnation",
    "scope_pressure",
    "contradiction_pressure",
    "rework_churn",
    "verification_health_risk",
    "intervention_urgency",
];

#[derive(Debug, Error)]
pub enum PulseError {
    #[error("invalid observer weights: {0}")]
    InvalidWeights(String),
    #[error("incompatible snapshot: {0}")]
    IncompatibleSnapshot(String),
    #[error("observer unavailable: {0}")]
    Unavailable(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PulseError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PulseEvent {
    pub id: String,
    pub initiative_id: String,
    pub task_id: Option<String>,
    pub actor_role: Role,
    pub kind: String,
    pub timestamp_seconds: f64,
    pub requirement_ids: Vec<String>,
    pub adr_ids: Vec<String>,
    pub assumption_ids: Vec<String>,
    pub features: BTreeMap<String, f64>,
    pub provenance: String,
    pub summary: String,
}

impl PulseEvent {
    pub fn from_orchestration(event: &OrchestrationEvent, timestamp_seconds: f64) -> Self {
        Self {
            id: event.id.clone(),
            initiative_id: event.initiative_id.clone(),
            task_id: event.task_id.clone(),
            actor_role: event.actor_role,
            kind: event.kind.clone(),
            timestamp_seconds,
            requirement_ids: event.requirement_ids.clone(),
            adr_ids: event.adr_ids.clone(),
            assumption_ids: event.assumption_ids.clone(),
            features: event.features.clone(),
            provenance: event.provenance.clone(),
            summary: event.redacted_summary.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SignalTrend {
    Rising,
    Stable,
    Falling,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PulseFinding {
    pub kind: String,
    pub severity: f64,
    pub trend: SignalTrend,
    pub source: String,
    pub experimental: bool,
    pub primary_factors: Vec<String>,
    pub related_requirements: Vec<String>,
    pub supporting_events: Vec<String>,
    pub recommended_intervention: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BeliefSnapshot {
    pub id: String,
    pub role: Role,
    pub requirement_complete: BTreeMap<String, bool>,
    pub adr_followed: BTreeMap<String, bool>,
    pub confidence: f64,
}

#[derive(Default)]
pub struct SymbolicMonitor;

impl SymbolicMonitor {
    pub fn evaluate(&self, events: &[PulseEvent], beliefs: &[BeliefSnapshot]) -> Vec<PulseFinding> {
        let mut findings = Vec::new();
        self.direct_kind_detector(
            events,
            "intent.drift",
            "intent_drift",
            "ASK_FDE",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "requirement.drift",
            "requirement_drift",
            "ASK_PLANNER",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "constraint.violation",
            "constraint_drift",
            "PAUSE_TASK",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "adr.violation",
            "architecture_drift",
            "ASK_ARCHITECT",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "assumption.invalidated",
            "assumption_invalidation",
            "REQUEST_REPLAN",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "scope.expanded",
            "scope_drift",
            "LOWER_AUTONOMY",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "value.drift",
            "value_drift",
            "ASK_FDE",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "test.deleted_after_failure",
            "test_deletion_after_failure",
            "ASK_VERIFIER",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "test.weakening_suspected",
            "test_weakening_suspicion",
            "ASK_VERIFIER",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "question.blocking_unresolved",
            "unresolved_blocking_question",
            "REQUEST_ALIGNMENT_REVIEW",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "path.unauthorized",
            "unauthorized_path_attempt",
            "PAUSE_TASK",
            &mut findings,
        );
        self.direct_kind_detector(
            events,
            "evidence.regressed",
            "evidence_regression",
            "REQUEST_EVIDENCE",
            &mut findings,
        );
        for event_kind in [
            "context.stale_spec",
            "context.stale_adr",
            "context.mandatory_missing",
            "context.critical_omitted",
            "context.summary_conflict",
            "context.summary_stale",
            "context.binding_invalid",
            "context.generation_failed_after_omission",
            "context.mandatory_overflow",
        ] {
            self.direct_kind_detector(
                events,
                event_kind,
                "context_drift",
                "REQUEST_CONTEXT_REFRESH",
                &mut findings,
            );
        }
        self.repeated_failure(events, &mut findings);
        self.diff_churn(events, &mut findings);
        self.regression_after_success(events, &mut findings);
        self.repeated_adr_violation(events, &mut findings);
        self.budget_pressure(events, &mut findings);
        self.stagnation(events, &mut findings);
        self.oscillation(events, &mut findings);
        self.context_pressure(events, &mut findings);
        if let Some(finding) = belief_divergence(beliefs) {
            findings.push(finding);
        }
        findings.sort_by(|a, b| {
            b.severity
                .total_cmp(&a.severity)
                .then_with(|| a.kind.cmp(&b.kind))
        });
        findings
    }

    fn context_pressure(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let requests: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| event.kind == "context.requested")
            .collect();
        let capsules: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| event.kind == "context.capsule_compiled")
            .collect();
        let pressure: Vec<f64> = capsules
            .iter()
            .filter_map(|event| event.features.get("token_pressure").copied())
            .collect();
        let repeated_requests = requests.len() >= 3;
        let churn = capsules
            .iter()
            .filter(|event| {
                event
                    .features
                    .get("bundle_change_fraction")
                    .is_some_and(|value| *value >= 0.5)
            })
            .count()
            >= 3;
        let low_priority_dominance = capsules.iter().any(|event| {
            event
                .features
                .get("low_priority_fraction")
                .is_some_and(|value| *value > 0.5)
        });
        let token_trend = pressure.len() >= 3
            && pressure[pressure.len() - 3] < pressure[pressure.len() - 2]
            && pressure[pressure.len() - 2] < pressure[pressure.len() - 1]
            && pressure[pressure.len() - 1] >= 0.75;
        if repeated_requests || churn || low_priority_dominance || token_trend {
            let mut factors = Vec::new();
            if repeated_requests {
                factors.push("repeated_context_requests".into());
            }
            if churn {
                factors.push("bundle_churn".into());
            }
            if low_priority_dominance {
                factors.push("low_priority_context_dominance".into());
            }
            if token_trend {
                factors.push("increasing_token_pressure".into());
            }
            let supporting_events = requests
                .iter()
                .chain(capsules.iter())
                .map(|event| event.id.clone())
                .collect();
            findings.push(finding(
                "context_pressure",
                pressure.last().copied().unwrap_or(0.65).max(0.65),
                factors,
                supporting_events,
                requirements(&capsules),
                "PARTITION_TASK_OR_REFRESH_CONTEXT",
            ));
        }
    }

    fn direct_kind_detector(
        &self,
        events: &[PulseEvent],
        event_kind: &str,
        finding_kind: &str,
        intervention: &str,
        findings: &mut Vec<PulseFinding>,
    ) {
        let matching: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| event.kind == event_kind)
            .collect();
        if !matching.is_empty() {
            let severity = matching
                .iter()
                .filter_map(|event| event.features.get("severity"))
                .copied()
                .fold(0.65, f64::max)
                .clamp(0.0, 1.0);
            findings.push(finding(
                finding_kind,
                severity,
                vec![event_kind.into()],
                matching.iter().map(|event| event.id.clone()).collect(),
                requirements(&matching),
                intervention,
            ));
        }
    }

    fn repeated_failure(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let failures: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| event.kind == "test.failed")
            .collect();
        if failures.len() >= 3 {
            findings.push(finding(
                "repeated_failure",
                (0.55 + failures.len() as f64 * 0.08).min(1.0),
                vec!["three_or_more_test_failures".into()],
                failures.iter().map(|event| event.id.clone()).collect(),
                requirements(&failures),
                "PAUSE_TASK",
            ));
        }
    }

    fn diff_churn(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let diffs: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| event.kind == "patch.applied")
            .collect();
        let total: f64 = diffs
            .iter()
            .filter_map(|event| event.features.get("changed_lines"))
            .sum();
        let reversions = events
            .iter()
            .filter(|event| event.kind == "patch.reverted")
            .count();
        if (diffs.len() >= 3 && total >= 300.0) || reversions >= 2 {
            findings.push(finding(
                "diff_churn",
                (0.5 + total / 2000.0 + reversions as f64 * 0.12).min(1.0),
                vec![
                    "increasing_changed_lines".into(),
                    "repeated_reversion".into(),
                ],
                diffs.iter().map(|event| event.id.clone()).collect(),
                requirements(&diffs),
                "REQUEST_ALIGNMENT_REVIEW",
            ));
        }
    }

    fn regression_after_success(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let mut passed = BTreeMap::<String, String>::new();
        let mut regressions = Vec::new();
        for event in events {
            for requirement in &event.requirement_ids {
                if event.kind == "test.passed" {
                    passed.insert(requirement.clone(), event.id.clone());
                } else if event.kind == "test.failed" && passed.contains_key(requirement) {
                    regressions.push(event);
                }
            }
        }
        if !regressions.is_empty() {
            findings.push(finding(
                "regression_after_prior_success",
                0.82,
                vec!["passing_requirement_failed_later".into()],
                regressions.iter().map(|event| event.id.clone()).collect(),
                requirements(&regressions),
                "ASK_VERIFIER",
            ));
        }
    }

    fn repeated_adr_violation(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let violations: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| event.kind == "adr.violation")
            .collect();
        if violations.len() >= 3 {
            findings.push(finding(
                "repeated_adr_violation",
                0.9,
                vec!["same_architecture_boundary_repeatedly_violated".into()],
                violations.iter().map(|event| event.id.clone()).collect(),
                requirements(&violations),
                "PAUSE_TASK",
            ));
        }
    }

    fn budget_pressure(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let matching: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| {
                event
                    .features
                    .get("budget_fraction")
                    .is_some_and(|fraction| *fraction >= 0.8)
            })
            .collect();
        if let Some(maximum) = matching
            .iter()
            .filter_map(|event| event.features.get("budget_fraction"))
            .copied()
            .max_by(f64::total_cmp)
        {
            findings.push(finding(
                "budget_pressure",
                maximum.clamp(0.0, 1.0),
                vec!["budget_usage_above_80_percent".into()],
                matching.iter().map(|event| event.id.clone()).collect(),
                requirements(&matching),
                "LOWER_AUTONOMY",
            ));
        }
    }

    fn stagnation(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let window: Vec<&PulseEvent> = events.iter().rev().take(12).collect();
        if window.len() >= 8
            && !window.iter().any(|event| {
                event.kind == "evidence.added"
                    || event
                        .features
                        .get("requirements_gaining_evidence")
                        .is_some_and(|value| *value > 0.0)
            })
        {
            findings.push(finding(
                "stagnation",
                0.7,
                vec!["no_evidence_gain_in_recent_window".into()],
                window.iter().map(|event| event.id.clone()).collect(),
                requirements(&window),
                "REQUEST_REPLAN",
            ));
        }
    }

    fn oscillation(&self, events: &[PulseEvent], findings: &mut Vec<PulseFinding>) {
        let outcomes: Vec<&PulseEvent> = events
            .iter()
            .filter(|event| matches!(event.kind.as_str(), "test.passed" | "test.failed"))
            .collect();
        let oscillating_window = outcomes
            .windows(4)
            .rev()
            .find(|window| window.windows(2).all(|pair| pair[0].kind != pair[1].kind));
        if let Some(window) = oscillating_window {
            findings.push(finding(
                "oscillation",
                0.78,
                vec!["alternating_test_outcomes".into()],
                window.iter().map(|event| event.id.clone()).collect(),
                requirements(window),
                "PAUSE_TASK",
            ));
        }
    }
}

fn requirements(events: &[&PulseEvent]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| event.requirement_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn finding(
    kind: &str,
    severity: f64,
    factors: Vec<String>,
    supporting_events: Vec<String>,
    related_requirements: Vec<String>,
    intervention: &str,
) -> PulseFinding {
    PulseFinding {
        kind: kind.into(),
        severity: severity.clamp(0.0, 1.0),
        trend: SignalTrend::Rising,
        source: "rule_based".into(),
        experimental: false,
        primary_factors: factors,
        related_requirements,
        supporting_events,
        recommended_intervention: intervention.into(),
    }
}

pub fn belief_divergence(beliefs: &[BeliefSnapshot]) -> Option<PulseFinding> {
    let mut discrepancies = Vec::new();
    let mut supporting = BTreeSet::new();
    for (index, left) in beliefs.iter().enumerate() {
        for right in beliefs.iter().skip(index + 1) {
            for (id, left_value) in &left.requirement_complete {
                if right
                    .requirement_complete
                    .get(id)
                    .is_some_and(|right_value| right_value != left_value)
                {
                    discrepancies.push(format!("requirement_{id}_disagreement"));
                    supporting.insert(left.id.clone());
                    supporting.insert(right.id.clone());
                }
            }
            for (id, left_value) in &left.adr_followed {
                if right
                    .adr_followed
                    .get(id)
                    .is_some_and(|right_value| right_value != left_value)
                {
                    discrepancies.push(format!("adr_{id}_disagreement"));
                    supporting.insert(left.id.clone());
                    supporting.insert(right.id.clone());
                }
            }
        }
    }
    if discrepancies.is_empty() {
        None
    } else {
        let related = discrepancies
            .iter()
            .filter_map(|factor| {
                factor
                    .strip_prefix("requirement_")
                    .and_then(|value| value.strip_suffix("_disagreement"))
            })
            .map(str::to_string)
            .collect();
        Some(PulseFinding {
            kind: "agent_belief_divergence".into(),
            severity: (0.55 + discrepancies.len() as f64 * 0.12).min(1.0),
            trend: SignalTrend::Rising,
            source: "rule_based".into(),
            experimental: false,
            primary_factors: discrepancies,
            related_requirements: related,
            supporting_events: supporting.into_iter().collect(),
            recommended_intervention: "REQUEST_ALIGNMENT_REVIEW".into(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverMetadata {
    pub observer_kind: String,
    pub model_version: String,
    pub checksum: String,
    pub calibrated: bool,
    pub experimental: bool,
    pub shadow_only: bool,
    pub has_authority: bool,
    pub mathematical_behavior: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverSnapshot {
    pub schema_version: i64,
    pub observer_kind: String,
    pub model_version: String,
    pub checksum: String,
    pub last_timestamp_seconds: Option<f64>,
    pub state: Vec<f64>,
}

pub trait TemporalObserver {
    fn ingest(&mut self, event: &PulseEvent, delta_time_seconds: f64) -> Result<()>;
    fn current_state(&self) -> Vec<f64>;
    fn evaluate(&self) -> Vec<PulseFinding>;
    fn reset(&mut self);
    fn snapshot(&self) -> ObserverSnapshot;
    fn restore(&mut self, snapshot: &ObserverSnapshot) -> Result<()>;
    fn model_metadata(&self) -> ObserverMetadata;
}

#[derive(Debug, Clone)]
pub struct RuleBasedTemporalObserver {
    state: Vec<f64>,
    last_timestamp_seconds: Option<f64>,
    recent_event_ids: VecDeque<String>,
}

impl Default for RuleBasedTemporalObserver {
    fn default() -> Self {
        Self {
            state: vec![0.0; FEATURE_NAMES.len()],
            last_timestamp_seconds: None,
            recent_event_ids: VecDeque::new(),
        }
    }
}

impl TemporalObserver for RuleBasedTemporalObserver {
    fn ingest(&mut self, event: &PulseEvent, delta_time_seconds: f64) -> Result<()> {
        let delta = delta_time_seconds.max(0.0);
        let decay = (-delta / 600.0).exp();
        for (index, name) in FEATURE_NAMES.iter().enumerate() {
            let input = event
                .features
                .get(*name)
                .copied()
                .unwrap_or_else(|| inferred_feature(event, name));
            self.state[index] =
                (self.state[index] * decay + input.clamp(0.0, 1.0) * (1.0 - decay)).clamp(0.0, 1.0);
        }
        self.last_timestamp_seconds = Some(event.timestamp_seconds);
        self.recent_event_ids.push_back(event.id.clone());
        while self.recent_event_ids.len() > 50 {
            self.recent_event_ids.pop_front();
        }
        Ok(())
    }

    fn current_state(&self) -> Vec<f64> {
        self.state.clone()
    }

    fn evaluate(&self) -> Vec<PulseFinding> {
        self.state
            .iter()
            .enumerate()
            .filter(|(_, risk)| **risk >= 0.55)
            .map(|(index, risk)| PulseFinding {
                kind: FEATURE_NAMES[index].into(),
                severity: *risk,
                trend: SignalTrend::Stable,
                source: "rule_based_temporal".into(),
                experimental: false,
                primary_factors: vec![format!("decayed_rolling_{}", FEATURE_NAMES[index])],
                related_requirements: vec![],
                supporting_events: self
                    .recent_event_ids
                    .iter()
                    .rev()
                    .take(8)
                    .cloned()
                    .collect(),
                recommended_intervention: if *risk >= 0.8 {
                    "REQUEST_ALIGNMENT_REVIEW"
                } else {
                    "REQUEST_EVIDENCE"
                }
                .into(),
            })
            .collect()
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
        self.last_timestamp_seconds = None;
        self.recent_event_ids.clear();
    }

    fn snapshot(&self) -> ObserverSnapshot {
        ObserverSnapshot {
            schema_version: FEATURE_SCHEMA_VERSION,
            observer_kind: "rule_based".into(),
            model_version: "rule-v1".into(),
            checksum: "built-in-deterministic-rules-v1".into(),
            last_timestamp_seconds: self.last_timestamp_seconds,
            state: self.state.clone(),
        }
    }

    fn restore(&mut self, snapshot: &ObserverSnapshot) -> Result<()> {
        if snapshot.schema_version != FEATURE_SCHEMA_VERSION
            || snapshot.observer_kind != "rule_based"
            || snapshot.state.len() != FEATURE_NAMES.len()
        {
            return Err(PulseError::IncompatibleSnapshot(
                "rule observer schema mismatch".into(),
            ));
        }
        self.state = snapshot.state.clone();
        self.last_timestamp_seconds = snapshot.last_timestamp_seconds;
        Ok(())
    }

    fn model_metadata(&self) -> ObserverMetadata {
        ObserverMetadata {
            observer_kind: "rule_based".into(),
            model_version: "rule-v1".into(),
            checksum: "built-in-deterministic-rules-v1".into(),
            calibrated: true,
            experimental: false,
            shadow_only: false,
            has_authority: false,
            mathematical_behavior: "Exponential decay over bounded explicit event features and deterministic thresholds.".into(),
        }
    }
}

fn inferred_feature(event: &PulseEvent, feature: &str) -> f64 {
    match (event.kind.as_str(), feature) {
        ("intent.drift", "intent_alignment_risk") => 1.0,
        ("adr.violation", "architecture_risk") => 1.0,
        ("test.failed", "verification_health_risk") => 0.8,
        ("scope.expanded", "scope_pressure") => 0.9,
        ("assumption.invalidated", "contradiction_pressure") => 0.85,
        ("patch.reverted", "rework_churn") => 0.8,
        ("question.blocking_unresolved", "coordination_deterioration") => 0.85,
        _ => 0.0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LiquidWeights {
    pub schema_version: i64,
    pub model_version: String,
    pub feature_names: Vec<String>,
    pub hidden_size: usize,
    pub output_names: Vec<String>,
    pub input_mean: Vec<f64>,
    pub input_scale: Vec<f64>,
    pub w_input: Vec<Vec<f64>>,
    pub w_state: Vec<Vec<f64>>,
    pub bias: Vec<f64>,
    pub time_constant: Vec<f64>,
    pub w_output: Vec<Vec<f64>>,
    pub output_bias: Vec<f64>,
    pub calibrated: bool,
    pub checksum: String,
}

impl LiquidWeights {
    pub fn computed_checksum(&self) -> Result<String> {
        let mut copy = self.clone();
        copy.checksum.clear();
        Ok(hex::encode(Sha256::digest(serde_json::to_vec(&copy)?)))
    }

    pub fn validate(&self) -> Result<()> {
        let inputs = FEATURE_NAMES.len();
        if self.schema_version != FEATURE_SCHEMA_VERSION
            || self.feature_names
                != FEATURE_NAMES
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect::<Vec<_>>()
            || self.hidden_size == 0
            || self.output_names
                != FEATURE_NAMES
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect::<Vec<_>>()
        {
            return Err(PulseError::InvalidWeights(
                "schema, features, or output heads are incompatible".into(),
            ));
        }
        if self.input_mean.len() != inputs
            || self.input_scale.len() != inputs
            || self
                .input_scale
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || self.w_input.len() != self.hidden_size
            || self.w_input.iter().any(|row| row.len() != inputs)
            || self.w_state.len() != self.hidden_size
            || self.w_state.iter().any(|row| row.len() != self.hidden_size)
            || self.bias.len() != self.hidden_size
            || self.time_constant.len() != self.hidden_size
            || self
                .time_constant
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || self.w_output.len() != FEATURE_NAMES.len()
            || self
                .w_output
                .iter()
                .any(|row| row.len() != self.hidden_size)
            || self.output_bias.len() != FEATURE_NAMES.len()
        {
            return Err(PulseError::InvalidWeights(
                "matrix dimensions or normalization metadata are invalid".into(),
            ));
        }
        if self.checksum != self.computed_checksum()? {
            return Err(PulseError::InvalidWeights("checksum mismatch".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LiquidTemporalObserver {
    weights: LiquidWeights,
    state: Vec<f64>,
    outputs: Vec<f64>,
    last_timestamp_seconds: Option<f64>,
    supporting_events: VecDeque<String>,
}

impl LiquidTemporalObserver {
    pub fn load(weights: LiquidWeights) -> Result<Self> {
        weights.validate()?;
        Ok(Self {
            state: vec![0.0; weights.hidden_size],
            outputs: vec![0.0; weights.output_names.len()],
            weights,
            last_timestamp_seconds: None,
            supporting_events: VecDeque::new(),
        })
    }

    pub fn can_propose_intervention(&self) -> bool {
        self.weights.calibrated
    }

    pub fn has_authority(&self) -> bool {
        false
    }
}

impl TemporalObserver for LiquidTemporalObserver {
    fn ingest(&mut self, event: &PulseEvent, delta_time_seconds: f64) -> Result<()> {
        let inputs: Vec<f64> = FEATURE_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let raw = event
                    .features
                    .get(*name)
                    .copied()
                    .unwrap_or_else(|| inferred_feature(event, name));
                (raw - self.weights.input_mean[index]) / self.weights.input_scale[index]
            })
            .collect();
        let previous = self.state.clone();
        let delta = delta_time_seconds.max(0.0);
        for hidden in 0..self.weights.hidden_size {
            let input_sum = dot(&self.weights.w_input[hidden], &inputs);
            let state_sum = dot(&self.weights.w_state[hidden], &previous);
            let candidate = (input_sum + state_sum + self.weights.bias[hidden]).tanh();
            let alpha = 1.0 - (-delta / self.weights.time_constant[hidden]).exp();
            self.state[hidden] =
                ((1.0 - alpha) * previous[hidden] + alpha * candidate).clamp(-1.0, 1.0);
        }
        for output in 0..self.outputs.len() {
            self.outputs[output] = sigmoid(
                dot(&self.weights.w_output[output], &self.state) + self.weights.output_bias[output],
            );
        }
        self.last_timestamp_seconds = Some(event.timestamp_seconds);
        self.supporting_events.push_back(event.id.clone());
        while self.supporting_events.len() > 50 {
            self.supporting_events.pop_front();
        }
        Ok(())
    }

    fn current_state(&self) -> Vec<f64> {
        self.state.clone()
    }

    fn evaluate(&self) -> Vec<PulseFinding> {
        if !self.weights.calibrated {
            return vec![];
        }
        self.outputs
            .iter()
            .enumerate()
            .filter(|(_, value)| **value >= 0.5)
            .map(|(index, value)| PulseFinding {
                kind: self.weights.output_names[index].clone(),
                severity: *value,
                trend: SignalTrend::Stable,
                source: "liquid_shadow".into(),
                experimental: true,
                primary_factors: vec!["experimental_continuous_time_hidden_state".into()],
                related_requirements: vec![],
                supporting_events: self
                    .supporting_events
                    .iter()
                    .rev()
                    .take(8)
                    .cloned()
                    .collect(),
                recommended_intervention: "REQUEST_ALIGNMENT_REVIEW".into(),
            })
            .collect()
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
        self.outputs.fill(0.0);
        self.last_timestamp_seconds = None;
        self.supporting_events.clear();
    }

    fn snapshot(&self) -> ObserverSnapshot {
        ObserverSnapshot {
            schema_version: FEATURE_SCHEMA_VERSION,
            observer_kind: "liquid_shadow".into(),
            model_version: self.weights.model_version.clone(),
            checksum: self.weights.checksum.clone(),
            last_timestamp_seconds: self.last_timestamp_seconds,
            state: self.state.clone(),
        }
    }

    fn restore(&mut self, snapshot: &ObserverSnapshot) -> Result<()> {
        if snapshot.schema_version != FEATURE_SCHEMA_VERSION
            || snapshot.observer_kind != "liquid_shadow"
            || snapshot.model_version != self.weights.model_version
            || snapshot.checksum != self.weights.checksum
            || snapshot.state.len() != self.weights.hidden_size
        {
            return Err(PulseError::IncompatibleSnapshot(
                "liquid model or state mismatch".into(),
            ));
        }
        self.state = snapshot.state.clone();
        self.last_timestamp_seconds = snapshot.last_timestamp_seconds;
        Ok(())
    }

    fn model_metadata(&self) -> ObserverMetadata {
        ObserverMetadata {
            observer_kind: "liquid_shadow".into(),
            model_version: self.weights.model_version.clone(),
            checksum: self.weights.checksum.clone(),
            calibrated: self.weights.calibrated,
            experimental: true,
            shadow_only: true,
            has_authority: false,
            mathematical_behavior: "alpha = 1-exp(-delta_time/time_constant); candidate=tanh(W_input*x + W_state*h + bias); h=(1-alpha)*h + alpha*candidate".into(),
        }
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value.clamp(-60.0, 60.0)).exp())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InterventionKind {
    AskFde,
    AskUx,
    AskArchitect,
    AskPlanner,
    AskVerifier,
    RequestEvidence,
    RequestAlignmentReview,
    RequestReplan,
    LowerAutonomy,
    PauseTask,
    PauseInitiative,
    EscalateToHuman,
    RequestContextRefresh,
    PartitionTaskOrRefreshContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InterventionProposal {
    pub kind: InterventionKind,
    pub finding_kind: String,
    pub rationale: String,
    pub requires_backend_validation: bool,
    pub mutates_state: bool,
}

pub fn route_intervention(finding: &PulseFinding) -> Option<InterventionProposal> {
    if finding.experimental {
        return None;
    }
    let kind = match finding.recommended_intervention.as_str() {
        "ASK_FDE" => InterventionKind::AskFde,
        "ASK_UX" => InterventionKind::AskUx,
        "ASK_ARCHITECT" => InterventionKind::AskArchitect,
        "ASK_PLANNER" => InterventionKind::AskPlanner,
        "ASK_VERIFIER" => InterventionKind::AskVerifier,
        "REQUEST_EVIDENCE" => InterventionKind::RequestEvidence,
        "REQUEST_ALIGNMENT_REVIEW" => InterventionKind::RequestAlignmentReview,
        "REQUEST_REPLAN" => InterventionKind::RequestReplan,
        "LOWER_AUTONOMY" => InterventionKind::LowerAutonomy,
        "PAUSE_TASK" => InterventionKind::PauseTask,
        "PAUSE_INITIATIVE" => InterventionKind::PauseInitiative,
        "ESCALATE_TO_HUMAN" => InterventionKind::EscalateToHuman,
        "REQUEST_CONTEXT_REFRESH" => InterventionKind::RequestContextRefresh,
        "PARTITION_TASK_OR_REFRESH_CONTEXT" => InterventionKind::PartitionTaskOrRefreshContext,
        _ => return None,
    };
    Some(InterventionProposal {
        kind,
        finding_kind: finding.kind.clone(),
        rationale: format!(
            "Rule-based signal {:.2}: {}",
            finding.severity,
            finding.primary_factors.join(", ")
        ),
        requires_backend_validation: true,
        mutates_state: false,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShadowTrajectoryRow {
    pub event: PulseEvent,
    pub predictions: BTreeMap<String, f64>,
    pub label: Option<String>,
    pub model: ObserverMetadata,
    pub privacy_warning: String,
}

pub fn export_shadow_jsonl(rows: &[ShadowTrajectoryRow]) -> Result<String> {
    rows.iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map(|rows| rows.join("\n"))
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, kind: &str, time: f64) -> PulseEvent {
        PulseEvent {
            id: id.into(),
            initiative_id: "INIT".into(),
            task_id: Some("TASK".into()),
            actor_role: Role::Builder,
            kind: kind.into(),
            timestamp_seconds: time,
            requirement_ids: vec!["REQ-1".into()],
            adr_ids: vec!["ADR-1".into()],
            assumption_ids: vec![],
            features: BTreeMap::new(),
            provenance: "test".into(),
            summary: kind.into(),
        }
    }

    #[test]
    fn direct_symbolic_detectors_cover_drift_and_security_events() {
        let kinds = [
            ("intent.drift", "intent_drift"),
            ("requirement.drift", "requirement_drift"),
            ("constraint.violation", "constraint_drift"),
            ("adr.violation", "architecture_drift"),
            ("assumption.invalidated", "assumption_invalidation"),
            ("scope.expanded", "scope_drift"),
            ("value.drift", "value_drift"),
            ("test.deleted_after_failure", "test_deletion_after_failure"),
            ("test.weakening_suspected", "test_weakening_suspicion"),
            (
                "question.blocking_unresolved",
                "unresolved_blocking_question",
            ),
            ("path.unauthorized", "unauthorized_path_attempt"),
            ("evidence.regressed", "evidence_regression"),
        ];
        for (input, expected) in kinds {
            let findings = SymbolicMonitor.evaluate(&[event("e", input, 1.0)], &[]);
            assert!(
                findings.iter().any(|finding| finding.kind == expected),
                "missing {expected}"
            );
            assert!(findings
                .iter()
                .all(|finding| !finding.supporting_events.is_empty()));
        }
    }

    #[test]
    fn repeated_failure_churn_regression_stagnation_and_oscillation_are_detected() {
        let mut events = vec![];
        for index in 0..4 {
            events.push(event(
                &format!("t{index}"),
                if index % 2 == 0 {
                    "test.passed"
                } else {
                    "test.failed"
                },
                index as f64,
            ));
        }
        for index in 0..3 {
            let mut applied = event(&format!("p{index}"), "patch.applied", 10.0 + index as f64);
            applied.features.insert("changed_lines".into(), 140.0);
            events.push(applied);
        }
        events.push(event("f3", "test.failed", 20.0));
        events.push(event("f4", "test.failed", 21.0));
        let findings = SymbolicMonitor.evaluate(&events, &[]);
        for expected in [
            "repeated_failure",
            "diff_churn",
            "regression_after_prior_success",
            "oscillation",
        ] {
            assert!(
                findings.iter().any(|finding| finding.kind == expected),
                "missing {expected}"
            );
        }

        let idle: Vec<PulseEvent> = (0..10)
            .map(|index| event(&format!("i{index}"), "role.completed", index as f64))
            .collect();
        assert!(SymbolicMonitor
            .evaluate(&idle, &[])
            .iter()
            .any(|finding| finding.kind == "stagnation"));
    }

    #[test]
    fn repeated_adr_and_budget_pressure_are_detected() {
        let mut events = vec![
            event("a", "adr.violation", 1.0),
            event("b", "adr.violation", 2.0),
            event("c", "adr.violation", 3.0),
        ];
        let mut budget = event("budget", "budget.warning", 4.0);
        budget.features.insert("budget_fraction".into(), 0.91);
        events.push(budget);
        let findings = SymbolicMonitor.evaluate(&events, &[]);
        assert!(findings
            .iter()
            .any(|finding| finding.kind == "repeated_adr_violation"));
        assert!(findings
            .iter()
            .any(|finding| finding.kind == "budget_pressure"));
    }

    #[test]
    fn context_drift_and_pressure_are_deterministic_and_explainable() {
        let mut events = vec![event("stale", "context.stale_spec", 1.0)];
        for index in 0..3 {
            events.push(event(
                &format!("request-{index}"),
                "context.requested",
                2.0 + index as f64,
            ));
            let mut capsule = event(
                &format!("capsule-{index}"),
                "context.capsule_compiled",
                5.0 + index as f64,
            );
            capsule
                .features
                .insert("token_pressure".into(), 0.70 + index as f64 * 0.1);
            capsule
                .features
                .insert("bundle_change_fraction".into(), 0.75);
            if index == 2 {
                capsule.features.insert("low_priority_fraction".into(), 0.6);
            }
            events.push(capsule);
        }
        let findings = SymbolicMonitor.evaluate(&events, &[]);
        let drift = findings
            .iter()
            .find(|finding| finding.kind == "context_drift")
            .unwrap();
        assert_eq!(drift.recommended_intervention, "REQUEST_CONTEXT_REFRESH");
        assert!(drift.supporting_events.contains(&"stale".to_string()));
        let pressure = findings
            .iter()
            .find(|finding| finding.kind == "context_pressure")
            .unwrap();
        for factor in [
            "repeated_context_requests",
            "bundle_churn",
            "low_priority_context_dominance",
            "increasing_token_pressure",
        ] {
            assert!(pressure.primary_factors.contains(&factor.to_string()));
        }
        let proposal = route_intervention(pressure).unwrap();
        assert!(!proposal.mutates_state);
        assert_eq!(
            proposal.kind,
            InterventionKind::PartitionTaskOrRefreshContext
        );
    }

    #[test]
    fn belief_divergence_proposes_alignment_without_authority() {
        let beliefs = vec![
            BeliefSnapshot {
                id: "builder".into(),
                role: Role::Builder,
                requirement_complete: BTreeMap::from([("REQ-1".into(), true)]),
                adr_followed: BTreeMap::from([("ADR-1".into(), true)]),
                confidence: 0.9,
            },
            BeliefSnapshot {
                id: "verifier".into(),
                role: Role::Verifier,
                requirement_complete: BTreeMap::from([("REQ-1".into(), false)]),
                adr_followed: BTreeMap::from([("ADR-1".into(), false)]),
                confidence: 0.7,
            },
        ];
        let finding = belief_divergence(&beliefs).unwrap();
        assert_eq!(finding.recommended_intervention, "REQUEST_ALIGNMENT_REVIEW");
        let proposal = route_intervention(&finding).unwrap();
        assert!(proposal.requires_backend_validation);
        assert!(!proposal.mutates_state);
    }

    #[test]
    fn rule_observer_uses_elapsed_time_and_restores_snapshots() {
        let mut observer = RuleBasedTemporalObserver::default();
        let mut risk = event("risk", "intent.drift", 10.0);
        risk.features.insert("intent_alignment_risk".into(), 1.0);
        observer.ingest(&risk, 600.0).unwrap();
        assert!(observer.current_state()[0] > 0.6);
        let snapshot = observer.snapshot();
        let mut restored = RuleBasedTemporalObserver::default();
        restored.restore(&snapshot).unwrap();
        assert_eq!(restored.current_state(), observer.current_state());
        assert!(!restored.model_metadata().has_authority);
    }

    fn weights(calibrated: bool) -> LiquidWeights {
        let hidden = 2;
        let mut weights = LiquidWeights {
            schema_version: FEATURE_SCHEMA_VERSION,
            model_version: "fixture-liquid-v1".into(),
            feature_names: FEATURE_NAMES.iter().map(|s| (*s).into()).collect(),
            hidden_size: hidden,
            output_names: FEATURE_NAMES.iter().map(|s| (*s).into()).collect(),
            input_mean: vec![0.0; FEATURE_NAMES.len()],
            input_scale: vec![1.0; FEATURE_NAMES.len()],
            w_input: vec![vec![0.1; FEATURE_NAMES.len()]; hidden],
            w_state: vec![vec![0.05; hidden]; hidden],
            bias: vec![0.0; hidden],
            time_constant: vec![5.0, 10.0],
            w_output: vec![vec![0.3; hidden]; FEATURE_NAMES.len()],
            output_bias: vec![0.0; FEATURE_NAMES.len()],
            calibrated,
            checksum: String::new(),
        };
        weights.checksum = weights.computed_checksum().unwrap();
        weights
    }

    #[test]
    fn liquid_observer_is_deterministic_elapsed_time_aware_and_restorable() {
        let fixture = weights(true);
        let mut first = LiquidTemporalObserver::load(fixture.clone()).unwrap();
        let mut second = LiquidTemporalObserver::load(fixture.clone()).unwrap();
        let risk = event("risk", "intent.drift", 10.0);
        first.ingest(&risk, 2.0).unwrap();
        second.ingest(&risk, 2.0).unwrap();
        assert_eq!(first.current_state(), second.current_state());
        let short_delta = first.current_state();
        first.reset();
        first.ingest(&risk, 20.0).unwrap();
        assert_ne!(first.current_state(), short_delta);
        let snapshot = first.snapshot();
        let mut restored = LiquidTemporalObserver::load(fixture).unwrap();
        restored.restore(&snapshot).unwrap();
        assert_eq!(restored.current_state(), first.current_state());
        assert!(!restored.has_authority());
    }

    #[test]
    fn invalid_checksum_and_uncalibrated_active_mode_are_refused() {
        let mut invalid = weights(true);
        invalid.checksum = "forged".into();
        assert!(LiquidTemporalObserver::load(invalid).is_err());
        let mut observer = LiquidTemporalObserver::load(weights(false)).unwrap();
        observer
            .ingest(&event("risk", "intent.drift", 1.0), 10.0)
            .unwrap();
        assert!(!observer.can_propose_intervention());
        assert!(observer.evaluate().is_empty());
        assert!(observer.model_metadata().shadow_only);
    }

    #[test]
    fn experimental_signal_cannot_route_an_intervention() {
        let signal = PulseFinding {
            kind: "scope_pressure".into(),
            severity: 0.99,
            trend: SignalTrend::Rising,
            source: "liquid_shadow".into(),
            experimental: true,
            primary_factors: vec!["opaque".into()],
            related_requirements: vec![],
            supporting_events: vec!["e".into()],
            recommended_intervention: "PAUSE_INITIATIVE".into(),
        };
        assert!(route_intervention(&signal).is_none());
    }
}
