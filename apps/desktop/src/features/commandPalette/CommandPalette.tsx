export type PaletteCommand = {
  id: string;
  label: string;
  description: string;
  run: () => void | Promise<void>;
  disabled?: boolean;
};

export function CommandPalette(props: { open: boolean; commands: PaletteCommand[]; onClose: () => void }) {
  if (!props.open) return null;
  return (
    <div className="modal-backdrop" onClick={props.onClose}>
      <div className="modal palette" onClick={(e) => e.stopPropagation()}>
        <div className="panel-heading-row"><h3>Command Palette</h3><button onClick={props.onClose}>Close</button></div>
        <div className="small">Keyboard-first workbench actions. Backend-governed operations still enforce their own authority.</div>
        <div className="command-list">
          {props.commands.map((command) => (
            <button key={command.id} disabled={command.disabled} onClick={async () => { await command.run(); props.onClose(); }}>
              <strong>{command.label}</strong>
              <span>{command.description}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
