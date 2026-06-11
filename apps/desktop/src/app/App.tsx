import { useEffect, useMemo, useRef, useState } from 'react';
import { classifyEndpoint } from '@synthesize/runtime-adapters';
import { invoke } from '@tauri-apps/api/core';
import Editor from '@monaco-editor/react';
import type { AgentOperation, ProposePatchOperation } from '@synthesize/shared-types';
import { RuntimeControl } from '../features/runtimeControl/RuntimeControl';
import { AgentPanel, type AgentProfileId } from '../features/agentProfiles/AgentPanel';
import { ChatPanel } from '../features/chat/ChatPanel';
import { DiffQueue } from '../features/diffQueue/DiffQueue';
import { CommandApproval } from '../features/commandApproval/CommandApproval';
import { ContextVisibility } from '../features/chat/ContextVisibility';
import { RepoExplorer } from '../features/repoExplorer/RepoExplorer';
import { SessionLog } from '../features/sessionReplay/SessionLog';
import { GitPanel } from '../features/gitPanel/GitPanel';
import { SearchPanel } from '../features/searchPanel/SearchPanel';
import { LspPanel } from '../features/lspPanel/LspPanel';
import { TaskRunner } from '../features/taskRunner/TaskRunner';
import { PersonalTerminal } from '../features/taskRunner/PersonalTerminal';
import { CommandPalette, type PaletteCommand } from '../features/commandPalette/CommandPalette';
import { QuickOpen } from '../features/quickOpen/QuickOpen';
import { SettingsPanel, type EditorSettings } from '../features/settingsPanel/SettingsPanel';
import { ProblemsPanel } from '../features/problemsPanel/ProblemsPanel';
import { ReadinessPanel } from '../features/readinessPanel/ReadinessPanel';

const sampleCode = `export function refreshToken() {\n  throw new Error("not implemented");\n}\n`;
export const SESSION_ID = 'synthesize-session';

export type RepoFileView = { path: string; kind: string; denied: boolean };

export type RepoState = {
  repoRoot: string;
  currentFilePath: string;
  currentFileContent: string;
  currentCommit?: string | null;
  files: RepoFileView[];
};

export type ValidationResult = {
  ok: boolean;
  proposal_id: string;
  operation_sha256: string;
  status: string;
  files: Array<{ id: string; path: string; risk: string; real_path: string }>;
  warnings: string[];
  errors: string[];
  message: string;
  audit_event_id?: string | null;
};

export type ApprovalResult = {
  proposal_id: string;
  approval_id: string;
  operation_sha256: string;
  approved_by_source: string;
  approved_at: string;
  audit_event_id: string;
};

export type ApplyResult = {
  proposal_id: string;
  approval_id: string;
  checkpoint_id: string;
  checkpoint_dir: string;
  applied_files: Array<{ id: string; path: string; after_sha256: string }>;
  audit_event_id: string;
};

export type RollbackResult = { proposal_id: string; checkpoint_id: string; checkpoint_dir: string; restored_paths: string[]; deleted_paths: string[]; audit_event_id: string };

export type OperationEvent = {
  id: string;
  label: string;
  detail: string;
  status: 'info' | 'ok' | 'error';
};

export type AuditEvent = { id: string; timestamp: string; kind: string; payload_json: string };

export type RuntimeSettings = {
  provider: 'fake' | 'local-server' | 'managed-llamacpp';
  endpointUrl: string;
  modelName: string;
  remoteConfirmed: boolean;
};

export type ContextBundleView = {
  context_bundle_id: string;
  session_id: string;
  repo_root: string;
  user_message: string;
  selected_file_path: string;
  dirty_buffer_state: boolean;
  git_commit?: string | null;
  endpoint_classification: 'local' | 'private-lan' | 'remote' | string;
  destination_warning: string;
  char_estimate: number;
  included: Array<{ kind: string; path?: string | null; chars: number; note: string }>;
  messages: Array<{ role: string; content: string }>;
  exact_prompt: string;
  messages_sha256: string;
  exact_context: boolean;
  agent_profile_id: AgentProfileId;
};

