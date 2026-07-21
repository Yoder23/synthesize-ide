import test from 'node:test';
import assert from 'node:assert/strict';
import {
  applyBackendSnapshot,
  applyPrototypeInteraction,
  filterTimeline,
  pulseSourceLabel,
  selectProductMode,
  summarizeEvidence
} from './studioModel.mjs';

test('mode switching accepts only trusted product modes', () => {
  assert.equal(selectProductMode('assist', 'studio'), 'studio');
  assert.equal(selectProductMode('studio', 'dream'), 'dream');
  assert.equal(selectProductMode('dream', 'admin'), 'dream');
});

test('backend snapshot replaces optimistic lifecycle state', () => {
  const result = applyBackendSnapshot(
    { loading: true, error: 'old', snapshot: { initiative: { status: 'implementing' } } },
    { initiative: { status: 'blocked' } }
  );
  assert.equal(result.snapshot.initiative.status, 'blocked');
  assert.equal(result.loading, false);
  assert.equal(result.error, null);
});

test('prototype interactions remain local and reject unknown keys/actions', () => {
  const initial = { detailsOpen: false };
  assert.deepEqual(applyPrototypeInteraction(initial, { action: 'toggle_state', key: 'detailsOpen' }), { detailsOpen: true });
  assert.equal(applyPrototypeInteraction(initial, { action: 'invoke', key: 'detailsOpen' }), initial);
  assert.equal(applyPrototypeInteraction(initial, { action: 'set_state', key: 'repoRoot', value: '/' }), initial);
});

test('team filtering, evidence status, and Pulse labels are explicit', () => {
  const events = [{ role: 'builder', kind: 'patch' }, { role: 'verifier', kind: 'evidence' }];
  assert.deepEqual(filterTimeline(events, 'VERIFIER'), [events[1]]);
  assert.deepEqual(summarizeEvidence({ complete: ['REQ-1'], incomplete: [], blocked: [], unverified: [] }), { complete: 1, incomplete: 0, verified: true });
  assert.equal(pulseSourceLabel({ experimental: true }), 'Experimental · shadow only');
  assert.equal(pulseSourceLabel({ experimental: false }), 'Deterministic · evidence-backed');
});

