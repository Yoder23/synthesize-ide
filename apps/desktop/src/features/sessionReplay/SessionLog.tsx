import type { AuditEvent } from '../../app/App';

export function SessionLog(props: { events: AuditEvent[]; onRefresh: () => void; onClearLocalData: () => void }) {
  return (
    <div className="panel">
      <div className="panel-heading-row">
        <h3>Session Log</h3>
        <button onClick={props.onRefresh}>Refresh</button>
        <button onClick={props.onClearLocalData}>Clear local context/audit data</button>
      </div>
      <div className="small">Context bundles and audit events are persisted locally in the repo <code>.synthesize</code> database. Clearing local session data does not delete patch lifecycle records or checkpoints.</div>
      {props.events.length === 0 ? (
        <div className="small">No audit events yet. Chat, validation, approval, apply, checkpoint, and rollback events will appear here.</div>
      ) : props.events.map((event) => {
        const summary = summarizeEvent(event);
        return (
          <details className="audit-event" key={event.id}>
            <summary><strong>{event.kind}</strong> <span className="small">{event.timestamp} · {summary}</span></summary>
            <pre className="audit-json">{pretty(event.payload_json)}</pre>
          </details>
        );
      })}
    </div>
  );
}

function summarizeEvent(event: AuditEvent): string {
  try {
    const p = JSON.parse(event.payload_json) as Record<string, unknown>;
    const parts = [
      p.proposal_id ? `proposal ${String(p.proposal_id)}` : '',
      p.approval_id ? `approval ${String(p.approval_id)}` : '',
      p.checkpoint_id ? `checkpoint ${String(p.checkpoint_id)}` : '',
      p.context_bundle_id ? `context ${String(p.context_bundle_id)}` : '',
      p.endpoint_classification ? `endpoint ${String(p.endpoint_classification)}` : '',
      p.model ? `model ${String(p.model)}` : '',
      p.error ? `error ${String(p.error).slice(0, 120)}` : ''
    ].filter(Boolean);
    return parts.join(' · ') || 'event recorded';
  } catch {
    return 'event recorded';
  }
}

function pretty(value: string): string {
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}