export type ProposalSource = { contextBundleId: string; agentProfileId: AgentProfileId };

export type ProposalUiState<T> = Record<string, T | undefined>;

export type EditorTab = { path: string; content: string; diskContent: string; dirty: boolean };

export type FileMutationResult = { path: string; message: string; audit_event_id?: string | null };


const defaultRuntimeSettings: RuntimeSettings = {
  provider: 'fake',
  endpointUrl: 'http://localhost:8080/v1',
  modelName: 'local-coder',
  remoteConfirmed: false
};

const defaultEditorSettings: EditorSettings = { wordWrap: 'off', fontSize: 14, minimap: false, theme: 'vs-dark' };

function loadEditorSettings(): EditorSettings {
  try {
    const raw = localStorage.getItem('synthesize.editorSettings.v1');
    return raw ? { ...defaultEditorSettings, ...JSON.parse(raw) } : defaultEditorSettings;
  } catch { return defaultEditorSettings; }
}


function loadRuntimeSettings(): RuntimeSettings {
  try {
    const raw = localStorage.getItem('synthesize.runtimeSettings.v2') ?? localStorage.getItem('synthesize.runtimeSettings.v1');
    if (!raw) return defaultRuntimeSettings;
    const parsed = { ...defaultRuntimeSettings, ...JSON.parse(raw) } as Record<string, unknown>;
    const providerRaw = parsed.provider === 'openai-compatible' ? 'local-server' : parsed.provider;
    const provider: RuntimeSettings['provider'] = (
      providerRaw === 'fake' || providerRaw === 'local-server' || providerRaw === 'managed-llamacpp'
    ) ? providerRaw : 'local-server';
    return {
      provider,
      endpointUrl: typeof parsed.endpointUrl === 'string' ? parsed.endpointUrl : defaultRuntimeSettings.endpointUrl,
      modelName: typeof parsed.modelName === 'string' ? parsed.modelName : defaultRuntimeSettings.modelName,
      remoteConfirmed: Boolean(parsed.remoteConfirmed)
    };
  } catch {
    return defaultRuntimeSettings;
  }
}

