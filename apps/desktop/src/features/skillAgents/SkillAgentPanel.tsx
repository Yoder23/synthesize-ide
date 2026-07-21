import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------------
// Types (mirrored from Rust backend — keep in sync with main.rs)
// ---------------------------------------------------------------------------

export type SkillDefinition = {
  id: string;
  name: string;
  description: string;
  model_registry_id: string;
  tier: 'fast' | 'balanced' | 'powerful' | 'frontier-local' | 'cloud-heavy' | 'cloud-reasoning';
  system_prompt_addon: string;
  allowed_operations: string[];
  allowed_hand_off_targets: string[];
  max_iterations: number;
  enabled: boolean;
  tags: string[];
};

export type SkillQueueEntry = {
  id: string;
  skill_id: string;
  skill_name: string;
  context_summary: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  started_at?: string | null;
  completed_at?: string | null;
  error_message?: string | null;
  iterations: number;
  hand_off_from?: string | null;
};

export type SkillQueueState = {
  current_entry: SkillQueueEntry | null;
  queue: SkillQueueEntry[];
  history: SkillQueueEntry[];
  gpu_lock_held: boolean;
};

type SkillSpawnResult = {
  entry_id: string;
  skill_id: string;
  skill_name: string;
  status: string;
  message: string;
  gpu_lock_required: boolean;
};

// ---------------------------------------------------------------------------
// Tier colors
// ---------------------------------------------------------------------------

function tierBadgeStyle(tier: SkillDefinition['tier']): { background: string; color: string } {
  switch (tier) {
    case 'fast': return { background: '#1e3a5f', color: '#7ec8e3' };
    case 'balanced': return { background: '#1a3d2b', color: '#6ee7a0' };
    case 'powerful': return { background: '#3a2c1a', color: '#ffb347' };
    case 'frontier-local': return { background: '#3d1a3d', color: '#d97ef7' };
    case 'cloud-heavy': return { background: '#3d1a1a', color: '#ff7979' };
    case 'cloud-reasoning': return { background: '#2a1a1a', color: '#ff5555' };
  }
}

function statusColor(status: string): string {
  switch (status) {
    case 'running': return '#6ee7a0';
    case 'completed': return '#4a9eff';
    case 'failed': return '#ff5555';
    case 'cancelled': return '#888';
    case 'queued': return '#ffb347';
    default: return '#ccc';
  }
}

// ---------------------------------------------------------------------------
// SkillCard — compact card for a single skill
// ---------------------------------------------------------------------------

