import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { RepoState } from '../../app/App';
import { SESSION_ID } from '../../app/App';

type LspCapability = { language: string; detected: boolean; server_hint: string; capabilities: string[]; notes: string };

export function LspPanel(props: { repo: RepoState | null }) {
  const [items, setItems] = useState<LspCapability[]>([]);
  useEffect(() => {
    async function load() {
      if (!props.repo) { setItems([]); return; }
      const r = await invoke<LspCapability[]>('lsp_capabilities', { req: { session_id: SESSION_ID, repo_root: props.repo.repoRoot } });
      setItems(r);
    }
    void load();
  }, [props.repo?.repoRoot]);
  return (
    <div className="panel lsp-panel">
      <h3>Language Intelligence</h3>
      <div className="small">V16 keeps LSP as a transparent foundation/status layer. Full live LSP JSON-RPC wiring is the next IDE-hardening step.</div>
      {items.map((item) => (
        <div className={item.detected ? 'notice ok' : 'notice'} key={item.language}>
          <strong>{item.language}</strong> · {item.detected ? 'detected' : 'not detected'}
          <div className="small">Server: {item.server_hint}</div>
          <div className="small">Planned: {item.capabilities.join(', ')}</div>
        </div>
      ))}
    </div>
  );
}
