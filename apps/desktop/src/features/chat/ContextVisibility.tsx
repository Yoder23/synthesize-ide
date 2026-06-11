import { classifyEndpoint } from '@synthesize/runtime-adapters';
import type { ContextBundleView, RepoState, RuntimeSettings } from '../../app/App';

export function ContextVisibility(props: { repo: RepoState | null; currentContent: string; isDirty: boolean; runtimeSettings: RuntimeSettings; contextBundle: ContextBundleView | null }) {
  const endpointClass = classifyEndpoint(props.runtimeSettings.endpointUrl);
  const visibleFiles = props.repo?.files.filter((f) => !f.denied).slice(0, 40) ?? [];
  return (
    <div className="panel">
      <h3>Context Sent to Model</h3>
      {!props.repo ? (
        <div className="small">Open a repo to build a context bundle.</div>
      ) : props.contextBundle ? (
        <div className="diffitem">
          <strong>Exact persisted context bundle</strong>
          <div className="small">Bundle ID: <code>{props.contextBundle.context_bundle_id}</code></div>
          <div className="small">Source agent profile: <code>{props.contextBundle.agent_profile_id}</code></div>
          <div className="small">Destination: {props.runtimeSettings.provider === 'fake' ? 'in-memory fake runtime' : `${props.contextBundle.endpoint_classification} local model server`}</div>
          <div className="small">Repo context leaves machine: {props.contextBundle.endpoint_classification === 'local' ? 'No remote endpoint selected by Synthesize' : 'Yes, if this endpoint is reachable off-machine'}</div>
          <div className="small">Warning: {props.contextBundle.destination_warning}</div>
          <div className="small">Selected file: <code>{props.contextBundle.selected_file_path}</code></div>
          <div className="small">Approx context size: {props.contextBundle.char_estimate} characters</div>
          <div className="small">Prompt/messages hash: <code>{props.contextBundle.messages_sha256}</code></div>
          <div className="small">Context precision: {props.contextBundle.exact_context ? 'This is the exact persisted context used by backend runtime_generate.' : 'Preview only / approximate.'}</div>
          <div className="small">Current commit: {props.contextBundle.git_commit ?? 'none / non-git repo'}</div>
          <div className="small">Dirty buffer at build: {props.contextBundle.dirty_buffer_state ? 'dirty' : 'clean'}</div>
          <details open>
            <summary>Included context items</summary>
            {props.contextBundle.included.map((item, idx) => (
              <div className="small" key={`${item.kind}-${idx}`}>{item.kind}: {item.path ?? 'metadata'} · {item.chars} chars · {item.note}</div>
            ))}
          </details>
          <details>
            <summary>Exact messages sent to runtime</summary>
            <div className="notice"><strong>Local persistence notice</strong><div className="small">Exact messages may include repo code and user prompt text. Synthesize stores this context bundle locally in the repo .synthesize database until you clear session data.</div></div>
            <pre className="audit-json">{JSON.stringify(props.contextBundle.messages, null, 2)}</pre>
          </details>
        </div>
      ) : (
        <div className="diffitem">
          <strong>Preview only — no context bundle has been sent yet</strong>
          <div className="small">Destination: {props.runtimeSettings.provider === 'fake' ? 'in-memory fake runtime' : `${endpointClass} local model server`}</div>
          {props.runtimeSettings.provider !== 'fake' && endpointClass !== 'local' && <div className="small error-text">Non-local model server warning: repo context may leave this machine and backend approval is required.</div>}
          <div className="small">Selected file: <code>{props.repo.currentFilePath}</code></div>
          <div className="small">Current file lines: {props.currentContent.split('\n').length}</div>
          <div className="small">Buffer state: {props.isDirty ? 'dirty — sending is disabled' : 'clean — backend will read disk through RepoGuard'}</div>
          <details>
            <summary>File tree excerpt preview</summary>
            <pre className="audit-json">{visibleFiles.map((f) => `${f.kind} ${f.path}`).join('\n')}</pre>
          </details>
          <div className="small">Excluded by policy: hidden/credential-like files, .git internals, .synthesize internals, dependency/build directories.</div>
        </div>
      )}
    </div>
  );
}
