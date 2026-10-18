import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import test from 'node:test';
import assert from 'node:assert/strict';
import { applyRevocation, mergeRefreshedManifest, isManifest, resolveIdentity } from '../src/contracts.ts';

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

test('g009 missing status text and duplicate subject identities are rejected', () => {
  const missing=structuredClone(manifest); (missing.campaign as unknown as {scientific_release:null}).scientific_release=null;
  assert.equal(isManifest(missing),false);
  const duplicate=structuredClone(manifest);duplicate.subjects.push(structuredClone(subject));
  assert.equal(isManifest(duplicate),false);
});
test('g009 invalid event sequence is rejected and revocation frontier never moves backward', () => {
  assert.equal(isManifest({...manifest,registry_event:{sequence:NaN,revoked_claim_ids:[]}}),false);
  const first=applyRevocation(structuredClone(manifest) as never,8,['a']);
  const second=applyRevocation(first,3,['b']);
  assert.equal(second.registry_event?.sequence,8);
  assert.deepEqual(new Set(second.registry_event?.revoked_claim_ids),new Set(['a','b']));
});

test('g009 duplicate subjects cannot silently resolve to the first entry', () => {
  const duplicate=structuredClone(manifest);duplicate.subjects.push(structuredClone(subject));
  assert.equal(isManifest(duplicate),false);
});
test('g009 an older revocation event cannot reset the stored frontier', () => {
  const first=applyRevocation(structuredClone(manifest) as never,8,['a']);
  const second=applyRevocation(first,3,['b']);
  assert.equal(second.registry_event?.sequence,8);
  assert.deepEqual(new Set(second.registry_event?.revoked_claim_ids),new Set(['a','b']));
});

test('g009 stale refresh and early events retain revocation authority', () => {
  const raw=structuredClone(manifest) as never;
  const event={sequence:8,revoked_claim_ids:['claim-a']};
  const initial=mergeRefreshedManifest(null,raw,event);
  assert.equal(initial.current,false);assert.equal(initial.values[0].status,'revoked');
  const refreshed=mergeRefreshedManifest(initial,raw);
  assert.equal(refreshed.current,false);assert.equal(refreshed.values[0].status,'revoked');
});

test('g009 default software view has exact source bytes and no invented scientific data', () => {
  const root=new URL('../public/',import.meta.url);
  const raw=JSON.parse(readFileSync(new URL('evidence-manifest.json',root),'utf8'));
  assert.equal(isManifest(raw),true);
  const source=readFileSync(new URL(raw.source_snapshot.source,root));
  assert.equal(createHash('sha256').update(source).digest('hex'),raw.revision);
  assert.deepEqual(raw.values,[]);assert.deepEqual(raw.subjects,[]);
  assert.equal(raw.campaign.attempts,null);assert.equal(raw.campaign.spend,null);
});
