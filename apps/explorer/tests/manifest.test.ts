import test from 'node:test';
import assert from 'node:assert/strict';
import { applyRevocation, isManifest, resolveIdentity } from '../src/contracts.ts';

const subject = { id: 's1', geometry_sha256: 'a'.repeat(64), geometry_source: null, ancestry: [], values: [], overlays: [] };
const manifest = {
  schema_version: 1, revision: 'rev-a', generated_at: '2026-10-18T00:00:00Z', current: true,
  campaign: { tasks_total: 10, tasks_completed: 9, attempts: null, spend: null, reservations: null, checkpoint: null, control_arms: [], blockers: [], admitted_capabilities: [], software_status: 'software complete', scientific_release: 'not released' },
  subjects: [subject], values: [{ id: 'claim-a', label: 'Pressure', value: null, unit: 'Pa', uncertainty: null, tier: 'unavailable', source: null, status: 'missing' }], archive: [], historical_corrections: [],
};

test('an exact revision and subject resolve; a stale URL never selects another subject', () => {
  assert.equal(resolveIdentity(manifest as never, 'rev-a', 's1')?.id, 's1');
  assert.equal(resolveIdentity(manifest as never, 'rev-old', 's1'), undefined);
  assert.equal(resolveIdentity(manifest as never, 'rev-a', 'missing'), undefined);
  assert.equal(resolveIdentity(manifest as never, null, 's1'), undefined);
});

test('strict manifest view preserves absent values and rejects wrong geometry overlays', () => {
  assert.equal(isManifest(manifest), true);
  assert.equal((manifest.values[0] as { value: null }).value, null);
  const bad = structuredClone(manifest);
  bad.subjects[0].overlays.push({ id: 'wrong', field: 'pressure', time_s: 1, geometry_sha256: 'b'.repeat(64), source: 'field.vtu' });
  assert.equal(isManifest(bad), false);
  assert.equal(isManifest({ ...manifest, schema_version: 2 }), false);
});

test('registry revocation invalidates current eligibility in live subject and root values', () => {
  const m = structuredClone(manifest) as never;
  const value = { id: 'claim-a', label: 'Pressure', value: 4, unit: 'Pa', uncertainty: '±1 Pa', tier: 'unvalidated_model', source: './source.json', status: 'available' };
  const complete = { ...(m as object), values: [value], subjects: [{ ...subject, values: [value] }] } as Parameters<typeof applyRevocation>[0];
  const revised = applyRevocation(complete, 7, ['claim-a']);
  assert.equal(revised.current, false);
  assert.equal(revised.values[0].status, 'revoked');
  assert.equal(revised.subjects[0].values[0].status, 'revoked');
  assert.deepEqual(revised.registry_event?.revoked_claim_ids, ['claim-a']);
});
