import { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { RepoState } from '../../app/App';
import { SESSION_ID } from '../../app/App';

type TaskApproval = { command_id: string; task_id: string; risk: string; approved: boolean; message: string };
type TaskRunResult = { command_id: string; exit_code?: number | null; timed_out: boolean; stdout_tail: string; stderr_tail: string; message: string; audit_event_id?: string | null };

type HistoryItem = {
  id: string;
  command: string;
  argv: string[];
  cwd: string;
  risk?: string;
  exit_code?: number | null;
  timed_out?: boolean;
  stdout_tail?: string;
  stderr_tail?: string;
};

function parseCommandLine(input: string): string[] {
  const out: string[] = [];
  let current = '';
  let quote: 'single' | 'double' | null = null;
  let escape = false;
  for (const ch of input) {
    if (escape) { current += ch; escape = false; continue; }
    if (ch === '\\') { escape = true; continue; }
    if (quote === 'single') {
      if (ch === "'") quote = null; else current += ch;
      continue;
    }
    if (quote === 'double') {
      if (ch === '"') quote = null; else current += ch;
      continue;
    }
    if (ch === "'") { quote = 'single'; continue; }
    if (ch === '"') { quote = 'double'; continue; }
    if (/\s/.test(ch)) {
      if (current) { out.push(current); current = ''; }
      continue;
    }
    current += ch;
  }
  if (escape) current += '\\';
  if (quote) throw new Error('Unclosed quote in command.');
  if (current) out.push(current);
  return out;
}

function repairPrompt(item: HistoryItem): string {
  return `A Synthesize personal terminal command just ran. Analyze the output and propose the smallest safe repair patch if needed. Return Synthesize typed operations only.\n\nCommand: ${item.command}\nArgv: ${JSON.stringify(item.argv)}\nCwd: ${item.cwd}\nRisk: ${item.risk ?? 'unknown'}\nExit code: ${item.exit_code ?? 'none'}\nTimed out: ${String(item.timed_out)}\n\nSTDOUT tail:\n\`\`\`\n${(item.stdout_tail ?? '').slice(-6000)}\n\`\`\`\n\nSTDERR tail:\n\`\`\`\n${(item.stderr_tail ?? '').slice(-6000)}\n\`\`\``;
}

export function PersonalTerminal(props: {
  repo: RepoState | null;
  onRefreshAudit?: () => void;
  onSendOutputToAgent?: (prompt: string) => void;
}) {
  const [command, setCommand] = useState('pnpm test');
  const [cwd, setCwd] = useState('.');
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [approval, setApproval] = useState<TaskApproval | null>(null);
  const [result, setResult] = useState<TaskRunResult | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);

  const argvPreview = useMemo(() => {
    try { return parseCommandLine(command); } catch { return []; }
  }, [command]);

  async function approveAndRun() {
    if (!props.repo) return;
    setError(null);
    setApproval(null);
    setResult(null);
    let argv: string[];
    try { argv = parseCommandLine(command); } catch (err) { setError(String(err)); return; }
    if (argv.length === 0) { setError('Enter a command first.'); return; }
    setRunning(true);
    try {
      const approved = await invoke<TaskApproval>('approve_personal_command', {
        req: {
          session_id: SESSION_ID,
          repo_root: props.repo.repoRoot,
          argv,
          cwd,
          requires_network: false,
          may_modify_files: false
        }
      });
      setApproval(approved);
      props.onRefreshAudit?.();
      if (!approved.approved) {
        setHistory((prev) => [{ id: crypto.randomUUID(), command, argv, cwd, risk: approved.risk }, ...prev].slice(0, 12));
        return;
      }
      const ran = await invoke<TaskRunResult>('run_approved_task', {
        req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, command_id: approved.command_id }
      });
      setResult(ran);
      setHistory((prev) => [{
        id: crypto.randomUUID(),
        command,
        argv,
        cwd,
        risk: approved.risk,
        exit_code: ran.exit_code,
        timed_out: ran.timed_out,
        stdout_tail: ran.stdout_tail,
        stderr_tail: ran.stderr_tail
      }, ...prev].slice(0, 12));
      props.onRefreshAudit?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }

  const last = history[0];

  return (
    <div className="panel personal-terminal">
      <h3>Personal Terminal</h3>
      <div className="small">Run repo-local commands for your iteration loop. Personal Terminal now uses strict explicit-rule policy: safe Git reads, search/list/read commands, and known test/build commands only. No shell, no fallback allowlist, repo-bounded cwd, env scrubbing, audit logging, and a 120s timeout.</div>
      <label className="small">Command</label>
      <input value={command} onChange={(e) => setCommand(e.target.value)} placeholder="pnpm test" disabled={!props.repo || running} />
      <label className="small">Working directory inside repo</label>
      <input value={cwd} onChange={(e) => setCwd(e.target.value)} placeholder="." disabled={!props.repo || running} />
      <div className="small">Argv preview: <code>{argvPreview.length ? JSON.stringify(argvPreview) : '[]'}</code></div>
      <div className="small">Allowed examples: <code>pnpm test</code>, <code>pnpm run lint</code>, <code>cargo test</code>, <code>pytest</code>, <code>go test ./...</code>, <code>git status</code>, <code>git diff</code>, <code>rg TODO</code>.</div>
      <div className="row">
        <button className="primary" onClick={approveAndRun} disabled={!props.repo || running}>{running ? 'Running...' : 'Approve + run'}</button>
        <button disabled={!last?.stdout_tail && !last?.stderr_tail} onClick={() => last && props.onSendOutputToAgent?.(repairPrompt(last))}>Feed last output to agent</button>
      </div>
      {approval && <div className={approval.approved ? 'notice ok' : 'notice error'}><strong>{approval.approved ? 'Approved' : 'Refused'}</strong><div className="small">Risk: {approval.risk} · {approval.message}</div></div>}
      {result && <details open><summary>Result: exit {result.exit_code ?? 'none'}{result.timed_out ? ' · timed out' : ''}</summary><pre>{result.stdout_tail}</pre>{result.stderr_tail && <pre className="stderr">{result.stderr_tail}</pre>}</details>}
      {error && <div className="notice error">{error}</div>}
      {history.length > 0 && <details><summary>Terminal history</summary>{history.map((h) => <div key={h.id} className="small"><code>{h.command}</code> · exit {h.exit_code ?? 'none'} · {h.risk ?? 'unclassified'}</div>)}</details>}
    </div>
  );
}
