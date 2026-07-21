export const STUDIO_TABS = ['overview', 'intent', 'dreams', 'ux', 'architecture', 'plan', 'team', 'pulse', 'evidence', 'changes'];

export function selectProductMode(current, requested) {
  return ['assist', 'studio', 'dream'].includes(requested) ? requested : current;
}

export function applyBackendSnapshot(state, backendSnapshot) {
  return { ...state, loading: false, error: null, snapshot: backendSnapshot };
}

export function applyPrototypeInteraction(state, interaction) {
  if (!interaction || typeof interaction.key !== 'string' || !(interaction.key in state)) return state;
  switch (interaction.action) {
    case 'set_state': return { ...state, [interaction.key]: interaction.value };
    case 'toggle_state': return { ...state, [interaction.key]: !Boolean(state[interaction.key]) };
    case 'open_modal': return { ...state, [interaction.key]: true };
    case 'close_modal': return { ...state, [interaction.key]: false };
    default: return state;
  }
}

export function filterTimeline(events, filter) {
  const normalized = String(filter ?? '').trim().toLowerCase();
  if (!normalized) return events;
  return events.filter((event) => JSON.stringify(event).toLowerCase().includes(normalized));
}

export function summarizeEvidence(proof) {
  const complete = Array.isArray(proof?.complete) ? proof.complete.length : 0;
  const incomplete = ['incomplete', 'blocked', 'unverified']
    .flatMap((key) => Array.isArray(proof?.[key]) ? proof[key] : []).length;
  return { complete, incomplete, verified: incomplete === 0 && complete > 0 };
}

export function pulseSourceLabel(finding) {
  return finding?.experimental ? 'Experimental · shadow only' : 'Deterministic · evidence-backed';
}

