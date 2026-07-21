import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DeclarativePrototypeRenderer } from './DeclarativePrototypeRenderer';
import { filterTimeline, pulseSourceLabel, STUDIO_TABS, summarizeEvidence } from './studioModel.mjs';

type ProductMode = 'studio' | 'dream';
type RepoBinding = { repoRoot: string; currentCommit?: string | null };
type Initiative = {
  id: string; title: string; mode: string; status: string; activeSpecVersion: number;
  activeWorktreeId?: string | null; autonomyLevel: number; updatedAt: string;
};
type Snapshot = {
  initiative: Initiative;
  proof: Record<string, unknown> & { objective?: unknown[]; assumptions?: unknown[]; constraints?: unknown[]; requirements?: unknown[]; architectureDecisions?: unknown[]; tasks?: unknown[]; evidence?: unknown[] };
  dreams: Array<Record<string, unknown>>;
  artifacts: Array<Record<string, unknown>>;
  questions: Array<Record<string, unknown>>;
  agentRuns: Array<Record<string, unknown>>;
  contextCapsules: Array<Record<string, unknown>>;
  timeline: Array<Record<string, unknown>>;
  uxDocuments: Array<{ id: string; contract: Record<string, unknown>; prototype: Parameters<typeof DeclarativePrototypeRenderer>[0]['document'] }>;
  architecture: Array<Record<string, unknown>>;
  worktree?: { id: string; path: string; branch: string; baseCommit: string; status: string } | null;
};
type DreamInboxItem = { id: string; initiativeId: string; horizon: string; status: string; payload: Record<string, unknown>; createdAt: string };
type PulseFinding = { kind: string; severity: number; source: string; experimental: boolean; primaryFactors: string[]; recommendedIntervention: string; supportingEvents: string[] };
type PulseView = { symbolicFindings: PulseFinding[]; temporalFindings: PulseFinding[]; ruleObserver: Record<string, unknown>; liquidObserver: Record<string, unknown> };
type RoleProfile = { role: string; displayName: string; shortLabel: string; version: number; purpose: string[]; allowedArtifacts: string[]; forbiddenActions: string[] };
type RoleRuntime = {
  role: string; runtime: string; model: string; endpointUrl?: string | null; timeoutSeconds: number;
  contextWindowTokens: number; maximumOutputTokens: number; tokenEstimationMethod: string;
  safetyMarginTokens: number; structuredOutputBehavior: string; capabilitySource: string;
  lastValidatedAt?: string | null;
};

