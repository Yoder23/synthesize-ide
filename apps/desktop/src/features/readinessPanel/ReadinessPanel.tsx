import type { RepoState, RuntimeSettings } from '../../app/App';
import type { AgentProfileId } from '../agentProfiles/AgentPanel';
import { classifyEndpoint } from '@synthesize/runtime-adapters';

export function ReadinessPanel(props: { repo: RepoState | null; runtimeSettings: RuntimeSettings; agentProfileId: AgentProfileId; isDirty: boolean }) {
  const endpointClass = classifyEndpoint(props.runtimeSettings.endpointUrl);
  const runtimeReady = props.runtimeSettings.provider === 'fake' || endpointClass === 'local' || props.runtimeSettings.remoteConfirmed;
  return (
    <div className="panel readiness-panel">
      <h3>Ready to Work</h3>
      <div className="small">Personal-production workflow checklist. Synthesize is strongest when used on a clean branch with a local/self-hosted model.</div>
      <ul className="checklist">
        <li className={props.repo ? 'ok' : 'warn'}>{props.repo ? 'Repo open' : 'Open a repo'}</li>
        <li className={runtimeReady ? 'ok' : 'warn'}>{runtimeReady ? `Runtime selected: ${props.runtimeSettings.provider}` : 'Confirm non-local runtime before sending context'}</li>
        <li className={props.agentProfileId === 'local-patcher' || props.agentProfileId === 'fake-demo' ? 'ok' : 'warn'}>Agent profile: {props.agentProfileId}{props.agentProfileId.includes('reviewer') || props.agentProfileId.includes('planner') ? ' · report-only' : ''}</li>
        <li className={!props.isDirty ? 'ok' : 'warn'}>{props.isDirty ? 'Save or refresh dirty file before agent patch planning' : 'Active file clean for patch planning'}</li>
        <li className="ok">Patch lifecycle: backend validation → approval → checkpointed apply → rollback</li>
        <li className="ok">Execution: personal terminal + backend-detected task snapshots</li>
      </ul>
    </div>
  );
}
