#!/usr/bin/env python3
"""Freeze structural interchange and generate TS. Rust enforces semantics/reviews.

This generator uses only Python's standard library. --check is used by the
dedicated architecture package; independent JSON Schema validation is separate.
"""
import argparse
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def ref(name):
    return {"$ref": f"#/$defs/{name}"}


def obj(**fields):
    return {"type": "object", "properties": fields, "required": list(fields),
            "additionalProperties": False}


def enum(*values):
    return {"enum": list(values)}


def array(item, count=None):
    result = {"type": "array", "items": item}
    if count is not None:
        result.update(minItems=count, maxItems=count)
    return result


def tagged(tag, variants):
    return {"oneOf": [obj(**{tag: {"const": name}}, **fields)
                      for name, fields in variants.items()]}


def nullable(schema):
    return {"oneOf": [schema, {"type": "null"}]}


def definitions():
    number = {"type": "number"}
    integer = {"type": "integer", "minimum": 0, "maximum": 2**53}
    text = {"type": "string"}
    digest = {"type": "string", "pattern": "^[0-9a-f]{64}$"}
    frame = enum("right_handed_si_z_downstream", "right_handed_si_z_up")
    unit = enum("metre", "millimetre")
    fmt = enum("triangle_surface_json", "step", "native_volume_mesh", "native_solver_case")
    d = {"BlobDigest": digest, "SemanticId": digest,
         "ContentRef": obj(id=text, revision=text, sha256=digest),
         "CoordinateFrame": frame,
         "FrameTransform": obj(**{"from": frame, "to": frame,
                                   "translation_m": array(number, 3),
                                   "rotation": array(array(number, 3), 3)}),
         "ArtifactRef": obj(role=enum("geometry", "surface_mesh", "volume_mesh", "solver_case"),
                            format=fmt, blob=ref("BlobDigest"), bytes=integer,
                            length_unit=unit, approximation_bound_m=number,
                            acceptance=ref("BlobDigest")),
         "Port": obj(tag={"type": "integer", "minimum": 0, "maximum": 65535},
                     purpose=enum("inlet", "outlet", "support", "fixed_interface"),
                     frame=frame, centre_m=array(number, 3), area_m2=number,
                     outward_normal=array(number, 3)),
         "FrozenRegion": obj(tag={"type": "integer", "minimum": 0, "maximum": 65535},
                             boundary=ref("BlobDigest")),
         "MaterialState": tagged("status", {"unknown": {}, "identified": {"passport": ref("BlobDigest")}}),
         "RoughnessState": tagged("status", {"unknown": {}, "characterized": {"sand_grain_m": number, "source": ref("BlobDigest")}}),
         "Calibration": tagged("status", {"uncalibrated": {}, "bound": {"record": ref("BlobDigest")}})}
    d["SubjectBundle"] = obj(schema_version={"const": 1}, artifacts=array(ref("ArtifactRef")),
                             transform=ref("FrameTransform"), material=ref("MaterialState"),
                             roughness=ref("RoughnessState"), interface_contract=ref("BlobDigest"),
                             ports=array(ref("Port")), frozen_regions=array(ref("FrozenRegion")))
    d["TriangleSurface"] = obj(vertices_m=array(array(number, 3)),
                                triangles=array(array({"type": "integer", "minimum": 0, "maximum": 2**32-1}, 3)))
    d["EvaluationKey"] = obj(schema_version={"const": 1}, subject=ref("SubjectBundle"),
        **{k: ref("BlobDigest") for k in ("manufacturing_state", "scenario", "solver_build", "container", "numerics", "mesh_policy", "sampling_design", "randomness", "extractor")},
        calibration=ref("Calibration"))
    d["DecisionKey"] = obj(observation=ref("SemanticId"), requirement=ref("BlobDigest"),
                            decision_profile=ref("BlobDigest"), preference_policy=ref("BlobDigest"))
    d["Quantity"] = obj(value=number, unit=enum("metre", "millimetre", "second", "kilogram", "cubic_metre", "cubic_metre_per_second", "pascal", "kilopascal", "kilogram_per_cubic_metre", "mg_per_litre_dispersed_oil", "dimensionless", "mechanical_loss_watt", "treated_cubic_metre_per_second", "joule_per_treated_cubic_metre"))
    variants = {"constant": {"quantity": ref("Quantity")}, "parameter": {"name": text},
                **{op: {"left": ref("Expr"), "right": ref("Expr")}
                   for op in ("add", "subtract", "multiply", "divide")},
                "negate": {"value": ref("Expr")},
                "concentration_to_volume_fraction": {"concentration": ref("Expr"), "oil_density": ref("Expr")},
                "energy_per_treated_volume": {"mechanical_loss": ref("Expr"), "treated_flow": ref("Expr")}}
    d["Expr"] = tagged("op", variants)
    d["TaskOutcome"] = tagged("state", {"succeeded": {"output_manifest": nullable(ref("BlobDigest"))},
        "numerical_failure": {"diagnostic": nullable(ref("BlobDigest"))},
        "infrastructure_failure": {"diagnostic": nullable(ref("BlobDigest"))}, "unsupported": {}, "not_run": {}})
    d["ObservationState"] = tagged("state", {"measured": {"claim": ref("ContentRef")}, "missing": {},
        "invalid": {}, "out_of_scope": {}, "not_applicable": {"rule": ref("ContentRef")}})
    d["NonPassingReason"] = enum("not_run", "numerical_failure", "infrastructure_failure", "unsupported", "absent_output", "missing_observation", "invalid_observation", "out_of_scope", "unregistered_not_applicable", "source_mismatch", "unreviewed_policy", "incomplete_evidence", "invalid_evidence", "reopened_evidence", "revoked_evidence")
    d["GateStateReport"] = tagged("state", {"pass": {"claim": ref("ContentRef")}, "fail": {"claim": ref("ContentRef")},
        "unknown": {"reason": ref("NonPassingReason")}, "not_applicable": {"rule": ref("ContentRef")}})
    d["GateDecisionReport"] = obj(state=ref("GateStateReport"), policy=ref("ContentRef"),
                                   requirements=ref("ContentRef"), gate_id=text)
    d["EligibilityStateReport"] = tagged("state", {"blocked": {"gates": array(text)},
        "ready_for_evidence_review": {"claims": array(ref("ContentRef"))}})
    d["ClaimEligibilityReport"] = obj(state=ref("EligibilityStateReport"))
    return d


