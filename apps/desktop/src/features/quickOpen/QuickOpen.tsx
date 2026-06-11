import { useMemo, useState } from 'react';
import type { RepoState } from '../../app/App';

export function QuickOpen(props: { open: boolean; repo: RepoState | null; onOpenFile: (path: string) => void; onClose: () => void }) {
  const [query, setQuery] = useState('');
  const matches = useMemo(() => {
    if (!props.repo) return [];
    const q = query.toLowerCase();
    return props.repo.files
      .filter((f) => f.kind === 'file' && !f.denied)
      .filter((f) => !q || f.path.toLowerCase().includes(q))
      .slice(0, 80);
  }, [props.repo, query]);
  if (!props.open) return null;
  return (
    <div className="modal-backdrop" onClick={props.onClose}>
      <div className="modal quick-open" onClick={(e) => e.stopPropagation()}>
        <div className="panel-heading-row"><h3>Quick Open</h3><button onClick={props.onClose}>Close</button></div>
        <input autoFocus value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Type a file path…" onKeyDown={(e) => {
          if (e.key === 'Enter' && matches[0]) { props.onOpenFile(matches[0].path); props.onClose(); }
          if (e.key === 'Escape') props.onClose();
        }} />
        <div className="resultlist quick-results">
          {matches.map((file) => <button key={file.path} onClick={() => { props.onOpenFile(file.path); props.onClose(); }}>{file.path}</button>)}
        </div>
      </div>
    </div>
  );
}
