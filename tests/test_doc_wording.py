"""DOC-08: strong evidence words need a tier or limitation; descriptor-backed claims need a prior label."""
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from tools.claims_audit import _descriptor_names, _evidence_tiers, audit_claim_wording

TIERS = _evidence_tiers(ROOT)
DESCRIPTORS = _descriptor_names(ROOT)


def flagged(text):
    errors, _ = audit_claim_wording({'docs/WHITEPAPER.md': text}, TIERS, DESCRIPTORS)
    return errors


class ClaimWordingTests(unittest.TestCase):
    def test_vocabularies_come_from_code(self):
        self.assertIn('physical_validation', TIERS)
        self.assertIn('numerically_verified', TIERS)
        self.assertIn('shear_mitigation_factor', DESCRIPTORS)

    def test_planted_strong_claims_fail(self):
        for bad in ('The promoted TPMS elite is qualified for service.',
                    'The probability of compliance is 0.98 for every duty.',
                    'This study proves that evolution discovers better separators.',
                    'The 6-jet design was validated on three grids.'):
            with self.subTest(bad=bad):
                self.assertEqual(len(flagged(bad)), 1, bad)

    def test_planted_descriptor_backed_claim_fails(self):
        errors = flagged('The ribbon cascade reduces entry shear because its shear_mitigation_factor is lower.')
        self.assertTrue(any('gene-derived descriptors' in e for e in errors), errors)

    def test_tiered_negated_or_cited_wording_passes(self):
        for ok in ('The `F2-carrier` solution is verified on three grids.',
                   'This design is not qualified, and physical validation remains open.',
                   'Images still show "Qualification" from the old generator.',
                   'The receipt records `qualified_fitness: null`.',
                   'The screen credits TPMS genes through shear_mitigation_factor, a gene-derived prior, which reduces the score.',
                   'See the physics and qualification contract.'):
            with self.subTest(ok=ok):
                self.assertEqual(flagged(ok), [], ok)

    def test_wrapped_paragraph_is_one_sentence(self):
        wrapped = 'The `F1` screen numbers are\nverified only for arithmetic consistency.'
        self.assertEqual(flagged(wrapped), [])
        self.assertEqual(len(flagged('The design is\nverified.')), 1)


if __name__ == '__main__':
    unittest.main()
