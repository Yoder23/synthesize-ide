import type { ProposePatchOperation } from '@synthesize/shared-types';
import type {
  ApplyResult,
  AuditEvent,
  OperationEvent,
  ProposalSource,
  ProposalUiState,
  RollbackResult,
  RuntimeSettings,
  ValidationResult
} from '../../app/App';
import type { AgentProfileId } from '../agentProfiles/AgentPanel';

export function MissionControlPanel(props: {
  runtimeSettings: RuntimeSettings;
  endpointClass: string;
  agentProfileId: AgentProfileId;
  patches: ProposePatchOperation[];
  validationByProposal: ProposalUiState<ValidationResult>;
  applyByProposal: ProposalUiState<ApplyResult>;
  rollbackByProposal: ProposalUiState<RollbackResult>;
  proposalSourceByProposal: Record<string, ProposalSource | undefined>;
  events: OperationEvent[];
  auditEvents: AuditEvent[];
}) {
  const latestPatch = props.patches[0];
  const latestValidation = latestPatch ? props.validationByProposal[latestPatch.proposalId] : undefined;
  const latestApply = latestPatch ? props.applyByProposal[latestPatch.proposalId] : undefined;
  const latestRollback = latestPatch ? props.rollbackByProposal[latestPatch.proposalId] : undefined;
  const latestSource = latestPatch ? props.proposalSourceByProposal[latestPatch.proposalId] : undefined;
  const isLocalRuntime = props.runtimeSettings.provider === 'managed-llamacpp' || props.endpointClass === 'local';
  const moaActive = props.agentProfileId === 'moa-action-planner' || latestSource?.agentProfileId === 'moa-action-planner';
  const auditKinds = new Set(props.auditEvents.map((event) => event.kind));
  const hasUnsafeBlock = props.auditEvents.some((event) => {
    const payload = event.payload_json.toLowerCase();
    return event.kind.includes('reject') || payload.includes('irreversibility') || payload.includes('rejected') || payload.includes('blocked');
  });
  const latestAudit = props.auditEvents.slice(0, 5);
  const blockedCount = props.auditEvents.filter((event) => {
    const payload = event.payload_json.toLowerCase();
    return event.kind.includes('reject') || payload.includes('rejected') || payload.includes('blocked');
  }).length;
  const appliedCount = props.auditEvents.filter((event) => event.kind.includes('applied')).length;
  const rollbackCount = props.auditEvents.filter((event) => event.kind.includes('rolled_back')).length;
  const expectedKinds = [
    'context.bundle_created',
    'runtime.request_completed',
    'operation.parse_succeeded',
    'moa.gate_decision',
    'patch.validated',
    'patch.applied',
    'patch.rolled_back'
  ];
  const evidenceKindsPresent = expectedKinds.filter((kind) => auditKinds.has(kind));
  const evidencePercent = Math.round((evidenceKindsPresent.length / expectedKinds.length) * 100);
  const noveltyScore = Math.min(100, Math.round(
    (moaActive ? 25 : 0)
    + (isLocalRuntime ? 20 : 0)
    + (latestValidation?.ok ? 20 : 0)
    + (latestApply ? 15 : 0)
    + (rollbackCount > 0 ? 10 : 0)
    + (blockedCount > 0 ? 10 : 0)
  ));
  const requestDurations = props.auditEvents
    .filter((event) => event.kind === 'runtime.request_completed')
    .map((event) => parseDurationMs(event.payload_json))
    .filter((value): value is number => value !== null)
    .sort((a, b) => a - b);
  const p50Latency = requestDurations.length ? requestDurations[Math.floor(requestDurations.length / 2)] : null;

  const cells: Array<{ label: string; value: string; state: 'ready' | 'warn' | 'live' }> = [
    {
      label: 'Local model',
      value: isLocalRuntime ? runtimeLabel(props.runtimeSettings) : 'Remote or fake runtime selected',
      state: isLocalRuntime ? 'ready' : 'warn'
    },
    {
      label: 'Typed operation',
      value: latestPatch ? `${latestPatch.files.length} file patch proposal` : 'Waiting for model output',
      state: latestPatch ? 'ready' : 'warn'
    },
    {
      label: 'MoA authority gate',
      value: moaActive ? 'MoA profile active; backend gate required' : 'Switch to MoA Action Planner for judging',
      state: moaActive ? 'live' : 'warn'
    },
    {
      label: 'Backend validation',
      value: latestValidation ? `${latestValidation.status}: ${latestValidation.message}` : 'No proposal validated yet',
      state: latestValidation?.ok ? 'ready' : 'warn'
    },
    {
      label: 'Checkpointed apply',
      value: latestApply ? `Checkpoint ${latestApply.checkpoint_id}` : 'Awaiting approval and apply',
      state: latestApply ? 'ready' : 'warn'
    },
    {
      label: 'Rollback proof',
      value: latestRollback ? `${latestRollback.restored_paths.length} restored path(s)` : 'Available after apply',
      state: latestRollback ? 'ready' : 'warn'
    },
    {
      label: 'Unsafe action proof',
      value: hasUnsafeBlock ? 'High-risk action blocked and recorded' : 'Run the winning demo to show rejection',
      state: hasUnsafeBlock ? 'ready' : 'warn'
    },
    {
      label: 'Audit replay',
      value: props.auditEvents.length ? `${props.auditEvents.length} persisted event(s)` : 'No persisted events yet',
      state: props.auditEvents.length ? 'ready' : 'warn'
    }
  ];

  return (
    <div className="panel mission-control">
      <div className="mission-kicker">Competition Demo Cockpit</div>
      <h3>MoA Mission Control</h3>
      <div className="mission-claim">
        Local model proposes. MoA reasons. The IDE enforces authority, evidence, checkpoints, rollback, and audit.
      </div>
      <div className="mission-grid">
        {cells.map((cell) => (
          <div className={`mission-cell ${cell.state}`} key={cell.label}>
            <span>{cell.label}</span>
            <strong>{cell.value}</strong>
          </div>
        ))}
      </div>

      <div className="mission-flow" aria-label="agent action pipeline">
        <div>Prompt</div>
        <div>Context Bundle</div>
        <div>Local Model</div>
        <div>Typed Operation</div>
        <div>MoA Gate</div>
        <div>Backend Apply</div>
        <div>Audit Replay</div>
      </div>

      <div className="mission-scorecard">
        <div>
          <span>Novelty score</span>
          <strong>{noveltyScore}/100</strong>
        </div>
        <div>
          <span>Evidence completeness</span>
          <strong>{evidencePercent}%</strong>
        </div>
        <div>
          <span>Blocked unsafe actions</span>
          <strong>{blockedCount}</strong>
        </div>
        <div>
          <span>P50 model response</span>
          <strong>{p50Latency !== null ? `${p50Latency} ms` : 'No samples yet'}</strong>
        </div>
        <div>
          <span>Applied proposals</span>
          <strong>{appliedCount}</strong>
        </div>
        <div>
          <span>Rollback proofs</span>
          <strong>{rollbackCount}</strong>
        </div>
      </div>

      <div className="mission-proof-row">
        <div>
          <strong>Current proposal</strong>
          <div className="small">{latestPatch ? latestPatch.summary : 'Ask the MoA Action Planner for a patch to populate the live proof.'}</div>
          {latestPatch && <div className="small">Proposal {latestPatch.proposalId} from {latestSource?.agentProfileId ?? props.agentProfileId}</div>}
        </div>
        <div>
          <strong>Evidence captured</strong>
          <div className="small">
            {auditKinds.size ? Array.from(auditKinds).slice(0, 5).join(', ') : 'No audit kinds recorded yet'}
          </div>
        </div>
      </div>

      <div className="mission-timeline">
        {(props.events.length ? props.events.slice(0, 4) : [{ id: 'empty', label: 'Ready', detail: 'Generate a local MoA action to start the live trace.', status: 'info' as const }]).map((event) => (
          <div className={`mission-event ${event.status}`} key={event.id}>
            <strong>{event.label}</strong>
            <span>{event.detail}</span>
          </div>
        ))}
      </div>

      {latestAudit.length > 0 && (
        <div className="mission-audit-strip">
          {latestAudit.map((event) => (
            <div key={event.id}>
              <strong>{event.kind}</strong>
              <span>{event.timestamp}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function runtimeLabel(settings: RuntimeSettings): string {
  if (settings.provider === 'managed-llamacpp') return `Managed llama.cpp / ${settings.modelName}`;
  if (settings.provider === 'cloud-openai') return `OpenAI cloud / ${settings.modelName}`;
  if (settings.provider === 'cloud-anthropic') return `Anthropic cloud / ${settings.modelName}`;
  if (settings.provider === 'local-server') return `OpenAI-compatible local / ${settings.modelName}`;
  return 'Fake runtime';
}

function parseDurationMs(payloadJson: string): number | null {
  try {
    const payload = JSON.parse(payloadJson) as { duration_ms?: unknown };
    if (typeof payload.duration_ms === 'number' && Number.isFinite(payload.duration_ms)) {
      return Math.max(0, Math.round(payload.duration_ms));
    }
  } catch {
    // Ignore malformed payloads and leave metrics empty.
  }
  return null;
}
