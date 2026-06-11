import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { RepoState } from '../../app/App';
import { SESSION_ID } from '../../app/App';

type DetectedTask = { id: string; label: string; argv: string[]; cwd: string; risk: string; reason: string; requires_network: boolean; may_modify_files: boolean };
type TaskApproval = { command_id: string; task_id: string; risk: string; approved: boolean; message: string };
type TaskRunResult = { command_id: string; exit_code?: number | null; timed_out: boolean; stdout_tail: string; stderr_tail: string; message: string; audit_event_id?: string | null };
type TaskHistoryItem = { id: string; task_id: string; label: string; exit_code?: number | null; timed_out: boolean; created_at: string };

function repairPrompt(task: DetectedTask, result: TaskRunResult): string {
  return `A governed task just ran in Synthesize and produced output. Analyze the failure/output and propose a small repair patch if appropriate. Return Synthesize typed operations only.\n\nTask: ${task.label}\nCommand argv: ${JSON.stringify(task.argv)}\nExit code: ${result.exit_code ?? 'none'}\nTimed out: ${result.timed_out}\n\nSTDOUT tail:\n\`\`\`\n${result.stdout_tail.slice(-6000)}\n\`\`\`\n\nSTDERR tail:\n\`\`\`\n${result.stderr_tail.slice(-6000)}\n\`\`\``;
}

export function TaskRunner(props: { repo: RepoState | null; onRefreshAudit: () => void; onSendOutputToAgent?: (prompt: string) => void }) {
  const [tasks, setTasks] = useState<DetectedTask[]>([]);
  const [approvalByTask, setApprovalByTask] = useState<Record<string, TaskApproval | undefined>>({});
  const [resultByTask, setResultByTask] = useState<Record<string, TaskRunResult | undefined>>({});
  const [history, setHistory] = useState<TaskHistoryItem[]>([]);
  const [runningTaskId, setRunningTaskId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const lastStatus = useMemo(() => {
    const last = history[0];
    if (!last) return 'No task has run in this session.';
    return `${last.label}: exit ${last.exit_code ?? 'none'}${last.timed_out ? ' · timed out' : ''}`;
  }, [history]);

  async function detect() {
    if (!props.repo) return;
    setError(null);
    try {
      const r = await invoke<DetectedTask[]>('detect_tasks', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot } });
      setTasks(r);
      props.onRefreshAudit();
    } catch (err) { setError(String(err)); }
  }

  useEffect(() => { void detect(); }, [props.repo?.repoRoot]);

  async function approve(task: DetectedTask) {
    if (!props.repo) return;
    setError(null);
    try {
      const r = await invoke<TaskApproval>('approve_task', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, task_id: task.id } });
      setApprovalByTask((prev) => ({ ...prev, [task.id]: r }));
      props.onRefreshAudit();
    } catch (err) { setError(String(err)); }
  }

  async function run(task: DetectedTask) {
    const approval = approvalByTask[task.id];
    if (!props.repo || !approval?.approved || runningTaskId) return;
    setError(null);
    setRunningTaskId(task.id);
    try {
      const r = await invoke<TaskRunResult>('run_approved_task', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, command_id: approval.command_id } });
      setResultByTask((prev) => ({ ...prev, [task.id]: r }));
      setHistory((prev) => [{ id: crypto.randomUUID(), task_id: task.id, label: task.label, exit_code: r.exit_code, timed_out: r.timed_out, created_at: new Date().toISOString() }, ...prev].slice(0, 10));
      props.onRefreshAudit();
    } catch (err) { setError(String(err)); }
    finally { setRunningTaskId(null); }
  }

  function cancelCurrentTask() {
    setError('Task cancellation is not wired for the current synchronous restricted runner yet. The backend timeout remains 120 seconds. V19 should make task execution async/cancellable.');
  }

  return (
    <div className="panel task-runner">
      <h3>Governed Tasks</h3>
      <div className="small">Code execution is possible only through backend-detected task snapshots: argv-only, backend-approved, audited, timeout-bounded, and env-scrubbed. Agent-suggested commands remain classification-only. This is not a free shell and not OS-network-sandboxed.</div>
      <div className="notice"><strong>Last task:</strong> {lastStatus}</div>
      <div className="row"><button onClick={detect} disabled={!props.repo || !!runningTaskId}>Detect tasks</button><button onClick={cancelCurrentTask} disabled={!runningTaskId}>Cancel running task</button></div>
      {error && <div className="notice error">{error}</div>}
      {tasks.map((task) => {
        const approval = approvalByTask[task.id];
        const result = resultByTask[task.id];
        const running = runningTaskId === task.id;
        return (
          <div className="diffitem" key={task.id}>
            <strong>{task.label}</strong>
            <div className="small"><code>{task.argv.join(' ')}</code> · cwd: {task.cwd}</div>
            <div className="small">Risk: {task.risk} · network: {String(task.requires_network)} · may modify files: {String(task.may_modify_files)}</div>
            <div className="small">{task.reason}</div>
            <div className="small">Approval uses backend-persisted task_id <code>{task.id}</code>; the frontend does not send argv/cwd at approval time.</div>
            <div className="row">
              <button onClick={() => approve(task)} disabled={!!runningTaskId}>Approve in backend</button>
              <button disabled={!approval?.approved || !!runningTaskId} onClick={() => run(task)}>{running ? 'Running...' : result ? 'Rerun approved task' : 'Run approved task'}</button>
              <button disabled={!result} onClick={() => result && props.onSendOutputToAgent?.(repairPrompt(task, result))}>Feed output to agent</button>
            </div>
            {approval && <div className={approval.approved ? 'notice ok' : 'notice error'}>{approval.message}</div>}
            {result && <details open><summary>Result: exit {result.exit_code ?? 'none'}{result.timed_out ? ' · timed out' : ''}</summary><pre>{result.stdout_tail}</pre>{result.stderr_tail && <pre className="stderr">{result.stderr_tail}</pre>}</details>}
          </div>
        );
      })}
      {history.length > 0 && <details><summary>Task history</summary>{history.map((h) => <div key={h.id} className="small">{h.created_at}: {h.label} · exit {h.exit_code ?? 'none'}{h.timed_out ? ' · timed out' : ''}</div>)}</details>}
    </div>
  );
}
