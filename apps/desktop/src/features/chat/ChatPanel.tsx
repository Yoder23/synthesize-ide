import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AgentOperation } from '@synthesize/shared-types';
import { parseAgentOperations } from '@synthesize/agent-harness';
import { classifyEndpoint } from '@synthesize/runtime-adapters';
import type { ContextBundleView, OperationEvent, RepoState, RuntimeSettings } from '../../app/App';
import type { AgentProfileId } from '../agentProfiles/AgentPanel';
import { SESSION_ID } from '../../app/App';

type ChatMessage = { id: string; role: 'user' | 'assistant' | 'system'; text: string };

type RuntimeGenerateResult = {
  provider: string;
  endpoint_url: string;
  endpoint_classification: string;
  model: string;
  content: string;
  duration_ms: number;
  input_chars: number;
  output_chars: number;
  audit_event_id?: string | null;
};

export function ChatPanel(props: {
  repo: RepoState | null;
  currentContent: string;
  selectedText?: string;
  isDirty: boolean;
  runtimeSettings: RuntimeSettings;
  agentProfileId: AgentProfileId;
  onOperations: (operations: AgentOperation[], source: { contextBundleId: string; agentProfileId: AgentProfileId }) => void;
  onContextBuilt: (bundle: ContextBundleView) => void;
  onModelCall: () => void;
  onParseFailed: (error: string, raw: string) => void;
  events: OperationEvent[];
  draft?: string;
  onDraftConsumed?: () => void;
}) {
  const [task, setTask] = useState('Fix the auth refresh bug and add a regression test.');
  const [error, setError] = useState<string | null>(null);
  const [lastRaw, setLastRaw] = useState('');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [lastParsedCount, setLastParsedCount] = useState(0);
  const [lastRuntimeResult, setLastRuntimeResult] = useState<RuntimeGenerateResult | null>(null);
  const [actionTrace, setActionTrace] = useState<string[]>([]);

  const endpointClass = useMemo(() => classifyEndpoint(props.runtimeSettings.endpointUrl), [props.runtimeSettings.endpointUrl]);

  useEffect(() => {
    if (props.draft && props.draft.trim()) {
      setTask(props.draft);
      props.onDraftConsumed?.();
    }
  }, [props.draft]);

  async function askAgent() {
    setError(null);
    setLastRaw('');
    setLastParsedCount(0);
    setLastRuntimeResult(null);
    setActionTrace([]);
    if (!props.repo) {
      setError('Open a repo first.');
      return;
    }
    if (props.isDirty) {
      setError('Editor buffer is dirty. The patch target is the disk file, so refresh/revert before planning.');
      return;
    }
    if (props.runtimeSettings.provider !== 'fake' && endpointClass !== 'local' && !props.runtimeSettings.remoteConfirmed) {
      setError('This endpoint is not local. Repo context may leave this machine. Confirm endpoint use in Runtime Control before asking the agent.');
      return;
    }

    setLoading(true);
    const userMessage: ChatMessage = { id: crypto.randomUUID(), role: 'user', text: task };
    setMessages((prev) => [...prev, userMessage]);
    try {
      const context = await invoke<ContextBundleView>('build_context_bundle', {
        req: {
          session_id: SESSION_ID,
          repo_root: props.repo.repoRoot,
          user_message: task,
          selected_file_path: props.repo.currentFilePath,
          selected_text: props.selectedText && props.selectedText.trim() ? props.selectedText : null,
          dirty_buffer_state: props.isDirty,
          provider: props.runtimeSettings.provider === 'fake' ? 'fake' : 'local-server',
          endpoint_url: props.runtimeSettings.provider === 'fake' ? null : props.runtimeSettings.endpointUrl,
          agent_profile_id: props.agentProfileId
        }
      });
      props.onContextBuilt(context);
      props.onModelCall();
      const result = await invoke<RuntimeGenerateResult>('runtime_generate', {
        req: {
          session_id: SESSION_ID,
          repo_root: props.repo.repoRoot,
          provider: props.runtimeSettings.provider === 'fake' ? 'fake' : 'local-server',
          endpoint_url: props.runtimeSettings.provider === 'fake' ? 'memory://fake-runtime' : props.runtimeSettings.endpointUrl,
          model: props.runtimeSettings.provider === 'fake' ? 'fixture-patcher' : props.runtimeSettings.modelName,
          temperature: 0.1,
          max_tokens: 4096,
          response_format: 'json_schema',
          context_bundle_id: context.context_bundle_id
        }
      });
      setLastRuntimeResult(result);
      setLastRaw(result.content);
      setMessages((prev) => [...prev, { id: crypto.randomUUID(), role: 'assistant', text: result.content || '(empty response)' }]);
      const parsed = parseAgentOperations(result.content);
      if (!parsed.ok) {
        setError(`Could not parse typed operations: ${parsed.error}`);
        props.onParseFailed(parsed.error, result.content);
        return;
      }
      setLastParsedCount(parsed.operations.length);
      setActionTrace(summarizeOperations(parsed.operations));
      props.onOperations(parsed.operations, { contextBundleId: context.context_bundle_id, agentProfileId: context.agent_profile_id as AgentProfileId });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  const isMoaMode = props.agentProfileId === 'moa-action-planner';

  return (
    <div className="panel chatbox">
      <h3>{isMoaMode ? 'MoA Action Chat' : 'Agent Chat'}</h3>
      <div className="small">Agent: {props.agentProfileId} · Runtime: {props.runtimeSettings.provider === 'fake' ? 'Fake Runtime' : props.runtimeSettings.provider === 'managed-llamacpp' ? `Managed llama.cpp · ${props.runtimeSettings.modelName || 'GGUF model'} · ${endpointClass}` : `Local Model Server · ${props.runtimeSettings.modelName || 'no model'} · ${endpointClass}`}</div>
      {isMoaMode && (
        <div className="notice ok"><strong>MoA action mode</strong><div className="small">The local model plans and emits typed operations. MoA/Synthesize governance remains the actor: validate, approve, apply, rollback, and audit. This panel shows an action trace, not hidden chain-of-thought.</div></div>
      )}
      {endpointClass !== 'local' && props.runtimeSettings.provider !== 'fake' && (
        <div className="notice error"><strong>Endpoint warning</strong><div className="small">Repo context may leave this machine. Backend endpoint approval is required before sending context.</div></div>
      )}
      <div className="chat-history">
        {messages.length === 0 ? <div className="small">No chat messages yet.</div> : messages.map((m) => (
          <div key={m.id} className={`chat-message ${m.role}`}><strong>{m.role}</strong><div>{m.text.slice(0, 1200)}</div></div>
        ))}
      </div>
      <textarea value={task} onChange={(e) => setTask(e.target.value)} placeholder="Ask the local coding agent for a repo change..." />
      <div className="row"><button disabled={!props.repo} onClick={() => setTask(`Review ${props.repo?.currentFilePath ?? 'the active file'} and explain likely bugs, test gaps, and safe next steps. Return a report unless a small patch is clearly warranted.`)}>Review active file</button><button disabled={!props.repo} onClick={() => setTask(`Use the latest governed task output or problem report to propose a small repair patch for ${props.repo?.currentFilePath ?? 'the active file'}. Return Synthesize typed operations only.`)}>Repair from output</button><button disabled={!props.repo} onClick={() => setTask(`MoA action mode: create a concise plan/action trace for improving ${props.repo?.currentFilePath ?? 'the active file'}, then emit the minimal Synthesize typed operations MoA should ask the backend to validate. Include a report operation for the trace and propose_patch only if a safe, reviewable change is warranted.`)}>Draft MoA action</button></div>
      <button className="primary" disabled={!props.repo || props.isDirty || loading} onClick={askAgent}>{loading ? 'Asking local model through backend...' : 'Ask agent'}</button>
      {lastRuntimeResult && <div className="notice ok"><strong>Runtime response</strong><div className="small">{lastRuntimeResult.endpoint_classification} · {lastRuntimeResult.duration_ms}ms · {lastRuntimeResult.output_chars} output chars · backend-derived context</div></div>}
      {lastParsedCount > 0 && <div className="notice ok"><strong>Parsed operations</strong><div className="small">{lastParsedCount} typed operation(s) parsed and sent to the diff/operation queue.</div></div>}
      {actionTrace.length > 0 && (
        <div className="opitem">
          <strong>Plan / action trace</strong>
          <ol className="small">{actionTrace.map((line, index) => <li key={`${index}-${line}`}>{line}</li>)}</ol>
        </div>
      )}
      {props.isDirty && <div className="notice error"><strong>Dirty buffer</strong><div className="small">Planning is disabled because patch validation hashes the disk file, not unsaved editor text.</div></div>}
      {error && <div className="opitem error"><strong>Error</strong><div className="small">{error}</div></div>}
      {lastRaw && <details className="opitem"><summary>Raw model payload</summary><pre className="audit-json">{lastRaw}</pre></details>}
      <div className="opitem">
        <strong>Operation timeline</strong>
        {props.events.length === 0 ? (
          <div className="small">No operations yet. Model output will be parsed into typed operations.</div>
        ) : props.events.map((event) => (
          <div key={event.id} className={`timeline ${event.status}`}>
            <strong>{event.label}</strong>
            <div className="small">{event.detail}</div>
          </div>
        ))}
      </div>
    </div>
  );
}


function summarizeOperations(operations: AgentOperation[]): string[] {
  return operations.map((operation) => {
    switch (operation.type) {
      case 'propose_patch':
        return `Propose patch ${operation.proposalId}: ${operation.summary} (${operation.files.length} file(s), ${operation.suggestedCommands.length} suggested command(s)).`;
      case 'run_command':
        return `Suggest command: ${operation.argv.join(' ')} — ${operation.reason}`;
      case 'report':
        return `Report: ${operation.summary}`;
      case 'final_report':
        return `Final report: ${operation.summary}`;
      case 'ask_user':
        return `Ask user: ${operation.question}`;
      case 'read_file':
        return `Request context file: ${operation.path} — ${operation.reason}`;
      case 'search_repo':
        return `Request repo search: ${operation.query} — ${operation.reason}`;
      default:
        return `Typed operation: ${(operation as { type: string }).type}`;
    }
  });
}