def ts(schema):
    if "$ref" in schema:
        return schema["$ref"].split("/")[-1]
    if "const" in schema:
        return json.dumps(schema["const"])
    if "enum" in schema:
        return " | ".join(map(json.dumps, schema["enum"]))
    if "oneOf" in schema:
        return "(" + " | ".join(map(ts, schema["oneOf"])) + ")"
    kind = schema["type"]
    if kind == "object":
        return "{ " + "; ".join(f"readonly {json.dumps(k)}: {ts(v)}" for k, v in schema["properties"].items()) + " }"
    if kind == "array":
        if schema.get("minItems") == schema.get("maxItems") and "minItems" in schema:
            return "readonly [" + ", ".join([ts(schema["items"])] * schema["minItems"]) + "]"
        return "ReadonlyArray<" + ts(schema["items"]) + ">"
    return {"number": "number", "integer": "number", "string": "string", "null": "null", "boolean": "boolean"}[kind]


def outputs():
    defs = definitions()
    schema = {"$schema": "https://json-schema.org/draft/2020-12/schema", "$id": "urn:joil:v5:physical-contracts:1",
              "$comment": "Structural JSON only; strict Rust decode, semantic checks and authenticated reviews required. Report types are integrator outputs, never worker authority.", "$defs": defs}
    files = {ROOT / "physical-contracts.schema.json": json.dumps(schema, indent=2) + "\n",
             ROOT / "contracts.generated.ts": "// Generated by generate_contracts.py; synthetic fixture contract.\n// Shape only. Gate/eligibility reports cannot authorize worker verdicts.\n" +
             "\n".join(f"export type {name} = {ts(value)};" for name, value in defs.items()) + "\n"}
    # Render provenance is a separate document, never an evaluator input field.
    origin = obj(schema_version={"const": 1}, subject=ref("SemanticId"),
                 module_kind={"type": "string"}, ancestry=array(ref("SemanticId")),
                 display_label={"type": "string"}, schematic={"type": "boolean"},
                 disclosure=enum("private", "public_reviewed"))
    files[ROOT / "render-origin.schema.json"] = json.dumps({
        "$schema": schema["$schema"], "$comment": "Render-only sidecar; never supplied to a judge.",
        "$defs": {"SemanticId": defs["SemanticId"]}, **origin}, indent=2) + "\n"
    files[ROOT / "render-origin.generated.ts"] = ('// Generated render-only sidecar; excluded from evaluator transport.\n'
        'import type {SemanticId} from "./contracts.generated";\nexport type RenderOriginSidecar = ' + ts(origin) + ';\n')
    valid = json.loads((ROOT / "fixtures/valid-subject.json").read_text())
    for name, key, value in [("expected-score", "expected_mechanism_score", 99), ("family-bonus", "family_bonus", 1), ("display-label", "display_label", "synthetic winner")]:
        bad = copy.deepcopy(valid)
        bad[key] = value
        files[ROOT / f"fixtures/invalid-subject-{name}.json"] = json.dumps(bad, indent=2) + "\n"
    bad = copy.deepcopy(valid)
    bad["artifacts"][0]["module_kind"] = "synthetic leaked registry label"
    files[ROOT / "fixtures/invalid-subject-nested-label.json"] = json.dumps(bad, indent=2) + "\n"
    files[ROOT / "fixtures/invalid-subject-duplicate-key.json"] = '{"schema_version":1,' + json.dumps(valid)[1:] + "\n"
    files[ROOT / "fixtures/invalid-subject-negative-zero.json"] = json.dumps(valid).replace('"translation_m": [0.0', '"translation_m": [-0.0', 1) + "\n"
    files[ROOT / "fixtures/invalid-task-outcome-forged-pass.json"] = '{"state":"pass"}\n'
    files[ROOT / "fixtures/invalid-expression-dimension.json"] = '{"op":"add","left":{"op":"constant","quantity":{"value":1,"unit":"pascal"}},"right":{"op":"constant","quantity":{"value":1,"unit":"metre"}}}\n'
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    stale = []
    for path, content in outputs().items():
        if args.check:
            if not path.is_file() or path.read_text() != content:
                stale.append(path.name)
        else:
            path.write_text(content)
    if stale:
        raise SystemExit("stale contract artifacts: " + ", ".join(stale))
    print("PASS: structural schema, generated TypeScript and frozen negative fixtures")


if __name__ == "__main__":
    main()
