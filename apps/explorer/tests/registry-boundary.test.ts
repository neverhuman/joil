import { test } from 'node:test';
import assert from 'node:assert/strict';
import { isRegistryEvent } from '../src/contracts.ts';
test('registry boundary rejects malformed sequences and revoked IDs', () => {
  for (const value of [null, [], {}, {sequence: '1', revoked_claim_ids: []},
    ...[-1, 1.5, Infinity, NaN, Number.MAX_SAFE_INTEGER + 1].map(sequence => ({sequence, revoked_claim_ids: []})),
    {sequence: 0, revoked_claim_ids: [42]}, {sequence: 0, revoked_claim_ids: 'claim'}]) {
    assert.equal(isRegistryEvent(value), false);
  }
  assert.equal(isRegistryEvent({sequence: 0, revoked_claim_ids: []}), true);
  assert.equal(isRegistryEvent({sequence: Number.MAX_SAFE_INTEGER, revoked_claim_ids: ['claim']}), true);
});
