import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { RepoState } from '../../app/App';
import { SESSION_ID } from '../../app/App';

type GitStatusFile = { path: string; status: string };
type GitStatusView = { branch: string; files: GitStatusFile[]; raw: string };
type GitMutationResult = { ok: boolean; message: string; stdout: string; stderr: string; audit_event_id?: string | null };
type GitDiffView = { path: string; staged: boolean; diff: string; truncated: boolean; audit_event_id?: string | null };

export function GitPanel(props: { repo: RepoState | null; onOpenFile: (path: string) => void }) {
  const [status, setStatus] = useState<GitStatusView | null>(null);
  const [commitMessage, setCommitMessage] = useState('');
  const [lastMutation, setLastMutation] = useState<GitMutationResult | null>(null);
  const [diff, setDiff] = useState<GitDiffView | null>(null);
  const [error, setError] = useState<string | null>(null);
  async function refresh() {
    if (!props.repo) return;
    setError(null);
    try {
      const result = await invoke<GitStatusView>('git_status', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot } });
      setStatus(result);
    } catch (err) { setError(String(err)); }
  }
  async function showDiff(path: string, staged = false) {
    if (!props.repo) return;
    setError(null);
    try {
      const result = await invoke<GitDiffView>('git_diff_file', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, path, staged } });
      setDiff(result);
    } catch (err) { setError(String(err)); }
  }
  async function stage(path: string) {
    if (!props.repo) return;
    const result = await invoke<GitMutationResult>('git_stage_file', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, path } });
    setLastMutation(result); await refresh();
  }
  async function unstage(path: string) {
    if (!props.repo) return;
    const result = await invoke<GitMutationResult>('git_unstage_file', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, path } });
    setLastMutation(result); await refresh();
  }
  async function commit() {
    if (!props.repo || commitMessage.trim().length < 3) return;
    const result = await invoke<GitMutationResult>('git_commit_changes', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, message: commitMessage } });
    setLastMutation(result); if (result.ok) setCommitMessage(''); await refresh();
  }
  return (
    <div className="panel git-panel">
      <h3>Source Control</h3>
      <button onClick={refresh} disabled={!props.repo}>Refresh Git status</button>
      {error && <div className="notice error">{error}</div>}
      {!status ? <div className="small">Git status, stage/unstage, and commit are user-initiated backend commands. Commit uses <code>--no-verify</code> to avoid repo hook execution.</div> : (
        <div>
          <div className="small">Branch: <strong>{status.branch}</strong></div>
          <div className="small">Changed files: {status.files.length}</div>
          <div className="filelist compact">
            {status.files.map((f) => <div className="git-row" key={`${f.status}-${f.path}`}><button onClick={() => props.onOpenFile(f.path)}><span className="badge">{f.status || '?'}</span> {f.path}</button><button onClick={() => showDiff(f.path, false)}>Diff</button><button onClick={() => showDiff(f.path, true)}>Staged diff</button><button onClick={() => stage(f.path)}>Stage</button><button onClick={() => unstage(f.path)}>Unstage</button></div>)}
          </div>
          <div className="commitbox">
            <textarea value={commitMessage} onChange={(e) => setCommitMessage(e.target.value)} placeholder="Commit message" />
            <button onClick={commit} disabled={commitMessage.trim().length < 3}>Commit staged changes</button>
            <div className="small">Git mutations are audited. Commit uses argv-only process spawning and <code>--no-verify</code>.</div>
          </div>
        </div>
      )}
      {diff && <details open className="diff-preview"><summary>Diff: {diff.path}{diff.staged ? ' · staged' : ''}{diff.truncated ? ' · truncated' : ''}</summary><pre>{diff.diff || '(no diff)'}</pre></details>}
      {lastMutation && <div className={lastMutation.ok ? 'notice ok' : 'notice error'}>{lastMutation.message}{lastMutation.stderr && <pre className="stderr">{lastMutation.stderr}</pre>}</div>}
    </div>
  );
}
