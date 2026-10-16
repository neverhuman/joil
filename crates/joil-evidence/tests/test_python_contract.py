"""Synthetic mutations of frozen contracts, never physical validation."""
import copy
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from analysis.evidence import strict_json, scientific_hash, EvidenceTier
from tools.gen_status import check_generated, generate, objective_contract


class ContractTests(unittest.TestCase):
    def fixture(self):
        private = ROOT/'.private'
        private.mkdir(exist_ok=True)
        temporary = tempfile.TemporaryDirectory(dir=private, prefix='ev-contract-')
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        names = ['requirements/joil-hydrocyclone-v1.toml', 'experiments/wave2-preregistration-v1.json',
                 'baselines/ct35/genome.json', 'crates/joil-evidence/registry/status-registry.json',
                 'crates/joil-evidence/registry/legacy-status-before-v4.json',
                 'crates/joil-evidence/receipts/EV-01-05.json', 'docs/STATUS.json']
        registry = json.loads((ROOT/'crates/joil-evidence/registry/status-registry.json').read_text())
        names.extend(entry['receipt']['path'] for entry in registry['software'])
        for name in names:
            destination = root/name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT/name, destination)
        return root

    def test_generated_status_rejects_hand_edits_and_stale_evidence(self):
        root = self.fixture()
        check_generated(root)
        status = generate(root)
        self.assertFalse(status['physical_validation'])
        self.assertFalse(status['structural_qualification'])
        status['physical_validation'] = True
        (root/'docs/STATUS.json').write_text(json.dumps(status, indent=2)+'\n')
        with self.assertRaisesRegex(ValueError, 'stale or hand-edited'):
            check_generated(root)
        receipt = root/'crates/joil-evidence/receipts/EV-01-05.json'
        receipt.write_text(receipt.read_text()+' ')
        with self.assertRaisesRegex(ValueError, 'stale or changed evidence'):
            generate(root)

    def test_objective_and_campaign_drift_fail_closed(self):
        root = self.fixture()
        _, objectives, digest = objective_contract(root)
        campaign = root/'runs/synthetic/campaign.json'
        campaign.parent.mkdir(parents=True)
        receipt = {'schema': 'joil.campaign.v1', 'objective_sha256': digest,
                   'executed_objectives': objectives}
        campaign.write_text(json.dumps(receipt))
        generate(root)
        receipt['objective_sha256'] = '0'*64
        campaign.write_text(json.dumps(receipt))
        with self.assertRaisesRegex(ValueError, 'objective hash mismatch'):
            generate(root)
        requirement = root/'requirements/joil-hydrocyclone-v1.toml'
        requirement.write_text(requirement.read_text().replace('id = "cout_p95_mg_l"', 'id = "entry_shear"', 1))
        with self.assertRaisesRegex(ValueError, 'objective IDs/order'):
            objective_contract(root)

    def test_strict_numeric_wire_and_typed_tiers(self):
        self.assertNotEqual(scientific_hash(0.0), scientific_hash(-0.0))
        self.assertEqual(EvidenceTier('numerically_verified'), EvidenceTier.NUMERICALLY_VERIFIED)
        with self.assertRaises(ValueError):
            EvidenceTier('qualified')
        for raw in ['{"a":1,"a":2}', '1e999', '1e-999', 'NaN', '{"é":0,"e\\u0301":1}']:
            with self.assertRaises(ValueError):
                strict_json(raw)

    def test_requirements_cannot_self_upgrade_with_missing_limits(self):
        root = self.fixture()
        requirement = root/'requirements/joil-hydrocyclone-v1.toml'
        original = requirement.read_text()
        requirement.write_text(original.replace('admission = "sandbox_only"', 'admission = "admitted"'))
        with self.assertRaisesRegex(ValueError, 'cannot admit unresolved'):
            generate(root)
        requirement.write_text(original.replace('min_n_p05_p95 = 72', 'min_n_p05_p95 = 3'))
        with self.assertRaisesRegex(ValueError, 'weakened tail'):
            generate(root)


if __name__ == '__main__':
    unittest.main()