function SkillCard(props: {
  skill: SkillDefinition;
  onSpawn: (skillId: string, context: string) => void;
  onEdit: (skill: SkillDefinition) => void;
}) {
  const [contextInput, setContextInput] = useState('');
  const [expanded, setExpanded] = useState(false);
  const tier = tierBadgeStyle(props.skill.tier);

  function handleSpawn() {
    props.onSpawn(props.skill.id, contextInput.trim() || 'No additional context provided.');
    setContextInput('');
  }

  return (
    <div style={{ border: '1px solid #333', borderRadius: 6, padding: '10px 12px', marginBottom: 8, background: '#1a1a1a' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
        <span style={{ fontWeight: 600, fontSize: 13, flex: 1 }}>{props.skill.name}</span>
        <span style={{ fontSize: 10, padding: '2px 6px', borderRadius: 4, ...tier }}>
          {props.skill.tier}
        </span>
        {!props.skill.enabled && (
          <span style={{ fontSize: 10, color: '#888', padding: '2px 6px', background: '#222', borderRadius: 4 }}>disabled</span>
        )}
        <button onClick={() => setExpanded(!expanded)} style={{ background: 'none', border: 'none', color: '#888', cursor: 'pointer', fontSize: 12 }}>
          {expanded ? '▲' : '▼'}
        </button>
        <button onClick={() => props.onEdit(props.skill)} style={{ background: 'none', border: '1px solid #444', color: '#ccc', cursor: 'pointer', fontSize: 11, padding: '2px 6px', borderRadius: 4 }}>
          Edit
        </button>
      </div>
      <div style={{ color: '#999', fontSize: 11, marginBottom: expanded ? 8 : 0 }}>{props.skill.description}</div>
      {expanded && (
        <div style={{ marginTop: 8 }}>
          <div style={{ fontSize: 11, color: '#777', marginBottom: 4 }}>
            Model: <span style={{ color: '#aaa' }}>{props.skill.model_registry_id}</span>
            {' · '}
            Max iterations: <span style={{ color: '#aaa' }}>{props.skill.max_iterations}</span>
          </div>
          {props.skill.tags.length > 0 && (
            <div style={{ fontSize: 11, color: '#777', marginBottom: 4 }}>
              Tags: {props.skill.tags.map(t => <span key={t} style={{ background: '#2a2a2a', padding: '1px 5px', borderRadius: 3, marginRight: 3 }}>{t}</span>)}
            </div>
          )}
          {props.skill.allowed_hand_off_targets.length > 0 && (
            <div style={{ fontSize: 11, color: '#777', marginBottom: 8 }}>
              Can hand off to: {props.skill.allowed_hand_off_targets.join(', ')}
            </div>
          )}
          {props.skill.enabled && (
            <>
              <textarea
                value={contextInput}
                onChange={(e) => setContextInput(e.target.value)}
                placeholder="Optional: describe the task or paste relevant context for this skill agent..."
                style={{ width: '100%', height: 60, background: '#111', color: '#ccc', border: '1px solid #444', borderRadius: 4, padding: 6, fontSize: 11, resize: 'vertical', boxSizing: 'border-box' }}
              />
              <button
                onClick={handleSpawn}
                style={{ marginTop: 6, background: '#1e3a5f', color: '#7ec8e3', border: 'none', borderRadius: 4, padding: '5px 12px', cursor: 'pointer', fontSize: 12, fontWeight: 600 }}
              >
                Spawn {props.skill.name}
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// QueueEntryRow — row in the queue/history list
// ---------------------------------------------------------------------------

function QueueEntryRow(props: { entry: SkillQueueEntry }) {
  const { entry } = props;
  return (
    <div style={{ borderBottom: '1px solid #222', padding: '6px 0', fontSize: 11 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{ color: statusColor(entry.status), fontWeight: 600, minWidth: 72 }}>{entry.status}</span>
        <span style={{ color: '#ccc', fontWeight: 500 }}>{entry.skill_name}</span>
        {entry.hand_off_from && (
          <span style={{ color: '#777' }}>← from {entry.hand_off_from}</span>
        )}
        <span style={{ color: '#555', marginLeft: 'auto' }}>{entry.iterations > 0 ? `${entry.iterations} iter` : ''}</span>
      </div>
      {entry.context_summary && (
        <div style={{ color: '#666', marginTop: 2, paddingLeft: 78 }}>{entry.context_summary.slice(0, 120)}{entry.context_summary.length > 120 ? '…' : ''}</div>
      )}
      {entry.error_message && (
        <div style={{ color: '#ff5555', marginTop: 2, paddingLeft: 78 }}>{entry.error_message}</div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// SkillEditor — simple form to edit a skill definition
// ---------------------------------------------------------------------------

function SkillEditor(props: {
  skill: SkillDefinition;
  onSave: (skill: SkillDefinition) => void;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState<SkillDefinition>(props.skill);

  function update<K extends keyof SkillDefinition>(key: K, value: SkillDefinition[K]) {
    setDraft(prev => ({ ...prev, [key]: value }));
  }

  return (
    <div style={{ background: '#161616', border: '1px solid #444', borderRadius: 8, padding: 16, marginBottom: 16 }}>
      <h4 style={{ margin: '0 0 12px', color: '#ccc' }}>Edit Skill: {draft.name}</h4>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
        <label style={{ fontSize: 11, color: '#999' }}>
          Name
          <input value={draft.name} onChange={e => update('name', e.target.value)} style={inputStyle} />
        </label>
        <label style={{ fontSize: 11, color: '#999' }}>
          Model Registry ID
          <input value={draft.model_registry_id} onChange={e => update('model_registry_id', e.target.value)} style={inputStyle} />
        </label>
        <label style={{ fontSize: 11, color: '#999', gridColumn: 'span 2' }}>
          Description
          <input value={draft.description} onChange={e => update('description', e.target.value)} style={inputStyle} />
        </label>
        <label style={{ fontSize: 11, color: '#999' }}>
          Tier
          <select value={draft.tier} onChange={e => update('tier', e.target.value as SkillDefinition['tier'])} style={{ ...inputStyle, height: 28 }}>
            <option value="fast">fast (Qwen3 0.6B)</option>
            <option value="balanced">balanced (Qwen3 1.7B)</option>
            <option value="powerful">powerful (Qwen3 8B)</option>
            <option value="frontier-local">frontier-local (Qwen3 14B+)</option>
            <option value="cloud-heavy">cloud-heavy (GPT-4o/Claude)</option>
            <option value="cloud-reasoning">cloud-reasoning (o3)</option>
          </select>
        </label>
        <label style={{ fontSize: 11, color: '#999' }}>
          Max Iterations
          <input type="number" min={1} max={50} value={draft.max_iterations} onChange={e => update('max_iterations', Number(e.target.value))} style={inputStyle} />
        </label>
        <label style={{ fontSize: 11, color: '#999', gridColumn: 'span 2' }}>
          System Prompt Addon (appended to base Synthesize prompt)
          <textarea
            value={draft.system_prompt_addon}
            onChange={e => update('system_prompt_addon', e.target.value)}
            style={{ ...inputStyle, height: 80, resize: 'vertical' }}
          />
        </label>
        <label style={{ fontSize: 11, color: '#999', gridColumn: 'span 2' }}>
          Allowed Hand-off Targets (comma-separated skill IDs)
          <input
            value={draft.allowed_hand_off_targets.join(', ')}
            onChange={e => update('allowed_hand_off_targets', e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
            style={inputStyle}
          />
        </label>
        <label style={{ fontSize: 11, color: '#999' }}>
          Tags (comma-separated)
          <input
            value={draft.tags.join(', ')}
            onChange={e => update('tags', e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
            style={inputStyle}
          />
        </label>
        <label style={{ fontSize: 11, color: '#999', display: 'flex', alignItems: 'center', gap: 8 }}>
          <input type="checkbox" checked={draft.enabled} onChange={e => update('enabled', e.target.checked)} />
          Enabled
        </label>
      </div>
      <div style={{ marginTop: 12, display: 'flex', gap: 8 }}>
        <button onClick={() => props.onSave(draft)} style={{ background: '#1a3d2b', color: '#6ee7a0', border: 'none', borderRadius: 4, padding: '6px 16px', cursor: 'pointer', fontSize: 12, fontWeight: 600 }}>
          Save Skill
        </button>
        <button onClick={props.onCancel} style={{ background: '#222', color: '#999', border: '1px solid #444', borderRadius: 4, padding: '6px 16px', cursor: 'pointer', fontSize: 12 }}>
          Cancel
        </button>
      </div>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  display: 'block', width: '100%', marginTop: 4, background: '#111', color: '#ccc',
  border: '1px solid #444', borderRadius: 4, padding: '4px 8px', fontSize: 12, boxSizing: 'border-box'
};

// ---------------------------------------------------------------------------
// Main SkillAgentPanel
// ---------------------------------------------------------------------------

export function SkillAgentPanel(props: { sessionId: string; repoRoot: string | null }) {
  const [skills, setSkills] = useState<SkillDefinition[]>([]);
  const [queueState, setQueueState] = useState<SkillQueueState | null>(null);
  const [editingSkill, setEditingSkill] = useState<SkillDefinition | null>(null);
  const [message, setMessage] = useState('');
  const [filter, setFilter] = useState('');
  const [tab, setTab] = useState<'skills' | 'queue' | 'history'>('skills');

  const refresh = useCallback(async () => {
    try {
      const [s, q] = await Promise.all([
        invoke<SkillDefinition[]>('skill_list'),
        invoke<SkillQueueState>('skill_queue_status')
      ]);
      setSkills(s);
      setQueueState(q);
    } catch (err) {
      setMessage(String(err));
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 3000);
    return () => clearInterval(interval);
  }, [refresh]);

  async function handleSpawn(skillId: string, context: string) {
    if (!props.repoRoot) { setMessage('Open a repository first.'); return; }
    setMessage('');
    try {
      const result = await invoke<SkillSpawnResult>('skill_queue_spawn', {
        req: { skill_id: skillId, context_summary: context, hand_off_from: null, session_id: props.sessionId, repo_root: props.repoRoot }
      });
      setMessage(result.message);
      await refresh();
    } catch (err) {
      setMessage(String(err));
    }
  }

  async function handleAdvance() {
    if (!props.repoRoot) { setMessage('Open a repository first.'); return; }
    setMessage('');
    try {
      const result = await invoke<SkillSpawnResult>('skill_queue_advance', { sessionId: props.sessionId, repoRoot: props.repoRoot });
      setMessage(result.message);
      await refresh();
    } catch (err) {
      setMessage(String(err));
    }
  }

  async function handleCancelAll() {
    setMessage('');
    try {
      await invoke('skill_queue_cancel_all');
      setMessage('All queued and running skill agents cancelled.');
      await refresh();
    } catch (err) {
      setMessage(String(err));
    }
  }

  async function handleSaveSkill(skill: SkillDefinition) {
    setMessage('');
    try {
      await invoke('skill_save', { skill });
      setMessage(`Skill '${skill.name}' saved.`);
      setEditingSkill(null);
      await refresh();
    } catch (err) {
      setMessage(String(err));
    }
  }

  async function handleDeleteSkill(skillId: string) {
    setMessage('');
    try {
      await invoke('skill_delete', { skillId });
      setMessage(`Skill '${skillId}' deleted; defaults will be used if registry is now empty.`);
      setEditingSkill(null);
      await refresh();
    } catch (err) {
      setMessage(String(err));
    }
  }

  async function handleResetToDefaults() {
    setMessage('');
    try {
      const defaults = await invoke<SkillDefinition[]>('skill_reset_to_defaults');
      setSkills(defaults);
      setMessage('Skills reset to built-in defaults.');
      setEditingSkill(null);
    } catch (err) {
      setMessage(String(err));
    }
  }

  const filteredSkills = filter.trim()
    ? skills.filter(s => s.name.toLowerCase().includes(filter.toLowerCase()) || s.tags.some(t => t.toLowerCase().includes(filter.toLowerCase())))
    : skills;

  const currentEntry = queueState?.current_entry;
  const queuedCount = queueState?.queue.length ?? 0;

  return (
    <div className="panel" style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      <h3 style={{ margin: '0 0 8px' }}>
        Skill Agents
        {currentEntry && <span style={{ fontSize: 11, color: '#6ee7a0', marginLeft: 8 }}>● {currentEntry.skill_name} running</span>}
        {queuedCount > 0 && <span style={{ fontSize: 11, color: '#ffb347', marginLeft: 8 }}>{queuedCount} queued</span>}
        {queueState?.gpu_lock_held && <span style={{ fontSize: 10, color: '#888', marginLeft: 8 }}>GPU locked</span>}
      </h3>

      {/* GPU serial lock explanation */}
      <div className="small" style={{ marginBottom: 8, color: '#666', fontSize: 11 }}>
        Skill agents run sequentially — only one Qwen3 instance holds the GPU at a time. Cloud skills (GPT-4o, o3, Claude) bypass the GPU lock and run via API. Use hand_off to chain agents.
      </div>

      {/* Tab nav */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 10 }}>
        {(['skills', 'queue', 'history'] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ background: tab === t ? '#2a2a2a' : 'none', border: '1px solid ' + (tab === t ? '#555' : '#333'), color: tab === t ? '#ccc' : '#777', borderRadius: 4, padding: '3px 10px', cursor: 'pointer', fontSize: 11, fontWeight: tab === t ? 600 : 400 }}>
            {t === 'queue' ? `Queue (${(queueState?.queue.length ?? 0) + (currentEntry ? 1 : 0)})` : t === 'history' ? `History (${queueState?.history.length ?? 0})` : 'Skills'}
          </button>
        ))}
      </div>

      {message && (
        <div style={{ background: '#1a2a1a', color: '#6ee7a0', border: '1px solid #2a4a2a', borderRadius: 4, padding: '6px 10px', marginBottom: 8, fontSize: 11 }}>{message}</div>
      )}

      <div style={{ flex: 1, overflowY: 'auto' }}>
        {/* SKILLS TAB */}
        {tab === 'skills' && (
          <>
            {editingSkill ? (
              <>
                <SkillEditor skill={editingSkill} onSave={handleSaveSkill} onCancel={() => setEditingSkill(null)} />
                <button
                  onClick={() => handleDeleteSkill(editingSkill.id)}
                  style={{ background: '#3d1a1a', color: '#ff7979', border: 'none', borderRadius: 4, padding: '5px 12px', cursor: 'pointer', fontSize: 11, marginBottom: 12 }}
                >
                  Delete This Skill
                </button>
              </>
            ) : (
              <>
                <div style={{ display: 'flex', gap: 6, marginBottom: 10 }}>
                  <input
                    value={filter}
                    onChange={e => setFilter(e.target.value)}
                    placeholder="Filter skills..."
                    style={{ flex: 1, background: '#111', color: '#ccc', border: '1px solid #444', borderRadius: 4, padding: '4px 8px', fontSize: 11 }}
                  />
                  <button onClick={handleResetToDefaults} style={{ background: '#222', color: '#999', border: '1px solid #444', borderRadius: 4, padding: '4px 10px', cursor: 'pointer', fontSize: 11 }}>
                    Reset Defaults
                  </button>
                </div>
                {filteredSkills.map(skill => (
                  <SkillCard key={skill.id} skill={skill} onSpawn={handleSpawn} onEdit={setEditingSkill} />
                ))}
                {filteredSkills.length === 0 && (
                  <div style={{ color: '#666', fontSize: 12, textAlign: 'center', marginTop: 20 }}>No skills match your filter.</div>
                )}
              </>
            )}
          </>
        )}

        {/* QUEUE TAB */}
        {tab === 'queue' && (
          <>
            <div style={{ display: 'flex', gap: 8, marginBottom: 10 }}>
              <button onClick={handleAdvance} disabled={!!currentEntry || queuedCount === 0}
                style={{ background: '#1e3a5f', color: '#7ec8e3', border: 'none', borderRadius: 4, padding: '5px 12px', cursor: 'pointer', fontSize: 12, opacity: (!!currentEntry || queuedCount === 0) ? 0.4 : 1 }}>
                ▶ Start Next Skill
              </button>
              <button onClick={handleCancelAll} disabled={!currentEntry && queuedCount === 0}
                style={{ background: '#3d1a1a', color: '#ff7979', border: 'none', borderRadius: 4, padding: '5px 12px', cursor: 'pointer', fontSize: 12, opacity: (!currentEntry && queuedCount === 0) ? 0.4 : 1 }}>
                ✕ Cancel All
              </button>
            </div>
            {currentEntry && (
              <div style={{ marginBottom: 10 }}>
                <div style={{ fontSize: 11, color: '#6ee7a0', fontWeight: 600, marginBottom: 4 }}>Currently Running</div>
                <QueueEntryRow entry={currentEntry} />
              </div>
            )}
            {queueState?.queue.length === 0 && !currentEntry && (
              <div style={{ color: '#666', fontSize: 12, textAlign: 'center', marginTop: 20 }}>Queue is empty. Spawn a skill agent from the Skills tab.</div>
            )}
            {queueState?.queue.map(entry => <QueueEntryRow key={entry.id} entry={entry} />)}
          </>
        )}

        {/* HISTORY TAB */}
        {tab === 'history' && (
          <>
            {(!queueState?.history || queueState.history.length === 0) && (
              <div style={{ color: '#666', fontSize: 12, textAlign: 'center', marginTop: 20 }}>No skill agents have completed yet.</div>
            )}
            {queueState?.history.map(entry => <QueueEntryRow key={entry.id} entry={entry} />)}
          </>
        )}
      </div>
    </div>
  );
}
