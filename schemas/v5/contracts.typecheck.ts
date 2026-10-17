import type {SubjectBundle, TaskOutcome, GateDecisionReport, Expr} from "./contracts.generated";
declare const subject: SubjectBundle;
const physical: SubjectBundle = subject;
const task: TaskOutcome = {state: "succeeded", output_manifest: null};
const expression: Expr = {op: "constant", quantity: {value: 1, unit: "pascal"}};
// @ts-expect-error Evaluator wire cannot carry a family bonus.
const leaked: SubjectBundle = {...subject, family_bonus: 1};
// @ts-expect-error Neither a worker outcome nor a process exit is a gate verdict.
const forged: TaskOutcome = {state: "pass"};
// @ts-expect-error Nested physical artifact tags exclude module registries.
const nested: SubjectBundle = {...subject, artifacts: [{...subject.artifacts[0], module_kind: "winner"}]};
declare const report: GateDecisionReport;
// @ts-expect-error Integrator reports are immutable wire views.
report.gate_id = "renamed";
void [physical, task, expression, leaked, forged, nested];
