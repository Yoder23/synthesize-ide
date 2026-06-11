import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { classifyEndpoint } from '@synthesize/runtime-adapters';
import type { RepoState, RuntimeSettings } from '../../app/App';
import { SESSION_ID } from '../../app/App';

type RuntimeStatusView = { active_runtime: string; loaded_model: string | null; local_only_target: boolean; llamacpp_supervisor: string; notes: string[] };
type CuratedModelView = { id: string; name: string; runtime: string; format: string; recommended_ram_gb: number; notes: string };
type RuntimeHealthResult = { ok: boolean; provider: string; endpoint_url: string; endpoint_classification: string; model: string; message: string };
type RuntimeModelView = { id: string };
type EndpointApprovalStatus = { endpoint_url: string; endpoint_classification: string; approved: boolean; allow_repo_context: boolean; approved_at?: string | null };
type RuntimePresetView = { id: string; label: string; default_url: string; protocol: string; notes: string; local_by_default: boolean };
type LocalModelView = { id: string; display_name: string; local_path: string; format: string; runtime_compatibility: string; size_bytes?: number | null; sha256?: string | null };
type ManagedStatus = { status: string; endpoint_url?: string | null; pid?: number | null; model_path?: string | null; binary_path?: string | null; stdout_tail?: string | null; stderr_tail?: string | null; message: string };

