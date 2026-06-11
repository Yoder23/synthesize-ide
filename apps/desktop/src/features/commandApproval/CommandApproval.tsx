import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { CommandRequestOperation } from '@synthesize/shared-types';
import type { RepoState } from '../../app/App';
import { SESSION_ID } from '../../app/App';

type CommandClassifyResult = { ok: boolean; risk: string; message: string };

export function CommandApproval(props: { operation?: CommandRequestOperation; repo: RepoState | null }) {
  const [classification, setClassification] = useState<CommandClassifyResult | null>(null);

  useEffect(() => {
    async function classify() {
      if (!props.operation) {
        setClassification(null);
        return;
      }
      const result = await invoke<CommandClassifyResult>('classify_command', {
        req: {
          argv: props.operation.argv,
          cwd: props.operation.cwd,
          requires_network: props.operation.requiresNetwork,
          may_modify_files: props.operation.mayModifyFiles,
          session_id: SESSION_ID,
          repo_root: props.repo?.repoRoot ?? null
        }
      });
      setClassification(result);
    }
    void classify();
  }, [props.operation]);

  return (
    <div className="panel command-panel">
      <h3>Command Approval</h3>
      {!props.operation ? (
        <div className="small">No command requested. Commands are classification-only; execution remains disabled.</div>
      ) : (
        <div className="diffitem">
          <strong>{props.operation.argv.join(' ')}</strong>
          <div className="small">cwd: {props.operation.cwd}</div>
          <div className="small">repo: {props.repo?.repoRoot ?? 'none'}</div>
          <div className="small">reason: {props.operation.reason}</div>
          <div className="small">expected: {props.operation.expectedOutcome}</div>
          <div className={classification?.ok ? 'notice ok' : 'notice error'}>
            <strong>Risk: {classification?.risk ?? 'classifying...'}</strong>
            <div className="small">{classification?.message ?? 'Backend command guard has not responded yet.'}</div>
          </div>
          <button disabled>Run command — disabled</button>
        </div>
      )}
    </div>
  );
}
