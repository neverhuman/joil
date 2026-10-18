"""Deterministic, fixture-grade tools for prospective Joil confirmation.

These routines validate records and perform declared arithmetic. They do not
admit a physical study or substitute for the canonical evidence registry.
"""

from .protocol import (
    AnalysisError,
    analyze_confirmation,
    build_private_dossier,
    export_reviewed_public,
    reduce_measurements,
    required_channels,
    sample_size_from_pilot,
    store_private_dossier,
    validate_independent_pairs,
    validate_holdout_partition,
    validate_specimen_passport,
)

__all__ = [
    "AnalysisError", "analyze_confirmation", "build_private_dossier",
    "export_reviewed_public", "reduce_measurements", "required_channels",
    "sample_size_from_pilot", "store_private_dossier", "validate_independent_pairs",
    "validate_holdout_partition", "validate_specimen_passport",
]
