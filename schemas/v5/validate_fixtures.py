#!/usr/bin/env python3
"""Independent JSON Schema structural check (requires available jsonschema).
Duplicate keys, signed zero and dimensional validity are separately checked in Rust.
"""
import json
from pathlib import Path
from jsonschema import Draft202012Validator
root=Path(__file__).resolve().parent;schema=json.loads((root/"physical-contracts.schema.json").read_text());Draft202012Validator.check_schema(schema)
names={"subject":"SubjectBundle","surface":"TriangleSurface","evaluation-key":"EvaluationKey","decision-key":"DecisionKey","expression":"Expr","task-outcome":"TaskOutcome","observation-state":"ObservationState"}
positive=negative=semantic=0
for name,target in names.items():
 v=Draft202012Validator({"$ref":"#/$defs/"+target,"$defs":schema["$defs"]});v.validate(json.loads((root/f"fixtures/valid-{name}.json").read_text()));positive+=1
for p in sorted((root/"fixtures").glob("invalid-*.json")):
 target="SubjectBundle" if p.name.startswith("invalid-subject") else ("TaskOutcome" if "task-outcome" in p.name else "Expr")
 v=Draft202012Validator({"$ref":"#/$defs/"+target,"$defs":schema["$defs"]});errors=list(v.iter_errors(json.loads(p.read_text())))
 if any(x in p.name for x in ["duplicate-key","negative-zero","expression-dimension"]):
  assert not errors,(p,errors);semantic+=1
 else:assert errors,p;negative+=1
print(json.dumps({"synthetic_only":True,"valid_structural":positive,"rejected_structural":negative,"requires_strict_Rust_semantics":semantic,"jsonschema_version":__import__("importlib.metadata").metadata.version("jsonschema")}))
