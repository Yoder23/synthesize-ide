import type { RepoState } from '../../app/App';

type Problem = { severity: 'info' | 'warning' | 'error'; line: number; message: string };

function braceBalanceProblems(content: string): Problem[] {
  const problems: Problem[] = [];
  const stack: Array<{ char: string; line: number }> = [];
  const pairs: Record<string, string> = { ')': '(', ']': '[', '}': '{' };
  const lines = content.split('\n');
  for (let li = 0; li < lines.length; li++) {
    const line = lines[li];
    for (const ch of line) {
      if (ch === '(' || ch === '[' || ch === '{') stack.push({ char: ch, line: li + 1 });
      if (ch === ')' || ch === ']' || ch === '}') {
        const last = stack.pop();
        if (!last || last.char !== pairs[ch]) problems.push({ severity: 'error', line: li + 1, message: `Unbalanced closing ${ch}` });
      }
    }
  }
  for (const item of stack.slice(-5)) problems.push({ severity: 'warning', line: item.line, message: `Possible unclosed ${item.char}` });
  return problems;
}

function lightweightProblems(content: string): Problem[] {
  const problems: Problem[] = [];
  const lines = content.split('\n');
  lines.forEach((line, index) => {
    if (/TODO|FIXME/.test(line)) problems.push({ severity: 'info', line: index + 1, message: 'TODO/FIXME marker' });
    if (/throw new Error\(["']not implemented["']\)/i.test(line)) problems.push({ severity: 'warning', line: index + 1, message: 'Not implemented placeholder' });
    if (/console\.log\(/.test(line)) problems.push({ severity: 'info', line: index + 1, message: 'console.log present' });
  });
  return [...problems, ...braceBalanceProblems(content)].slice(0, 30);
}

export function ProblemsPanel(props: { repo: RepoState | null; content: string; onGoToLine?: (line: number) => void }) {
  const problems = lightweightProblems(props.content);
  return (
    <div className="panel problems-panel">
      <h3>Problems</h3>
      <div className="small">Lightweight local checks only. Full LSP diagnostics are the next milestone.</div>
      {!props.repo ? <div className="empty">Open a repo to inspect the active file.</div> : problems.length === 0 ? <div className="notice ok">No lightweight problems detected in the active buffer.</div> : (
        <div className="problem-list">
          {problems.map((p, i) => <button key={`${p.line}-${i}`} className={`problem-row ${p.severity}`} onClick={() => props.onGoToLine?.(p.line)}><span className="badge">{p.severity}</span> line {p.line}: {p.message}</button>)}
        </div>
      )}
    </div>
  );
}
