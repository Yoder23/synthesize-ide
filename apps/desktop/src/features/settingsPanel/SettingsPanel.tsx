export type EditorSettings = {
  wordWrap: 'on' | 'off';
  fontSize: number;
  minimap: boolean;
  theme: 'vs-dark' | 'light';
};

export function SettingsPanel(props: { open: boolean; settings: EditorSettings; onChange: (settings: EditorSettings) => void; onClose: () => void }) {
  if (!props.open) return null;
  const update = (patch: Partial<EditorSettings>) => props.onChange({ ...props.settings, ...patch });
  return (
    <div className="modal-backdrop" onClick={props.onClose}>
      <div className="modal settings" onClick={(e) => e.stopPropagation()}>
        <div className="panel-heading-row"><h3>Settings</h3><button onClick={props.onClose}>Close</button></div>
        <label>Font size<input type="number" min={10} max={28} value={props.settings.fontSize} onChange={(e) => update({ fontSize: Number(e.target.value) || 14 })} /></label>
        <label>Word wrap<select value={props.settings.wordWrap} onChange={(e) => update({ wordWrap: e.target.value as EditorSettings['wordWrap'] })}><option value="off">Off</option><option value="on">On</option></select></label>
        <label><input type="checkbox" checked={props.settings.minimap} onChange={(e) => update({ minimap: e.target.checked })} /> Minimap</label>
        <label>Theme<select value={props.settings.theme} onChange={(e) => update({ theme: e.target.value as EditorSettings['theme'] })}><option value="vs-dark">Dark</option><option value="light">Light</option></select></label>
        <div className="small">Settings persist locally in the browser/Tauri webview storage. Backend-governed trust settings remain backend-owned.</div>
      </div>
    </div>
  );
}