export function RuntimeControl(props: { settings: RuntimeSettings; onChange: (settings: RuntimeSettings) => void; repo: RepoState | null }) {
  const [status, setStatus] = useState<RuntimeStatusView | null>(null);
  const [models, setModels] = useState<CuratedModelView[]>([]);
  const [presets, setPresets] = useState<RuntimePresetView[]>([]);
  const [importedModels, setImportedModels] = useState<LocalModelView[]>([]);
  const [endpointModels, setEndpointModels] = useState<RuntimeModelView[]>([]);
  const [localPath, setLocalPath] = useState('');
  const [modelDisplayName, setModelDisplayName] = useState('Local GGUF coding model');
  const [llamaBinary, setLlamaBinary] = useState('');
  const [llamaModel, setLlamaModel] = useState('');
  const [llamaPort, setLlamaPort] = useState('8080');
  const [llamaCtx, setLlamaCtx] = useState('8192');
  const [managedStatus, setManagedStatus] = useState<ManagedStatus | null>(null);
  const [message, setMessage] = useState('');
  const [health, setHealth] = useState('not tested');
  const [approvalStatus, setApprovalStatus] = useState<EndpointApprovalStatus | null>(null);
  const endpointClass = useMemo(() => classifyEndpoint(props.settings.endpointUrl), [props.settings.endpointUrl]);

  function providerForBackend() { return props.settings.provider === 'fake' ? 'fake' : 'local-server'; }

  async function refresh() {
    setStatus(await invoke<RuntimeStatusView>('runtime_status'));
    setModels(await invoke<CuratedModelView[]>('list_curated_models'));
    setPresets(await invoke<RuntimePresetView[]>('list_runtime_presets'));
    setImportedModels(await invoke<LocalModelView[]>('list_local_models'));
    setManagedStatus(await invoke<ManagedStatus>('managed_llamacpp_status'));
  }

  async function importModel() {
    setMessage('');
    try {
      const result = await invoke<LocalModelView>('import_local_model', { req: { display_name: modelDisplayName, local_path: localPath, calculate_sha256: false } });
      setMessage(`Imported ${result.display_name} (${result.format}) for ${result.runtime_compatibility}.`);
      setLlamaModel(result.local_path);
      await refresh();
    } catch (err) { setMessage(String(err)); }
  }

  async function testEndpoint() {
    setHealth('testing through Tauri backend...');
    try {
      const result = await invoke<RuntimeHealthResult>('runtime_health_check', { req: { session_id: SESSION_ID, repo_root: props.repo?.repoRoot ?? null, provider: providerForBackend(), endpoint_url: props.settings.provider === 'fake' ? 'memory://fake-runtime' : props.settings.endpointUrl, model: props.settings.provider === 'fake' ? 'fixture-patcher' : props.settings.modelName } });
      setHealth(`${result.ok ? 'ready' : 'failed'} · ${result.endpoint_classification}: ${result.message}`);
    } catch (err) { setHealth(String(err)); }
  }

  async function listEndpointModels() {
    try {
      const result = await invoke<RuntimeModelView[]>('list_runtime_models', { req: { session_id: SESSION_ID, repo_root: props.repo?.repoRoot ?? null, provider: providerForBackend(), endpoint_url: props.settings.provider === 'fake' ? 'memory://fake-runtime' : props.settings.endpointUrl } });
      setEndpointModels(result);
    } catch (err) { setMessage(String(err)); }
  }

  async function refreshEndpointApprovalStatus() {
    if (!props.repo) { setApprovalStatus(null); return; }
    try {
      const result = await invoke<EndpointApprovalStatus>('runtime_endpoint_approval_status', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, endpoint_url: props.settings.endpointUrl } });
      setApprovalStatus(result);
      if (result.approved && result.endpoint_classification !== 'local' && !props.settings.remoteConfirmed) update({ remoteConfirmed: true });
    } catch { setApprovalStatus(null); }
  }

  async function approveEndpoint(allow: boolean) {
    if (!allow) { update({ remoteConfirmed: false }); return; }
    if (!props.repo || endpointClass === 'local') { update({ remoteConfirmed: allow }); return; }
    try {
      await invoke('approve_runtime_endpoint', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, endpoint_url: props.settings.endpointUrl, allow_repo_context: true } });
      update({ remoteConfirmed: true });
      setMessage('Backend recorded non-local model server approval for repo context.');
      await refreshEndpointApprovalStatus();
    } catch (err) { update({ remoteConfirmed: false }); setMessage(String(err)); }
  }

  async function startManagedLlama() {
    try {
      const result = await invoke<ManagedStatus>('managed_llamacpp_start', { req: { binary_path: llamaBinary, model_path: llamaModel, port: Number(llamaPort), ctx_size: Number(llamaCtx) } });
      setManagedStatus(result);
      if (result.endpoint_url) update({ provider: 'managed-llamacpp', endpointUrl: result.endpoint_url, modelName: llamaModel.split(/[\\/]/).pop() || 'managed-gguf', remoteConfirmed: false });
      setMessage(result.message);
    } catch (err) { setMessage(String(err)); }
  }

  async function stopManagedLlama() {
    try { const result = await invoke<ManagedStatus>('managed_llamacpp_stop'); setManagedStatus(result); setMessage(result.message); }
    catch (err) { setMessage(String(err)); }
  }

  function applyPreset(preset: RuntimePresetView) {
    update({ provider: 'local-server', endpointUrl: preset.default_url, remoteConfirmed: false });
    setMessage(`${preset.label}: ${preset.notes}`);
  }

  function update(next: Partial<RuntimeSettings>) {
    const merged = { ...props.settings, ...next };
    props.onChange(merged);
    localStorage.setItem('synthesize.runtimeSettings.v2', JSON.stringify(merged));
    localStorage.setItem('synthesize.runtimeSettings.v1', JSON.stringify(merged));
  }

  useEffect(() => { void refresh(); }, []);
  useEffect(() => { void refreshEndpointApprovalStatus(); }, [props.repo?.repoRoot, props.settings.endpointUrl]);

  return (
    <div className="panel runtime-panel">
      <div className="panel-heading-row"><h3>Local Model Runtime Control</h3><button onClick={refresh}>Refresh</button></div>
      <div className="notice ok"><strong>Local-first model setup</strong><div className="small">Synthesize is intended for self-hosted open-source coding models. “OpenAI-compatible” below means local HTTP wire protocol, not OpenAI cloud.</div></div>

      <label className="small">Runtime mode</label>
      <select value={props.settings.provider} onChange={(e) => update({ provider: e.target.value as RuntimeSettings['provider'] })}>
        <option value="fake">Fake runtime</option>
        <option value="local-server">Local model server</option>
        <option value="managed-llamacpp">Managed llama.cpp</option>
      </select>
      <label className="small">Local server URL</label>
      <input value={props.settings.endpointUrl} onChange={(e) => update({ endpointUrl: e.target.value, remoteConfirmed: false })} placeholder="http://localhost:8080/v1" />
      <label className="small">Model name</label>
      <input value={props.settings.modelName} onChange={(e) => update({ modelName: e.target.value })} placeholder="qwen2.5-coder" />
      <div className="small">Protocol: {props.settings.provider === 'fake' ? 'in-process fixture' : props.settings.provider === 'managed-llamacpp' ? 'managed llama.cpp served over local HTTP' : 'OpenAI-compatible local HTTP'}</div>
      <div className="small">Endpoint classification: {endpointClass}</div>
      <div className="small">Backend repo-context approval: {endpointClass === 'local' ? 'not required for localhost' : approvalStatus?.approved ? `approved at ${approvalStatus.approved_at}` : 'not approved'}</div>
      {props.settings.provider !== 'fake' && endpointClass !== 'local' && (
        <div className="notice error"><strong>Non-local model server warning</strong><div className="small">This model server is not localhost. Repo context may leave this machine. Backend approval is required before sending context.</div><label className="small"><input type="checkbox" checked={props.settings.remoteConfirmed} onChange={(e) => void approveEndpoint(e.target.checked)} /> I explicitly allow sending repo context to this model server.</label></div>
      )}
      <div className="buttonrow"><button onClick={testEndpoint}>Health check through backend</button><button onClick={listEndpointModels}>List models if supported</button></div>
      <div className="small">Health: {health}</div>
      {endpointModels.length > 0 && <div className="small">Server models: {endpointModels.map((m) => m.id).join(', ')}</div>}
      <div className="small">Runtime status: {status?.active_runtime ?? 'unknown'} · model: {status?.loaded_model ?? 'none'}</div>

      <details open><summary>Local runtime presets</summary>{presets.map((preset) => <div className="model-card" key={preset.id}><strong>{preset.label}</strong><div className="small">{preset.default_url} · {preset.protocol}</div><div className="small">{preset.notes}</div><button onClick={() => applyPreset(preset)}>Use preset</button></div>)}</details>

      <details><summary>Managed llama.cpp / GGUF</summary><div className="small">Select an existing llama.cpp server binary and a local .gguf coding model. Synthesize starts it with argv-only process spawning bound to 127.0.0.1.</div><input value={llamaBinary} onChange={(e) => setLlamaBinary(e.target.value)} placeholder="/path/to/llama-server" /><input value={llamaModel} onChange={(e) => setLlamaModel(e.target.value)} placeholder="/models/qwen-coder.gguf" /><input value={llamaPort} onChange={(e) => setLlamaPort(e.target.value)} placeholder="8080" /><input value={llamaCtx} onChange={(e) => setLlamaCtx(e.target.value)} placeholder="8192" /><div className="buttonrow"><button onClick={startManagedLlama}>Start managed llama.cpp</button><button onClick={stopManagedLlama}>Stop managed llama.cpp</button></div><div className="small">Managed status: {managedStatus?.status ?? 'unknown'} · {managedStatus?.message ?? ''}</div>{managedStatus?.stderr_tail && <details><summary>llama.cpp stderr tail</summary><pre className="audit-json">{managedStatus.stderr_tail}</pre></details>}{managedStatus?.stdout_tail && <details><summary>llama.cpp stdout tail</summary><pre className="audit-json">{managedStatus.stdout_tail}</pre></details>}</details>

      <details><summary>Model Library: import local GGUF</summary><input value={modelDisplayName} onChange={(e) => setModelDisplayName(e.target.value)} placeholder="Display name" /><input value={localPath} onChange={(e) => setLocalPath(e.target.value)} placeholder="/models/qwen-coder.gguf" /><button className="primary" onClick={importModel}>Import GGUF metadata</button>{importedModels.map((model) => <div className="model-card" key={model.id}><strong>{model.display_name}</strong><div className="small">{model.format} · {model.runtime_compatibility} · {model.local_path}</div><button onClick={() => { setLlamaModel(model.local_path); update({ modelName: model.display_name }); }}>Use for managed llama.cpp</button></div>)}</details>

      <details><summary>Recommended local coding model classes</summary>{models.map((model) => <div className="model-card" key={model.id}><strong>{model.name}</strong><div className="small">{model.runtime} · {model.format} · approximate RAM {model.recommended_ram_gb}GB</div><div className="small">{model.notes}</div></div>)}</details>
      {message && <div className="small">{message}</div>}
      <div className="small">Model downloads are not automated in this build. Download GGUF models manually, import them here, or connect a self-hosted local model server.</div>
    </div>
  );
}
