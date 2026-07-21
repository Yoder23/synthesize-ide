import { useMemo, useState } from 'react';
import { applyPrototypeInteraction } from './studioModel.mjs';

type PrototypeValue = string | number | boolean;
type Interaction = { action: 'set_state' | 'toggle_state' | 'open_modal' | 'close_modal'; key: string; value?: PrototypeValue };
type Node = Record<string, unknown> & { type: string; id: string };
type Document = { schemaVersion?: number; schema_version?: number; title: string; initialState?: Record<string, PrototypeValue>; initial_state?: Record<string, PrototypeValue>; root: Node };

const ALLOWED_TYPES = new Set([
  'layout', 'stack', 'split_pane', 'tabs', 'card', 'text', 'status_badge', 'progress_indicator',
  'button', 'form_field', 'table', 'timeline', 'graph_placeholder', 'diff_placeholder',
  'code_placeholder', 'modal', 'callout'
]);

export function DeclarativePrototypeRenderer({ document }: { document: Document }) {
  const initial = document.initialState ?? document.initial_state ?? {};
  const [state, setState] = useState<Record<string, unknown>>(() => ({ ...initial }));
  const safe = useMemo(() => validateDocument(document), [document]);
  if (!safe.ok) return <div className="studio-empty error-text">Prototype rejected: {safe.error}</div>;

  function interact(interaction: unknown) {
    if (!isInteraction(interaction)) return;
    setState((current) => applyPrototypeInteraction(current, interaction));
  }

  return (
    <section className="prototype-shell" aria-label={`Declarative prototype: ${document.title}`}>
      <div className="prototype-banner">Trusted declarative renderer · fake local state only</div>
      {renderNode(document.root, state, interact)}
    </section>
  );
}

function renderNode(node: Node, state: Record<string, unknown>, interact: (interaction: unknown) => void): React.ReactNode {
  const children = childNodes(node);
  switch (node.type) {
    case 'layout':
      return <div key={node.id} className={`prototype-layout ${node.direction === 'row' ? 'row-direction' : ''}`}>{children.map((child) => renderNode(child, state, interact))}</div>;
    case 'stack':
    case 'split_pane':
      return <div key={node.id} className={`prototype-${node.type}`}>{children.map((child) => renderNode(child, state, interact))}</div>;
    case 'tabs':
      return <div key={node.id} className="prototype-tabs">{asArray(node.tabs).map((tab, index) => {
        const record = asRecord(tab);
        return <section key={asText(record.id, `tab-${index}`)}><h5>{asText(record.label, 'Tab')}</h5>{asArray(record.children).filter(isNode).map((child) => renderNode(child, state, interact))}</section>;
      })}</div>;
    case 'card':
      return <article key={node.id} className="prototype-card"><h5>{asText(node.title, 'Card')}</h5>{children.map((child) => renderNode(child, state, interact))}</article>;
    case 'text': return <p key={node.id}>{asText(node.text, '')}</p>;
    case 'status_badge': return <span key={node.id} className="prototype-badge">{asText(node.label, 'Status')}</span>;
    case 'progress_indicator': {
      const value = Math.max(0, Math.min(1, Number(node.value) || 0));
      return <label key={node.id} className="prototype-progress"><span>{asText(node.label, 'Progress')}</span><progress value={value} max={1} /><span>{Math.round(value * 100)}%</span></label>;
    }
    case 'button': return <button key={node.id} type="button" onClick={() => interact(node.interaction)}>{asText(node.label, 'Action')}</button>;
    case 'form_field': {
      const key = asText(node.stateKey ?? node.state_key, '');
      return <label key={node.id}>{asText(node.label, 'Field')}<input value={String(state[key] ?? '')} onChange={(event) => interact({ action: 'set_state', key, value: event.target.value })} /></label>;
    }
    case 'table':
      return <table key={node.id}><thead><tr>{asArray(node.columns).map((cell, index) => <th key={index}>{String(cell)}</th>)}</tr></thead><tbody>{asArray(node.rows).map((row, index) => <tr key={index}>{asArray(row).map((cell, cellIndex) => <td key={cellIndex}>{String(cell)}</td>)}</tr>)}</tbody></table>;
    case 'timeline': return <ol key={node.id}>{asArray(node.items).map((item, index) => <li key={index}>{String(item)}</li>)}</ol>;
    case 'graph_placeholder':
    case 'diff_placeholder':
    case 'code_placeholder': return <pre key={node.id} className="prototype-placeholder">{asText(node.label ?? node.code, node.type)}</pre>;
    case 'modal': {
      const key = asText(node.openStateKey ?? node.open_state_key, '');
      return state[key] ? <div key={node.id} className="prototype-modal" role="dialog" aria-modal="true"><h5>{asText(node.title, 'Dialog')}</h5>{children.map((child) => renderNode(child, state, interact))}</div> : null;
    }
    case 'callout': return <aside key={node.id} className="prototype-callout">{asText(node.text, '')}</aside>;
    default: return null;
  }
}

function validateDocument(document: Document): { ok: true } | { ok: false; error: string } {
  const version = document.schemaVersion ?? document.schema_version;
  if (version !== 1) return { ok: false, error: 'unsupported schema version' };
  let count = 0;
  const ids = new Set<string>();
  function visit(node: Node, depth: number): string | null {
    count += 1;
    if (count > 500 || depth > 12) return 'structure limit exceeded';
    if (!ALLOWED_TYPES.has(node.type)) return `component type ${node.type} is not allowlisted`;
    if (!/^[A-Za-z0-9_.-]{1,100}$/.test(node.id) || ids.has(node.id)) return `invalid or duplicate node ${node.id}`;
    ids.add(node.id);
    const serialized = JSON.stringify(node).toLowerCase();
    if (['<script', 'javascript:', 'window.__tauri', 'localstorage', 'fetch(', 'xmlhttprequest'].some((needle) => serialized.includes(needle))) return 'privileged or executable content is forbidden';
    for (const child of childNodes(node)) {
      const error = visit(child, depth + 1);
      if (error) return error;
    }
    return null;
  }
  const error = visit(document.root, 0);
  return error ? { ok: false, error } : { ok: true };
}

function childNodes(node: Node): Node[] {
  return asArray(node.children).filter(isNode);
}

function isNode(value: unknown): value is Node {
  const record = asRecord(value);
  return typeof record.type === 'string' && typeof record.id === 'string';
}

function isInteraction(value: unknown): value is Interaction {
  const record = asRecord(value);
  return ['set_state', 'toggle_state', 'open_modal', 'close_modal'].includes(String(record.action)) && typeof record.key === 'string';
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function asText(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback;
}

