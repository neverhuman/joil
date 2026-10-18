import unittest

from tools.v5_private_dossier import (
    PUBLIC_DOSSIER_ALLOWLIST,
    verify_private_dossier_and_public_export_gate,
)


class PrivateDossierExportTests(unittest.TestCase):
    def setUp(self):
        self.private = {
            "classification": "private",
            "industrial_installation_authorized": False,
            "private_fields": {"technical_record": "held privately"},
        }
        self.review = {
            "approved": True,
            "reviewer": "human reviewer",
            "reviewed_fields": {name: "reviewed synthetic summary" for name in PUBLIC_DOSSIER_ALLOWLIST},
        }
        self.review["reviewed_fields"]["subject_sha256"] = "a" * 64

    def test_approved_exact_allowlist_exports_only_reviewed_fields(self):
        exported = verify_private_dossier_and_public_export_gate(self.private, self.review)
        self.assertEqual(set(exported), PUBLIC_DOSSIER_ALLOWLIST)
        self.assertEqual(exported["subject_sha256"], "a" * 64)

    def test_sensitive_tokens_and_unapproved_reviews_are_rejected(self):
        self.review["reviewed_fields"]["result_summary"] = "customer site_id 17"
        with self.assertRaisesRegex(ValueError, "sensitive_identity_token"):
            verify_private_dossier_and_public_export_gate(self.private, self.review)
        self.review["reviewed_fields"]["result_summary"] = "reviewed summary"
        self.review["approved"] = False
        with self.assertRaisesRegex(ValueError, "human_review_required"):
            verify_private_dossier_and_public_export_gate(self.private, self.review)

    def test_installation_authorization_is_never_inferred(self):
        self.private["industrial_installation_authorized"] = True
        with self.assertRaisesRegex(ValueError, "cannot_authorize_installation"):
            verify_private_dossier_and_public_export_gate(self.private, self.review)


if __name__ == "__main__":
    unittest.main()
