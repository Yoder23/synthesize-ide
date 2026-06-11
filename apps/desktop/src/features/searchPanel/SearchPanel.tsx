import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { RepoState } from '../../app/App';
import { SESSION_ID } from '../../app/App';

type SearchResult = { path: string; line: number; preview: string };

export function SearchPanel(props: { repo: RepoState | null; onOpenFile: (path: string) => void }) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  async function search() {
    if (!props.repo || query.trim().length < 2) return;
    setError(null);
    try {
      const r = await invoke<SearchResult[]>('project_search', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot, query, max_results: 80 } });
      setResults(r);
    } catch (err) { setError(String(err)); }
  }
  return (
    <div className="panel search-panel">
      <h3>Project Search</h3>
      <div className="row"><input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search guarded text files" onKeyDown={(e) => { if (e.key === 'Enter') void search(); }} /><button onClick={search} disabled={!props.repo}>Search</button></div>
      {error && <div className="notice error">{error}</div>}
      <div className="small">Search reads only allowed text-like files through the backend guard; denied paths are skipped.</div>
      <div className="resultlist">
        {results.map((r, idx) => <button key={`${r.path}:${r.line}:${idx}`} onClick={() => props.onOpenFile(r.path)}><strong>{r.path}:{r.line}</strong><span>{r.preview}</span></button>)}
      </div>
    </div>
  );
}
