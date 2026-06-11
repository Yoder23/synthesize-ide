import type { RepoState } from '../../app/App';

export function RepoExplorer(props: {
  repo: RepoState | null;
  onOpenFile: (path: string) => void;
  onCreateFile?: () => void;
  onRenameCurrent?: () => void;
  onDeleteCurrent?: () => void;
}) {
  return (
    <div className="panel repo-explorer">
      <div className="panel-heading-row"><h3>Repo Explorer</h3><button onClick={props.onCreateFile} disabled={!props.repo}>New</button></div>
      {!props.repo ? <div className="small">Open a repo to inspect guarded files.</div> : (
        <>
          <div className="small">{props.repo.files.length} indexed entries. Denied files are visible as policy signals but cannot be opened.</div>
          <div className="row mini-actions"><button onClick={props.onRenameCurrent}>Rename active</button><button onClick={props.onDeleteCurrent}>Delete active</button></div>
          <div className="file-list">
            {props.repo.files.map((file) => (
              <button
                key={`${file.kind}:${file.path}`}
                className={`file-row ${file.denied ? 'denied' : ''}`}
                disabled={file.kind !== 'file' || file.denied}
                onClick={() => props.onOpenFile(file.path)}
                title={file.denied ? 'Denied by RepoGuard policy' : file.path}
              >
                <span>{file.kind.startsWith('dir') ? '▸' : '•'}</span>
                <span>{file.path}</span>
                {file.denied && <em>denied</em>}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
