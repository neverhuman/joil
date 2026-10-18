"""Software-only private dossier export gate; this does not grant installation authority."""

from __future__ import annotations

PUBLIC_DOSSIER_ALLOWLIST = frozenset(
    {
        "subject_sha256",
        "duty_summary",
        "method_summary",
        "software_version",
        "uncertainty_summary",
        "result_summary",
        "limitations",
    }
)
_SENSITIVE_TOKENS = (
    "customer",
    "facility",
    "site_id",
    "plant_id",
    "account_id",
    "address",
    "contact",
    "serial_number",
)


def verify_private_dossier_and_public_export_gate(
    private: dict, review: dict
) -> dict[str, str]:
    """Return the approved public projection, or raise ValueError on any policy violation."""
    if private.get("classification") != "private":
        raise ValueError("dossier_classification_must_be_private")
    if private.get("industrial_installation_authorized") is not False:
        raise ValueError("software_export_gate_cannot_authorize_installation")
    private_fields = private.get("private_fields")
    if not isinstance(private_fields, dict) or not private_fields:
        raise ValueError("private_dossier_content_required")
    if review.get("approved") is not True or not str(review.get("reviewer", "")).strip():
        raise ValueError("approved_human_review_required")
    fields = review.get("reviewed_fields")
    if not isinstance(fields, dict) or set(fields) != PUBLIC_DOSSIER_ALLOWLIST:
        raise ValueError("public_export_fields_must_match_exact_allowlist")
    public: dict[str, str] = {}
    for key, value in fields.items():
        if not isinstance(value, str) or not value.strip():
            raise ValueError("public_export_values_must_be_nonempty")
        normalized = "".join(character for character in f"{key}{value}".lower() if character.isalnum())
        if any("".join(character for character in token if character.isalnum()) in normalized for token in _SENSITIVE_TOKENS):
            raise ValueError("public_export_contains_sensitive_identity_token")
        public[key] = value
    return public