export function StudioWorkspace({ mode, repo, sessionId }: { mode: ProductMode; repo: RepoBinding | null; sessionId: string }) {
  const [initiatives, setInitiatives] = useState<Initiative[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [dreams, setDreams] = useState<DreamInboxItem[]>([]);
  const [pulse, setPulse] = useState<PulseView | null>(null);
  const [roleProfiles, setRoleProfiles] = useState<RoleProfile[]>([]);
  const [roleRuntimes, setRoleRuntimes] = useState<Record<string, RoleRuntime>>({});
  const [tab, setTab] = useState('overview');
  const [prompt, setPrompt] = useState('Build a governed, evidence-backed product improvement.');
  const [dreamFocus, setDreamFocus] = useState('Find one reversible improvement to Synthesize product-development flow.');
  const [timelineFilter, setTimelineFilter] = useState('');
  const [scenario, setScenario] = useState('successful_studio');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [continuousDream, setContinuousDream] = useState(false);
  const [mandateApproved, setMandateApproved] = useState(false);
  const dreamCycleInFlight = useRef(false);

  const requestBase = useMemo(() => ({ session_id: sessionId, repo_root: repo?.repoRoot ?? '' }), [repo?.repoRoot, sessionId]);

  const loadSnapshot = useCallback(async (initiativeId: string) => {
    if (!repo) return;
    const result = await invoke<Snapshot>('studio_get_snapshot', { req: { ...requestBase, initiative_id: initiativeId } });
    setSnapshot(result);
    setSelectedId(initiativeId);
  }, [repo, requestBase]);

  const refresh = useCallback(async () => {
    if (!repo) return;
    setLoading(true);
    setError(null);
    try {
      const [nextInitiatives, nextDreams, nextProfiles, nextRuntimes] = await Promise.all([
        invoke<Initiative[]>('studio_list_initiatives', { req: requestBase }),
        invoke<DreamInboxItem[]>('dream_list_inbox', { req: requestBase }),
        invoke<RoleProfile[]>('studio_role_profiles'),
        invoke<RoleRuntime[]>('studio_list_role_runtimes', { req: requestBase })
      ]);
      setInitiatives(nextInitiatives);
      setDreams(nextDreams);
      setRoleProfiles(nextProfiles);
      setRoleRuntimes((current) => Object.fromEntries(nextProfiles.map((profile) => {
        const persisted = nextRuntimes.find((runtime) => runtime.role === profile.role);
        return [profile.role, current[profile.role] ?? persisted ?? {
          role: profile.role, runtime: 'fake', model: 'studio-fixture-v1', endpointUrl: null,
          timeoutSeconds: 300, contextWindowTokens: 32768, maximumOutputTokens: 4096,
          tokenEstimationMethod: 'conservative_utf8_bytes_div3', safetyMarginTokens: 1024,
          structuredOutputBehavior: 'json_object', capabilitySource: 'local-user-declaration'
        }];
      })));
      const nextId = selectedId && nextInitiatives.some((item) => item.id === selectedId)
        ? selectedId
        : nextInitiatives.find((item) => mode === 'studio' ? item.mode === 'studio' : item.mode.startsWith('dream_'))?.id ?? null;
      if (nextId) await loadSnapshot(nextId);
      else setSnapshot(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setLoading(false);
    }
  }, [loadSnapshot, mode, repo, requestBase, selectedId]);

  useEffect(() => {
    setMandateApproved(false);
    setContinuousDream(false);
    void refresh();
  }, [mode, repo?.repoRoot]);
  useEffect(() => {
    if (mode !== 'dream' || !continuousDream || !mandateApproved || !repo) return;
    const timer = window.setInterval(() => { void startDreamCycle(); }, 60_000);
    return () => window.clearInterval(timer);
  }, [continuousDream, mandateApproved, mode, repo?.repoRoot, dreamFocus]);

  async function backendAction<T>(action: () => Promise<T>, success: string, useAsSnapshot = false) {
    setLoading(true);
    setError(null);
    try {
      const result = await action();
      if (useAsSnapshot) setSnapshot(result as Snapshot);
      setMessage(success);
      await refresh();
      return result;
    } catch (reason) {
      setError(String(reason));
      return null;
    } finally {
      setLoading(false);
    }
  }

  async function createStudio() {
    if (!repo || !prompt.trim()) return;
    const result = await backendAction(
      () => invoke<Snapshot>('studio_create_initiative', { req: { ...requestBase, prompt } }),
      'Studio discovery completed; frozen scope is ready for review.',
      true
    );
    const id = (result as Snapshot | null)?.initiative.id;
    if (id) setSelectedId(id);
  }

  async function startDreamCycle(approveMandate = false) {
    if (!repo || dreamCycleInFlight.current) return;
    if (!approveMandate && !mandateApproved) {
      setError('Approve the repository-bound standing mandate before starting Dream cycles.');
      return;
    }
    dreamCycleInFlight.current = true;
    const mandateId = `MANDATE-${stableRepoKey(repo.repoRoot)}`;
    const mandate = {
      id: mandateId,
      name: 'Local product exploration',
      purpose: 'Seek reversible improvements to governed local product development.',
      allowedModes: ['dream_ideation', 'dream_prototype', 'dream_incubator'],
      allowedRepoPaths: [], maximumCandidatesPerCycle: 10, maximumPrototypesPerCycle: 2,
      maximumBuilderIterations: 8, maximumChangedFiles: 20, maximumElapsedMinutes: 240,
      networkPolicy: 'disabled', packageInstallPolicy: 'forbidden',
      activeBranchWritePolicy: 'forbidden', mergeAuthority: 'human_only', enabled: true
    };
    try {
      await backendAction(async () => {
        if (approveMandate) {
          await invoke('dream_save_mandate', { req: { ...requestBase, mandate } });
          setMandateApproved(true);
        }
        return invoke('dream_start_cycle', { req: { ...requestBase, mandate_id: mandateId, focus: dreamFocus } });
      }, 'Bounded Dream ideation completed without repository changes.');
    } finally {
      dreamCycleInFlight.current = false;
    }
  }

  async function control(action: string) {
    if (!snapshot) return;
    await backendAction(
      () => invoke<Snapshot>('studio_control', { req: { ...requestBase, initiative_id: snapshot.initiative.id, action, reason: `Local user selected ${action}` } }),
      `Backend accepted ${action}.`,
      true
    );
  }

  async function approveScope() {
    if (!snapshot) return;
    await backendAction(
      () => invoke<Snapshot>('studio_approve_scope', { req: { ...requestBase, initiative_id: snapshot.initiative.id } }),
      'Frozen scope approved by local user.',
      true
    );
  }

  async function runFixture() {
    if (!snapshot) return;
    await backendAction(
      () => invoke<Snapshot>('studio_run_fake', { req: { ...requestBase, initiative_id: snapshot.initiative.id, scenario } }),
      `Deterministic ${scenario} role flow completed.`,
      true
    );
  }

  async function loadPulse() {
    if (!snapshot) return;
    await backendAction(async () => {
      const result = await invoke<PulseView>('studio_pulse', { req: { ...requestBase, initiative_id: snapshot.initiative.id } });
      setPulse(result);
      return result;
    }, 'Pulse evaluated authoritative events.');
  }

  async function createWorktree() {
    if (!snapshot || !repo?.currentCommit) return;
    await backendAction(
      () => invoke('governed_worktree_create', { req: { ...requestBase, initiative_id: snapshot.initiative.id, approved_base_commit: repo.currentCommit } }),
      'Backend created and bound an isolated Git worktree.'
    );
  }

  async function exportProof() {
    if (!snapshot) return;
    const result = await backendAction(
      () => invoke<string>('studio_export_proof', { req: { ...requestBase, initiative_id: snapshot.initiative.id } }),
      'Privacy-filtered proof report generated.'
    );
    if (typeof result === 'string') {
      const blob = new Blob([result], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `${snapshot.initiative.id}-proof.json`;
      anchor.click();
      URL.revokeObjectURL(url);
    }
  }

  function editRoleRuntime(role: string, changes: Partial<RoleRuntime>) {
    setRoleRuntimes((current) => ({ ...current, [role]: { ...current[role], role, ...changes } }));
  }

  async function saveRoleRuntime(role: string) {
    const runtime = roleRuntimes[role];
    if (!runtime) return;
    await backendAction(
      () => invoke('studio_save_role_runtime', { req: {
        ...requestBase, role, runtime: runtime.runtime, model: runtime.model,
        endpoint_url: runtime.endpointUrl || null, timeout_seconds: runtime.timeoutSeconds,
        context_window_tokens: runtime.contextWindowTokens, maximum_output_tokens: runtime.maximumOutputTokens,
        token_estimation_method: runtime.tokenEstimationMethod, safety_margin_tokens: runtime.safetyMarginTokens,
        structured_output_behavior: runtime.structuredOutputBehavior, capability_source: runtime.capabilitySource
      } }),
      `${role} runtime configuration saved. Endpoint approval remains separately enforced.`
    );
  }

  async function runRole(role: string) {
    if (!snapshot) return;
    const deliveryRole = ['builder', 'verifier', 'reviewer'].includes(role);
    const firstTask = (snapshot.proof.tasks as Array<Record<string, unknown>> | undefined)?.[0];
    await backendAction(
      () => invoke('studio_run_role', { req: {
        ...requestBase, initiative_id: snapshot.initiative.id,
        task_id: deliveryRole ? String(firstTask?.id ?? '') || null : null, role
      } }),
      `${role} returned a validated typed operation.`
    );
  }

  async function cancelRoleRun(runId: string) {
    await backendAction(
      () => invoke('studio_cancel_role_run', { req: { run_id: runId } }),
      `Cancellation requested for ${runId}.`
    );
  }

  if (!repo) return <main className="studio-workspace"><div className="studio-empty">Open a repository to use {mode === 'studio' ? 'Studio' : 'Dream'} Mode.</div></main>;

  const evidence = summarizeEvidence(snapshot?.proof);
  const timeline = filterTimeline(snapshot?.timeline ?? [], timelineFilter);
  return (
    <main className="studio-workspace" aria-busy={loading}>
      <header className="studio-commandbar">
        <div>
          <span className="studio-kicker">{mode === 'studio' ? 'Outcome-governed delivery' : 'Mandate-bound exploration'}</span>
          <h2>{mode === 'studio' ? 'Studio Workspace' : 'Dream Workspace'}</h2>
        </div>
        <div className="studio-runtime-state" aria-label="automation state">
          <span>Status: {snapshot?.initiative.status ?? 'idle'}</span>
          <span>Autonomy: {snapshot?.initiative.autonomyLevel ?? 0}</span>
          <span>Worktree: {snapshot?.worktree?.branch ?? 'none'}</span>
          <span>Spec: v{snapshot?.initiative.activeSpecVersion ?? '—'}</span>
        </div>
      </header>

      <section className="studio-launcher">
        {mode === 'studio' ? <>
          <textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} aria-label="Studio outcome prompt" />
          <button className="primary" onClick={createStudio} disabled={loading || !prompt.trim()}>Start Studio discovery</button>
        </> : <>
          <div className="dream-autoprompt"><strong>Background Dreamer brief</strong><span>Invent, challenge, and shape a new app idea for this repository. No user prompt is required.</span></div>
          <button className="primary" onClick={() => void startDreamCycle(true)} disabled={loading}>Approve mandate & run one bounded cycle</button>
          <label className="dream-continuous"><input type="checkbox" checked={continuousDream} onChange={(event) => { const enabled = event.target.checked; setContinuousDream(enabled); if (enabled) void startDreamCycle(true); }} />Repeat bounded cycles continuously while Synthesize is open</label>
          <span className="small">Checking this saves the mandate and starts the first cycle. Each completed idea starts the next bounded idea cycle. Stop by unchecking or closing Synthesize.</span>
        </>}
        <select value={selectedId ?? ''} onChange={(event) => void loadSnapshot(event.target.value)} aria-label="Initiative">
          <option value="">Select initiative</option>
          {initiatives.map((item) => <option key={item.id} value={item.id}>{item.title} · {item.status}</option>)}
        </select>
      </section>

      {error && <div className="studio-notice error" role="alert">{error}<button onClick={() => void refresh()}>Retry</button></div>}
      {message && <div className="studio-notice ok" role="status">{message}</div>}
      {loading && <div className="studio-loading" role="status">Working through the trusted backend…</div>}

      <nav className="studio-tabs" aria-label="Initiative workspace sections">
        {STUDIO_TABS.map((item) => <button key={item} className={tab === item ? 'active' : ''} aria-current={tab === item ? 'page' : undefined} onClick={() => setTab(item)}>{title(item)}</button>)}
      </nav>

      {!snapshot ? <div className="studio-empty">{mode === 'studio' ? 'Start or select an initiative.' : 'Run a bounded cycle or select a Dream initiative.'}</div> : (
        <section className="studio-content">
          {tab === 'overview' && <Overview snapshot={snapshot} evidence={evidence} />}
          {tab === 'intent' && <JsonSections sections={[
            ['Objectives', snapshot.proof.objective], ['Assumptions', snapshot.proof.assumptions],
            ['Constraints & non-goals', snapshot.proof.constraints], ['Requirements', snapshot.proof.requirements]
          ]} />}
          {tab === 'dreams' && <DreamInbox dreams={dreams} loading={loading} onAction={(dreamId, action) => backendAction(() => invoke('dream_action', { req: { ...requestBase, dream_id: dreamId, action } }), `Dream ${action} recorded.`)} />}
          {tab === 'ux' && <UxView snapshot={snapshot} />}
          {tab === 'architecture' && <JsonSections sections={[["Architecture alternatives and ADR", snapshot.architecture]]} />}
          {tab === 'plan' && <JsonSections sections={[["Dependency-aware tasks", snapshot.proof.tasks]]} />}
          {tab === 'team' && <TeamTimeline events={timeline} filter={timelineFilter} onFilter={setTimelineFilter} artifacts={snapshot.artifacts} questions={snapshot.questions} runs={snapshot.agentRuns} capsules={snapshot.contextCapsules} profiles={roleProfiles} runtimes={roleRuntimes} onRuntimeChange={editRoleRuntime} onRuntimeSave={saveRoleRuntime} onRoleRun={runRole} onCancelRun={cancelRoleRun} loading={loading} />}
          {tab === 'pulse' && <PulsePanel pulse={pulse} onEvaluate={loadPulse} loading={loading} />}
          {tab === 'evidence' && <EvidenceView proof={snapshot.proof} evidence={evidence} />}
          {tab === 'changes' && <ChangesView snapshot={snapshot} canCreateWorktree={Boolean(repo.currentCommit)} onCreateWorktree={createWorktree} onExport={exportProof} />}
        </section>
      )}

      {snapshot && <footer className="studio-controls" aria-label="Initiative controls">
        {snapshot.initiative.status === 'awaiting_scope_approval' && <button className="primary" onClick={approveScope}>Approve frozen scope</button>}
        {snapshot.initiative.status === 'implementing' && <><select value={scenario} onChange={(event) => setScenario(event.target.value)} aria-label="Fake Runtime scenario">
          <option value="successful_studio">Successful full flow</option><option value="reviewer_revision">Reviewer revision</option>
          <option value="replan">Replan</option><option value="blocking_question">Blocking question</option><option value="budget_stop">Budget stop</option>
          <option value="malformed_artifact">Malformed artifact rejection</option><option value="role_permission_violation">Role permission violation</option><option value="drift_signal">Pulse drift signal</option>
        </select><button onClick={runFixture}>Run deterministic role flow</button></>}
        {snapshot.initiative.status !== 'paused' && !['completed', 'abandoned', 'failed'].includes(snapshot.initiative.status) && <button onClick={() => control('pause')}>Pause</button>}
        {snapshot.initiative.status === 'paused' && <button onClick={() => control('resume')}>Resume</button>}
        {!['completed', 'abandoned', 'failed'].includes(snapshot.initiative.status) && <button onClick={() => control('stop')}>Stop</button>}
        <button onClick={() => control('lower_autonomy')}>Lower autonomy</button>
        <button onClick={() => control('request_alignment_review')}>Request alignment review</button>
        {snapshot.initiative.status === 'awaiting_merge_review' && <button className="primary" onClick={() => control('complete_review')}>Review final candidate</button>}
      </footer>}
    </main>
  );
}

function Overview({ snapshot, evidence }: { snapshot: Snapshot; evidence: ReturnType<typeof summarizeEvidence> }) {
  return <div className="studio-grid">
    <Metric label="Initiative" value={snapshot.initiative.title} /><Metric label="Phase" value={snapshot.initiative.status} />
    <Metric label="Evidence" value={`${evidence.complete} complete · ${evidence.incomplete} open`} /><Metric label="Worktree" value={snapshot.worktree?.branch ?? 'No isolated workspace'} />
    <Metric label="Active spec" value={`v${snapshot.initiative.activeSpecVersion}`} /><Metric label="Outcome" value={(snapshot.proof.outcomePending as unknown[] | undefined)?.length ? 'Pending measurement' : 'Not yet pending'} />
  </div>;
}

function Metric({ label, value }: { label: string; value: string }) { return <article className="studio-card"><span>{label}</span><strong>{value}</strong></article>; }

function JsonSections({ sections }: { sections: Array<[string, unknown]> }) {
  return <div className="studio-sections">{sections.map(([label, value]) => <section key={label}><h3>{label}</h3>{Array.isArray(value) && value.length === 0 ? <div className="studio-empty compact">No records yet.</div> : <pre>{JSON.stringify(value ?? [], null, 2)}</pre>}</section>)}</div>;
}

function UxView({ snapshot }: { snapshot: Snapshot }) {
  const ux = snapshot.uxDocuments[0];
  if (!ux) return <div className="studio-empty">No validated UX Contract is available.</div>;
  return <div className="studio-ux"><section><h3>UX Contract</h3><pre>{JSON.stringify(ux.contract, null, 2)}</pre></section><DeclarativePrototypeRenderer document={ux.prototype} /></div>;
}

function DreamInbox({ dreams, loading, onAction }: { dreams: DreamInboxItem[]; loading: boolean; onAction: (id: string, action: string) => unknown }) {
  if (!dreams.length) return <div className="studio-empty">Dream Inbox is empty. Candidates are options, never roadmap commitments.</div>;
  return <div className="dream-grid">{dreams.map((dream) => <article className="dream-card" key={dream.id}>
    <span className="studio-kicker">{dream.horizon} · {dream.status}</span><h3>{String(dream.payload.title ?? 'Untitled Dream')}</h3>
    <p>{String(dream.payload.problemObserved ?? '')}</p><strong>Proposed future</strong><p>{String(dream.payload.proposedFuture ?? '')}</p>
    <details><summary>Evidence, assumptions & counterarguments</summary><pre>{JSON.stringify({ supportingEvidence: dream.payload.supportingEvidence, assumptions: dream.payload.assumptions, counterarguments: dream.payload.counterarguments, smallestExperiment: dream.payload.smallestExperiment }, null, 2)}</pre></details>
    {dream.status === 'proposed' && <div className="buttonrow"><button disabled={loading} onClick={() => onAction(dream.id, 'reject')}>Reject</button><button disabled={loading} onClick={() => onAction(dream.id, 'archive')}>Archive</button><button disabled={loading} onClick={() => onAction(dream.id, 'promote')}>Promote to Studio</button><button disabled={loading} onClick={() => onAction(dream.id, 'approve_prototype')}>Approve prototype experiment</button></div>}
    {dream.status === 'prototype_approved' && <div className="buttonrow"><button disabled={loading} onClick={() => onAction(dream.id, 'enable_incubator')}>Enable bounded incubator</button><button disabled={loading} onClick={() => onAction(dream.id, 'promote')}>Promote to Studio</button></div>}
  </article>)}</div>;
}

function TeamTimeline({ events, filter, onFilter, artifacts, questions, runs, capsules, profiles, runtimes, onRuntimeChange, onRuntimeSave, onRoleRun, onCancelRun, loading }: { events: Record<string, unknown>[]; filter: string; onFilter: (value: string) => void; artifacts: Record<string, unknown>[]; questions: Record<string, unknown>[]; runs: Record<string, unknown>[]; capsules: Record<string, unknown>[]; profiles: RoleProfile[]; runtimes: Record<string, RoleRuntime>; onRuntimeChange: (role: string, changes: Partial<RoleRuntime>) => void; onRuntimeSave: (role: string) => unknown; onRoleRun: (role: string) => unknown; onCancelRun: (runId: string) => unknown; loading: boolean }) {
  return <div><section className="role-grid" aria-label="Studio role runtime configuration">{profiles.map((profile) => {
    const runtime = runtimes[profile.role];
    return <article className="role-card" key={profile.role}><div><span className="studio-kicker">{profile.shortLabel} · profile v{profile.version}</span><h3>{profile.displayName}</h3><p>{profile.purpose.join(' ')}</p></div>
      <label>Runtime<input value={runtime?.runtime ?? ''} onChange={(event) => onRuntimeChange(profile.role, { runtime: event.target.value })} /></label>
      <label>Model<input value={runtime?.model ?? ''} onChange={(event) => onRuntimeChange(profile.role, { model: event.target.value })} /></label>
      <label>Endpoint URL<input value={runtime?.endpointUrl ?? ''} placeholder="Local or approved remote endpoint" onChange={(event) => onRuntimeChange(profile.role, { endpointUrl: event.target.value })} /></label>
      <label>Timeout seconds<input type="number" min={1} max={1800} value={runtime?.timeoutSeconds ?? 300} onChange={(event) => onRuntimeChange(profile.role, { timeoutSeconds: Number(event.target.value) })} /></label>
      <div className="role-capability-grid">
        <label>Context window tokens<input type="number" min={512} value={runtime?.contextWindowTokens ?? 32768} onChange={(event) => onRuntimeChange(profile.role, { contextWindowTokens: Number(event.target.value) })} /></label>
        <label>Maximum output tokens<input type="number" min={1} value={runtime?.maximumOutputTokens ?? 4096} onChange={(event) => onRuntimeChange(profile.role, { maximumOutputTokens: Number(event.target.value) })} /></label>
        <label>Safety margin tokens<input type="number" min={1} value={runtime?.safetyMarginTokens ?? 1024} onChange={(event) => onRuntimeChange(profile.role, { safetyMarginTokens: Number(event.target.value) })} /></label>
        <label>Token method<select value={runtime?.tokenEstimationMethod ?? 'conservative_utf8_bytes_div3'} onChange={(event) => onRuntimeChange(profile.role, { tokenEstimationMethod: event.target.value })}><option value="conservative_utf8_bytes_div3">Conservative estimate</option><option value="runtime_tokenizer">Runtime tokenizer</option></select></label>
        <label>Structured output<select value={runtime?.structuredOutputBehavior ?? 'json_object'} onChange={(event) => onRuntimeChange(profile.role, { structuredOutputBehavior: event.target.value })}><option value="json_object">JSON object</option><option value="json_schema">JSON schema</option><option value="prompt_only">Prompt only</option></select></label>
        <label>Capability source<input value={runtime?.capabilitySource ?? ''} onChange={(event) => onRuntimeChange(profile.role, { capabilitySource: event.target.value })} /></label>
      </div>
      <small>Capacity: input + reserved output + safety margin must fit. {runtime?.lastValidatedAt ? `Last validated ${runtime.lastValidatedAt}.` : 'Not persisted yet.'}</small>
      <details><summary>Permissions</summary><p>May publish: {profile.allowedArtifacts.join(', ')}</p><p>Forbidden: {profile.forbiddenActions.join(', ')}</p></details>
      <div className="buttonrow"><button disabled={loading || !runtime?.runtime.trim() || !runtime?.model.trim()} onClick={() => onRuntimeSave(profile.role)}>Save runtime</button><button disabled={loading || !runtime?.runtime.trim() || !runtime?.model.trim()} onClick={() => onRoleRun(profile.role)}>Run typed role</button></div>
    </article>;
  })}</section><label>Filter team timeline<input value={filter} onChange={(event) => onFilter(event.target.value)} /></label>
    <div className="team-summary"><span>{runs.length} role runs</span><span>{artifacts.length} artifacts</span><span>{questions.length} questions</span><span>{events.length} visible events</span></div>
    <div className="role-run-list">{runs.slice(0, 30).map((run) => <div key={String(run.id)}><strong>{String(run.role)}</strong><span>{String(run.status)} · {String(run.runtime)}/{String(run.model)} · spec v{String(run.specVersion)}</span>{run.status === 'prepared' && <button disabled={loading} onClick={() => onCancelRun(String(run.id))}>Cancel</button>}</div>)}</div>
    <section className="capsule-list" aria-label="Exact Context Capsules"><h3>Context Capsules</h3>{capsules.slice(0, 20).map((capsule) => {
      const windowTokens = Number(capsule.modelContextWindowTokens ?? 0); const inputTokens = Number(capsule.compiledInputTokens ?? 0);
      const outputTokens = Number(capsule.reservedOutputTokens ?? 0); const safetyTokens = Number(capsule.safetyMarginTokens ?? 0);
      return <details key={String(capsule.id)}><summary>{String(capsule.role)} · {inputTokens}/{windowTokens} input tokens · {String(capsule.tokenCountKind)} via {String(capsule.tokenEstimationMethod)}</summary>
        <div className="capsule-metrics"><span>Task {String(capsule.taskId ?? 'none')}</span><span>Spec v{String(capsule.activeSpecVersion)}</span><span>Reserved output {outputTokens}</span><span>Safety {safetyTokens}</span><span>Remaining {Number(capsule.remainingCapacityTokens ?? windowTokens - inputTokens - outputTokens - safetyTokens)}</span><span>Hash {String(capsule.messagesSha256)}</span></div>
        <JsonSections sections={[["Included hot items", capsule.includedArtifacts], ["Omitted/warm/cold items", capsule.omittedArtifacts], ["Summaries used", capsule.summarizedArtifacts], ["Truncations", capsule.truncationRecords], ["Exact prompt/messages", capsule.exactMessages]]} />
      </details>;
    })}</section>
    <ol className="team-timeline">{events.map((event, index) => <li key={String(event.id ?? index)}><strong>{String(event.kind ?? 'event')}</strong><span>{String(event.actorRole ?? 'system')} · {String(event.createdAt ?? '')}</span><p>{String(event.redactedSummary ?? '')}</p></li>)}</ol></div>;
}

function PulsePanel({ pulse, onEvaluate, loading }: { pulse: PulseView | null; onEvaluate: () => unknown; loading: boolean }) {
  const findings = [...(pulse?.symbolicFindings ?? []), ...(pulse?.temporalFindings ?? [])];
  return <div><div className="buttonrow"><button onClick={onEvaluate} disabled={loading}>Evaluate Pulse</button></div>
    {!pulse ? <div className="studio-empty">Pulse has not evaluated this initiative yet.</div> : <>
      <div className="pulse-metadata"><pre>{JSON.stringify(pulse.ruleObserver, null, 2)}</pre><pre>{JSON.stringify(pulse.liquidObserver, null, 2)}</pre></div>
      <div className="pulse-findings">{findings.map((finding, index) => <article key={`${finding.kind}-${index}`}><span>{pulseSourceLabel(finding)}</span><strong>{finding.kind} · {Math.round(finding.severity * 100)}%</strong><p>{finding.primaryFactors.join(', ')}</p><small>Proposal: {finding.recommendedIntervention} · evidence: {finding.supportingEvents.join(', ')}</small></article>)}</div>
    </>}</div>;
}

function EvidenceView({ proof, evidence }: { proof: Snapshot['proof']; evidence: ReturnType<typeof summarizeEvidence> }) {
  return <div><div className={`evidence-banner ${evidence.verified ? 'ok' : 'warn'}`}>{evidence.verified ? 'All listed requirements have required evidence.' : `${evidence.incomplete} requirement state(s) still need proof.`}</div><JsonSections sections={[["Requirement evidence", proof.evidence], ["Missing or blocked", { incomplete: proof.incomplete, blocked: proof.blocked, unverified: proof.unverified }]]} /></div>;
}

function ChangesView({ snapshot, canCreateWorktree, onCreateWorktree, onExport }: { snapshot: Snapshot; canCreateWorktree: boolean; onCreateWorktree: () => unknown; onExport: () => unknown }) {
  return <div className="studio-sections"><section><h3>Isolated workspace</h3><pre>{JSON.stringify(snapshot.worktree ?? { status: 'not created' }, null, 2)}</pre>{!snapshot.worktree && <button onClick={onCreateWorktree} disabled={!canCreateWorktree}>Approve isolated worktree</button>}</section><section><h3>Proof-carrying change set</h3><p>Generated from persisted objectives, assumptions, constraints, requirements, ADRs, tasks, operations, and evidence. Exact context is excluded.</p><button onClick={onExport}>Export local structured proof</button></section></div>;
}

function title(value: string) { return value.charAt(0).toUpperCase() + value.slice(1); }
function stableRepoKey(value: string) { let hash = 2166136261; for (const char of value) { hash ^= char.charCodeAt(0); hash = Math.imul(hash, 16777619); } return (hash >>> 0).toString(16); }