export function App() {
  const [content, setContent] = useState(sampleCode);
  const [repo, setRepo] = useState<RepoState | null>(null);
  const [repoPathInput, setRepoPathInput] = useState('');
  const [operations, setOperations] = useState<AgentOperation[]>([]);
  const [events, setEvents] = useState<OperationEvent[]>([]);
  const [validationByProposal, setValidationByProposal] = useState<ProposalUiState<ValidationResult>>({});
  const [approvalByProposal, setApprovalByProposal] = useState<ProposalUiState<ApprovalResult>>({});
  const [applyByProposal, setApplyByProposal] = useState<ProposalUiState<ApplyResult>>({});
  const [rollbackByProposal, setRollbackByProposal] = useState<ProposalUiState<RollbackResult>>({});
  const [auditEvents, setAuditEvents] = useState<AuditEvent[]>([]);
  const [isDirty, setIsDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [runtimeSettings, setRuntimeSettings] = useState<RuntimeSettings>(() => loadRuntimeSettings());
  const [runtimeRequestAttempts, setRuntimeRequestAttempts] = useState(0);
  const [lastContextBundle, setLastContextBundle] = useState<ContextBundleView | null>(null);
  const [agentProfileId, setAgentProfileId] = useState<AgentProfileId>(() => (localStorage.getItem('synthesize.agentProfileId.v1') as AgentProfileId) || 'local-patcher');
  const [proposalSourceByProposal, setProposalSourceByProposal] = useState<Record<string, ProposalSource | undefined>>({});
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [showQuickOpen, setShowQuickOpen] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [editorSettings, setEditorSettings] = useState<EditorSettings>(() => loadEditorSettings());
  const [agentDraft, setAgentDraft] = useState('');
  const [selectionSummary, setSelectionSummary] = useState('No selection');
  const [selectedText, setSelectedText] = useState('');
  const editorRef = useRef<any>(null);

  const patchOperations = useMemo(() => operations.filter((op): op is ProposePatchOperation => op.type === 'propose_patch'), [operations]);
  const endpointClass = classifyEndpoint(runtimeSettings.endpointUrl);

  useEffect(() => { localStorage.setItem('synthesize.runtimeSettings.v2', JSON.stringify(runtimeSettings)); }, [runtimeSettings]);
  useEffect(() => { localStorage.setItem('synthesize.editorSettings.v1', JSON.stringify(editorSettings)); }, [editorSettings]);

  function addEvent(label: string, detail: string, status: OperationEvent['status'] = 'info') {
    setEvents((prev) => [...prev, { id: crypto.randomUUID(), label, detail, status }]);
  }



  function getEditorSelectionText(): string {
    const editor = editorRef.current;
    const model = editor?.getModel?.();
    const selection = editor?.getSelection?.();
    if (!model || !selection || selection.isEmpty?.()) return '';
    return model.getValueInRange(selection);
  }

  function draftAgentPrompt(kind: 'explain' | 'fix' | 'tests' | 'review') {
    if (!repo) return;
    const selected = getEditorSelectionText();
    const selectionBlock = selected ? `\n\nSelected code from ${repo.currentFilePath}:\n\`\`\`\n${selected.slice(0, 8000)}\n\`\`\`` : `\n\nNo text is selected. Use the active file ${repo.currentFilePath}.`;
    const prompts: Record<'explain' | 'fix' | 'tests' | 'review', string> = {
      explain: `Explain the selected code and identify any risks or edge cases.${selectionBlock}`,
      fix: `Inspect the selected code and propose a small, reviewable patch if you find a concrete issue. Return Synthesize typed operations only.${selectionBlock}`,
      tests: `Write or update tests for the selected code. Keep the patch small and include a clear summary.${selectionBlock}`,
      review: `Review the current file/selection for bugs, maintainability issues, and missing tests. Return a report or a small patch proposal.${selectionBlock}`
    };
    setAgentDraft(prompts[kind]);
    addEvent('agent.draft_from_selection', `${kind} prompt drafted for ${repo.currentFilePath}`, 'info');
  }

  async function refreshAudit(targetRepo = repo) {
    if (!targetRepo) return;
    const result = await invoke<AuditEvent[]>('list_audit_events', { repoRoot: targetRepo.repoRoot, sessionId: SESSION_ID });
    setAuditEvents(result);
  }

  async function clearLocalSessionData() {
    if (!repo) return;
    setError(null);
    try {
      await invoke('clear_local_session_data', {
        req: { session_id: SESSION_ID, repo_root: repo.repoRoot, clear_endpoint_approvals: false }
      });
      setAuditEvents([]);
      setLastContextBundle(null);
      addEvent('session.local_data_cleared', 'Cleared persisted context/runtime/audit data; patch lifecycle records were preserved.', 'ok');
    } catch (err) {
      setError(String(err));
    }
  }

  async function recordSessionEvent(kind: string, payload: Record<string, unknown>) {
    if (!repo) return;
    try {
      await invoke('record_session_event', {
        req: { session_id: SESSION_ID, repo_root: repo.repoRoot, kind, payload_json: JSON.stringify(payload) }
      });
      await refreshAudit();
    } catch {
      // Session event logging should not block the UI path. Backend patch lifecycle audit remains authoritative.
    }
  }

  async function hydrateRepo(result: RepoState) {
    setRepo(result);
    setRepoPathInput(result.repoRoot);
    setContent(result.currentFileContent);
    setTabs([{ path: result.currentFilePath, content: result.currentFileContent, diskContent: result.currentFileContent, dirty: false }]);
    localStorage.setItem('synthesize.recentRepo.latest', result.repoRoot);
    setOperations([]);
    setValidationByProposal({});
    setApprovalByProposal({});
    setApplyByProposal({});
    setRollbackByProposal({});
    setLastContextBundle(null);
    setProposalSourceByProposal({});
    setIsDirty(false);
    setEvents([{ id: crypto.randomUUID(), label: 'repo.opened', detail: result.repoRoot, status: 'ok' }]);
    await refreshAudit(result);
  }

  async function openFixtureRepo() {
    setError(null);
    const result = await invoke<RepoState>('open_repo_mock');
    await hydrateRepo(result);
  }

  async function openRepoPath() {
    setError(null);
    try {
      const result = await invoke<RepoState>('open_repo_path', { repoRoot: repoPathInput });
      await hydrateRepo(result);
    } catch (err) {
      setError(String(err));
    }
  }

  async function openFile(path: string) {
    if (!repo) return;
    setError(null);
    try {
      const text = await invoke<string>('read_guarded_file', { repoRoot: repo.repoRoot, relativePath: path });
      const updated = { ...repo, currentFilePath: path, currentFileContent: text };
      setRepo(updated);
      setContent(text);
      setTabs((prev) => {
        const existing = prev.find((tab) => tab.path === path);
        if (existing) return prev.map((tab) => tab.path === path ? { ...tab, content: text, diskContent: text, dirty: false } : tab);
        return [...prev, { path, content: text, diskContent: text, dirty: false }].slice(-12);
      });
      localStorage.setItem('synthesize.recentFile.latest', path);
      setIsDirty(false);
      addEvent('file.opened', path, 'ok');
    } catch (err) {
      setError(String(err));
      addEvent('file.rejected', String(err), 'error');
    }
  }

  async function validatePatch(operation: ProposePatchOperation) {
    if (!repo) return;
    const source = proposalSourceByProposal[operation.proposalId];
    if (!source?.contextBundleId) {
      const message = 'Cannot validate this proposal because it is not bound to a persisted context bundle. Ask the local agent again so Synthesize can bind the proposal to backend-owned context.';
      setError(message);
      addEvent('patch.validation_blocked', message, 'error');
      return;
    }
    const result = await invoke<ValidationResult>('validate_patch_proposal', {
      req: {
        session_id: SESSION_ID,
        repo_root: repo.repoRoot,
        operation,
        agent_profile_id: source.agentProfileId,
        context_bundle_id: source.contextBundleId
      }
    });
    setValidationByProposal((prev) => ({ ...prev, [operation.proposalId]: result }));
    setApprovalByProposal((prev) => ({ ...prev, [operation.proposalId]: undefined }));
    setApplyByProposal((prev) => ({ ...prev, [operation.proposalId]: undefined }));
    addEvent(result.ok ? 'patch.validated' : 'patch.rejected', result.message, result.ok ? 'ok' : 'error');
    await refreshAudit();
  }

  async function approvePatch(operation: ProposePatchOperation) {
    const validation = validationByProposal[operation.proposalId];
    if (!repo || !validation || validation.proposal_id !== operation.proposalId || !validation.ok) return;
    const result = await invoke<ApprovalResult>('approve_patch_proposal', {
      req: {
        session_id: SESSION_ID,
        repo_root: repo.repoRoot,
        proposal_id: operation.proposalId,
        operation_sha256: validation.operation_sha256
      }
    });
    setApprovalByProposal((prev) => ({ ...prev, [operation.proposalId]: result }));
    addEvent('patch.approved', `approval: ${result.approval_id}`, 'ok');
    await refreshAudit();
  }

  async function applyPatch(operation: ProposePatchOperation) {
    const approval = approvalByProposal[operation.proposalId];
    if (!repo || !approval || approval.proposal_id !== operation.proposalId) return;
    const result = await invoke<ApplyResult>('apply_approved_patch', {
      req: {
        session_id: SESSION_ID,
        repo_root: repo.repoRoot,
        proposal_id: operation.proposalId,
        approval_id: approval.approval_id
      }
    });
    setApplyByProposal((prev) => ({ ...prev, [operation.proposalId]: result }));
    setRollbackByProposal((prev) => ({ ...prev, [operation.proposalId]: undefined }));
    addEvent('patch.applied', `checkpoint: ${result.checkpoint_id}`, 'ok');
    const refreshed = await invoke<string>('read_guarded_file', { repoRoot: repo.repoRoot, relativePath: repo.currentFilePath });
    setContent(refreshed);
    setRepo((prev) => prev ? { ...prev, currentFileContent: refreshed } : prev);
    setTabs((prev) => prev.map((tab) => repo && tab.path === repo.currentFilePath ? { ...tab, content: refreshed, diskContent: refreshed, dirty: false } : tab));
    setIsDirty(false);
    await refreshAudit();
  }

  async function rollbackPatch(operation: ProposePatchOperation) {
    if (!repo || !applyByProposal[operation.proposalId]) return;
    const result = await invoke<RollbackResult>('rollback_patch', {
      req: { session_id: SESSION_ID, repo_root: repo.repoRoot, proposal_id: operation.proposalId }
    });
    setRollbackByProposal((prev) => ({ ...prev, [operation.proposalId]: result }));
    addEvent('patch.rolled_back', result.restored_paths.join(', '), 'ok');
    const refreshed = await invoke<string>('read_guarded_file', { repoRoot: repo.repoRoot, relativePath: repo.currentFilePath });
    setContent(refreshed);
    setRepo((prev) => prev ? { ...prev, currentFileContent: refreshed } : prev);
    setTabs((prev) => prev.map((tab) => repo && tab.path === repo.currentFilePath ? { ...tab, content: refreshed, diskContent: refreshed, dirty: false } : tab));
    setIsDirty(false);
    await refreshAudit();
  }


  async function refreshRepoFiles() {
    if (!repo) return;
    const files = await invoke<RepoFileView[]>('list_repo_files', { repoRoot: repo.repoRoot });
    setRepo((prev) => prev ? { ...prev, files } : prev);
  }

  async function saveCurrentFile() {
    if (!repo) return;
    setError(null);
    try {
      await invoke<FileMutationResult>('write_guarded_file', {
        req: { session_id: SESSION_ID, repo_root: repo.repoRoot, relative_path: repo.currentFilePath, content }
      });
      setRepo((prev) => prev ? { ...prev, currentFileContent: content } : prev);
      setTabs((prev) => prev.map((tab) => tab.path === repo.currentFilePath ? { ...tab, content, diskContent: content, dirty: false } : tab));
      setIsDirty(false);
      addEvent('file.saved', repo.currentFilePath, 'ok');
      await refreshAudit();
    } catch (err) { setError(String(err)); }
  }

  async function saveAllFiles() {
    if (!repo) return;
    for (const tab of tabs.filter((t) => t.dirty)) {
      await invoke<FileMutationResult>('write_guarded_file', {
        req: { session_id: SESSION_ID, repo_root: repo.repoRoot, relative_path: tab.path, content: tab.content }
      });
    }
    if (tabs.some((t) => t.dirty)) addEvent('file.save_all', `Saved ${tabs.filter((t) => t.dirty).length} dirty tab(s).`, 'ok');
    setTabs((prev) => prev.map((tab) => ({ ...tab, diskContent: tab.content, dirty: false })));
    setRepo((prev) => prev ? { ...prev, currentFileContent: content } : prev);
    setIsDirty(false);
    await refreshAudit();
  }

  async function refreshCurrentFileFromDisk() {
    if (!repo) return;
    const text = await invoke<string>('read_guarded_file', { repoRoot: repo.repoRoot, relativePath: repo.currentFilePath });
    setContent(text);
    setRepo((prev) => prev ? { ...prev, currentFileContent: text } : prev);
    setTabs((prev) => prev.map((tab) => tab.path === repo.currentFilePath ? { ...tab, content: text, diskContent: text, dirty: false } : tab));
    setIsDirty(false);
    addEvent('file.refreshed', repo.currentFilePath, 'ok');
  }

  function closeTab(path: string) {
    const tab = tabs.find((t) => t.path === path);
    if (tab?.dirty && !window.confirm(`${path} has unsaved changes. Close anyway?`)) return;
    const remaining = tabs.filter((t) => t.path !== path);
    setTabs(remaining);
    if (repo?.currentFilePath === path && remaining[0]) void openFile(remaining[remaining.length - 1].path);
  }

  async function createFileFromPrompt() {
    if (!repo) return;
    const relativePath = window.prompt('Create file path relative to repo root');
    if (!relativePath) return;
    await invoke<FileMutationResult>('create_repo_file', { req: { session_id: SESSION_ID, repo_root: repo.repoRoot, relative_path: relativePath, content: '' } });
    await refreshRepoFiles();
    await openFile(relativePath);
  }

  async function renameCurrentFileFromPrompt() {
    if (!repo) return;
    const toPath = window.prompt('Rename current file to', repo.currentFilePath);
    if (!toPath || toPath === repo.currentFilePath) return;
    await invoke<FileMutationResult>('rename_repo_path', { req: { session_id: SESSION_ID, repo_root: repo.repoRoot, from_path: repo.currentFilePath, to_path: toPath } });
    await refreshRepoFiles();
    setTabs((prev) => prev.filter((tab) => tab.path !== repo.currentFilePath));
    await openFile(toPath);
  }

  async function deleteCurrentFileWithConfirm() {
    if (!repo) return;
    if (!window.confirm(`Delete ${repo.currentFilePath}? This is user-initiated and audited, but not undoable through patch rollback.`)) return;
    await invoke<FileMutationResult>('delete_repo_path', { req: { session_id: SESSION_ID, repo_root: repo.repoRoot, relative_path: repo.currentFilePath, allow_directory: false, confirmation_token: null } });
    await refreshRepoFiles();
    setTabs((prev) => prev.filter((tab) => tab.path !== repo.currentFilePath));
    const next = repo.files.find((f) => f.kind === 'file' && !f.denied && f.path !== repo.currentFilePath)?.path;
    if (next) await openFile(next);
  }

  const paletteCommands: PaletteCommand[] = [
    { id: 'quick-open', label: 'Quick Open File', description: 'Find and open a repo file.', run: () => setShowQuickOpen(true), disabled: !repo },
    { id: 'save', label: 'Save Current File', description: 'Write active tab through RepoGuard.', run: saveCurrentFile, disabled: !repo || !isDirty },
    { id: 'save-all', label: 'Save All', description: 'Save all dirty tabs through RepoGuard.', run: saveAllFiles, disabled: !repo || !tabs.some((t) => t.dirty) },
    { id: 'refresh', label: 'Refresh From Disk', description: 'Reload active file through guarded read.', run: refreshCurrentFileFromDisk, disabled: !repo },
    { id: 'new-file', label: 'New File', description: 'Create a file through RepoGuard.', run: createFileFromPrompt, disabled: !repo },
    { id: 'rename-file', label: 'Rename Current File', description: 'Rename active file through RepoGuard.', run: renameCurrentFileFromPrompt, disabled: !repo },
    { id: 'delete-file', label: 'Delete Current File', description: 'Delete active file through RepoGuard after confirmation.', run: deleteCurrentFileWithConfirm, disabled: !repo },
    { id: 'agent-explain-selection', label: 'Agent: Explain Selection', description: 'Draft an agent prompt about the selected code.', run: () => draftAgentPrompt('explain'), disabled: !repo },
    { id: 'agent-fix-selection', label: 'Agent: Fix Selection', description: 'Draft a patch-oriented agent prompt for the selected code.', run: () => draftAgentPrompt('fix'), disabled: !repo },
    { id: 'agent-tests-selection', label: 'Agent: Write Tests for Selection', description: 'Draft a test-writing prompt for the selected code.', run: () => draftAgentPrompt('tests'), disabled: !repo },
    { id: 'settings', label: 'Open Settings', description: 'Editor theme, font, minimap, and wrapping.', run: () => setShowSettings(true) }
  ];

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const mod = event.metaKey || event.ctrlKey;
      if (mod && event.key.toLowerCase() === 'p') { event.preventDefault(); setShowQuickOpen(true); }
      if (mod && event.shiftKey && event.key.toLowerCase() === 'p') { event.preventDefault(); setShowCommandPalette(true); }
      if (mod && event.key.toLowerCase() === 's') { event.preventDefault(); void saveCurrentFile(); }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [repo, content, isDirty, tabs]);

  useEffect(() => {
    void openFixtureRepo();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="app">
      <header className="topbar">
        <strong>Synthesize IDE</strong>
        <button className="primary" onClick={openFixtureRepo}>Open fixture repo</button>
        <input className="repo-input" placeholder="/path/to/your/repo" value={repoPathInput} onChange={(e) => setRepoPathInput(e.target.value)} />
        <button className="primary" onClick={openRepoPath}>Open local repo path</button>
        <button onClick={() => setShowQuickOpen(true)} disabled={!repo}>Quick Open</button>
        <button onClick={() => setShowCommandPalette(true)}>Command Palette</button>
        <button onClick={saveCurrentFile} disabled={!repo || !isDirty}>Save</button>
        <button onClick={() => setShowSettings(true)}>Settings</button>
        <span className="badge">File: {repo?.currentFilePath ?? 'none'}{isDirty ? ' · dirty' : ''}</span>
      </header>
      {error && <div className="global-error">{error}</div>}
      <main className="layout">
        <aside className="sidebar">
          <RuntimeControl settings={runtimeSettings} onChange={setRuntimeSettings} repo={repo} />
          <AgentPanel selected={agentProfileId} onChange={(id) => { setAgentProfileId(id); localStorage.setItem('synthesize.agentProfileId.v1', id); }} />
          <ReadinessPanel repo={repo} runtimeSettings={runtimeSettings} agentProfileId={agentProfileId} isDirty={isDirty} />
          <RepoExplorer repo={repo} onOpenFile={openFile} onCreateFile={createFileFromPrompt} onRenameCurrent={renameCurrentFileFromPrompt} onDeleteCurrent={deleteCurrentFileWithConfirm} />
          <SearchPanel repo={repo} onOpenFile={openFile} />
          <GitPanel repo={repo} onOpenFile={openFile} />
          <LspPanel repo={repo} />
        </aside>
        <section className="editor">
          <div className="editor-toolbar">
            <span>{repo?.repoRoot ?? 'no repo'}</span>
            <span className={isDirty ? 'dirty' : 'clean'}>{isDirty ? 'Dirty buffer: planning disabled' : 'Clean buffer: patch target is disk file'}</span>
            <span className="small">{selectionSummary}</span>
            <button onClick={() => draftAgentPrompt('explain')} disabled={!repo}>Explain selection</button>
            <button onClick={() => draftAgentPrompt('fix')} disabled={!repo}>Fix selection</button>
            <button onClick={() => draftAgentPrompt('tests')} disabled={!repo}>Write tests</button>
          </div>
          <div className="tabbar">
            {tabs.map((tab) => <button key={tab.path} className={repo?.currentFilePath === tab.path ? 'tab active' : 'tab'} onClick={() => openFile(tab.path)} title={tab.path}>{tab.dirty ? '● ' : ''}{tab.path}<span onClick={(e) => { e.stopPropagation(); closeTab(tab.path); }}> ×</span></button>)}
          </div>
          <Editor
            height="100%"
            defaultLanguage="typescript"
            theme={editorSettings.theme}
            value={content}
            onMount={(editor) => {
              editorRef.current = editor;
              editor.onDidChangeCursorSelection?.(() => {
                const text = getEditorSelectionText();
                setSelectedText(text);
                setSelectionSummary(text ? `Selection: ${text.length} chars` : 'No selection');
              });
            }}
            onChange={(v) => {
              const next = v ?? '';
              setContent(next);
              const dirty = next !== (repo?.currentFileContent ?? '');
              setIsDirty(dirty);
              if (repo) setTabs((prev) => prev.map((tab) => tab.path === repo.currentFilePath ? { ...tab, content: next, dirty } : tab));
            }}
            options={{ minimap: { enabled: editorSettings.minimap }, fontSize: editorSettings.fontSize, wordWrap: editorSettings.wordWrap }}
          />
          <CommandApproval operation={patchOperations[0]?.suggestedCommands?.[0]} repo={repo} />
          <PersonalTerminal repo={repo} onRefreshAudit={() => refreshAudit()} onSendOutputToAgent={(prompt) => { setAgentDraft(prompt); addEvent('agent.draft_from_terminal_output', 'Terminal output queued for local agent repair loop.', 'info'); }} />
          <TaskRunner repo={repo} onRefreshAudit={() => refreshAudit()} onSendOutputToAgent={(prompt) => { setAgentDraft(prompt); addEvent('agent.draft_from_task_output', 'Task output queued for local agent repair loop.', 'info'); }} />
          <ProblemsPanel repo={repo} content={content} />
        </section>
        <aside className="rightbar">
          <ChatPanel
            repo={repo}
            currentContent={content}
            selectedText={selectedText}
            isDirty={isDirty}
            runtimeSettings={runtimeSettings}
            agentProfileId={agentProfileId}
            onContextBuilt={(bundle) => { setLastContextBundle(bundle); addEvent('context.bundle_created', `${bundle.context_bundle_id} · ${bundle.char_estimate} chars · ${bundle.endpoint_classification}`, 'ok'); }}
            onModelCall={() => { setRuntimeRequestAttempts((prev) => prev + 1); }}
            onParseFailed={(parseError, raw) => { addEvent('operation.parse_failed', parseError, 'error'); void recordSessionEvent('operation.parse_failed', { error: parseError, raw_chars: raw.length }); }}
            onOperations={(ops, source) => {
              setOperations(ops);
              const sources: Record<string, ProposalSource> = {};
              for (const op of ops) {
                if (op.type === 'propose_patch') {
                  sources[op.proposalId] = source;
                }
              }
              setProposalSourceByProposal(sources);
              setValidationByProposal({});
              setApprovalByProposal({});
              setApplyByProposal({});
              setRollbackByProposal({});
              addEvent('operations.parsed', `${ops.length} typed operation(s) parsed from ${runtimeSettings.provider} using ${source.agentProfileId}`, 'ok');
              void recordSessionEvent('operation.parse_succeeded', { count: ops.length, runtime: runtimeSettings.provider, context_bundle_id: source.contextBundleId, agent_profile_id: source.agentProfileId });
            }}
            events={events}
            draft={agentDraft}
            onDraftConsumed={() => setAgentDraft('')}
          />
          <ContextVisibility repo={repo} currentContent={content} isDirty={isDirty} runtimeSettings={runtimeSettings} contextBundle={lastContextBundle} />
          <DiffQueue
            patches={patchOperations}
            validationByProposal={validationByProposal}
            approvalByProposal={approvalByProposal}
            applyByProposal={applyByProposal}
            rollbackByProposal={rollbackByProposal}
            isDirty={isDirty}
            agentProfileId={agentProfileId}
            proposalSourceByProposal={proposalSourceByProposal}
            onValidate={validatePatch}
            onApprove={approvePatch}
            onApply={applyPatch}
            onRollback={rollbackPatch}
          />
          <SessionLog events={auditEvents} onRefresh={() => refreshAudit()} onClearLocalData={clearLocalSessionData} />
        </aside>
      </main>
      <CommandPalette open={showCommandPalette} commands={paletteCommands} onClose={() => setShowCommandPalette(false)} />
      <QuickOpen open={showQuickOpen} repo={repo} onOpenFile={openFile} onClose={() => setShowQuickOpen(false)} />
      <SettingsPanel open={showSettings} settings={editorSettings} onChange={setEditorSettings} onClose={() => setShowSettings(false)} />
      <footer className="footer">
        <span>Mode: Local-first workbench</span>
        <span>Runtime: {runtimeSettings.provider === 'fake' ? 'Fake Runtime' : runtimeSettings.provider === 'managed-llamacpp' ? 'Managed llama.cpp' : 'Local Model Server'}</span>
        <span>Endpoint: {runtimeSettings.provider === 'fake' ? 'in-memory' : endpointClass}{runtimeSettings.provider !== 'fake' && endpointClass !== 'local' ? ' · non-local warning' : ''}</span>
        <span>Runtime request attempts this session: {runtimeRequestAttempts} · prompts derived from persisted context bundles</span>
        <span>Code execution: governed personal terminal + detected tasks</span>
        <span>Repo boundary: enforced for read/validate/approve/apply/rollback</span>
        <span>Patch approval/rollback: backend-owned · Apply: checkpointed transaction-shaped · Task execution: approved/audited/timeout-bounded · Lock: in-process · Network sandbox: not OS-enforced</span>
      </footer>
    </div>
  );
}
