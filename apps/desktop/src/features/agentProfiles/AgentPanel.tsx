export type AgentProfileId = 'fake-demo' | 'local-planner' | 'local-patcher' | 'local-reviewer' | 'moa-action-planner';

const profileDescriptions: Record<AgentProfileId, { label: string; operations: string; guidance: string }> = {
  'fake-demo': {
    label: 'Fake Demo Agent',
    operations: 'report, ask_user, propose_patch, request_command suggestions',
    guidance: 'Deterministic fixture path for testing the governed patch lifecycle.'
  },
  'local-planner': {
    label: 'Local Planner',
    operations: 'report, ask_user; patching discouraged unless explicitly requested',
    guidance: 'Plans changes and requests context without implying apply authority.'
  },
  'local-patcher': {
    label: 'Local Patcher',
    operations: 'propose_patch, report, ask_user, request_command suggestions',
    guidance: 'May propose small unified-diff patches. Backend validation/approval/apply remain authoritative.'
  },
  'local-reviewer': {
    label: 'Local Reviewer',
    operations: 'report primarily; propose_patch only if explicitly requested',
    guidance: 'Reviews risk and correctness. Does not imply authority to change files.'
  },
  'moa-action-planner': {
    label: 'MoA Action Planner',
    operations: 'plan/action trace, report, ask_user, propose_patch, guarded command suggestions',
    guidance: 'Uses the local model for planning and emits typed operations for MoA/Synthesize governance. It can propose actions, but Synthesize validates, approves, applies, and audits them.'
  }
};

export function AgentPanel(props: { selected: AgentProfileId; onChange: (id: AgentProfileId) => void }) {
  const selected = profileDescriptions[props.selected];
  return (
    <div className="panel">
      <h3>Local Agent Profile</h3>
      <select style={{ width: '100%' }} value={props.selected} onChange={(e) => props.onChange(e.target.value as AgentProfileId)}>
        {Object.entries(profileDescriptions).map(([id, profile]) => <option key={id} value={id}>{profile.label}</option>)}
      </select>
      <div className="small" style={{ marginTop: 8 }}>
        Agent profiles feed the backend-generated system prompt. They shape local-model operation guidance, but do not grant filesystem or command authority.
      </div>
      <ul className="small">
        <li><strong>Selected:</strong> {selected.label}</li>
        <li><strong>Allowed operation guidance:</strong> {selected.operations}</li>
        <li><strong>Behavior:</strong> {selected.guidance}</li>
        <li>Command execution: disabled; suggested commands are classification-only.</li>
        <li>Patch authority: whole-proposal backend approval only in this build.</li>
        <li>MoA mode: the model plans; typed operations still pass through Synthesize validation, approval, rollback, and audit.</li>
      </ul>
    </div>
  );
}
