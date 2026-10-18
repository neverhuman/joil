export type Tier = 'measured' | 'validated_model' | 'unvalidated_model' | 'synthetic' | 'illustration' | 'unavailable';
export type EvidenceValue = {
  id: string; label: string; value: number | string | null; unit: string | null;
  uncertainty: string | null; tier: Tier; source: string | null; status: 'available' | 'missing' | 'failed' | 'revoked';
  field?: string; time_s?: number; geometry_sha256?: string;
};
export type Subject = { id: string; geometry_sha256: string; geometry_source: string | null; ancestry: string[]; values: EvidenceValue[]; overlays: { id: string; field: string; time_s: number; geometry_sha256: string; source: string }[]; };
export type HistoricalCorrection = { id: string; label: string; source: string | null };
export type Manifest = {
  schema_version: 1; revision: string; generated_at: string; current: boolean;
  registry_event?: { sequence: number; revoked_claim_ids: string[] };
  campaign: { tasks_total: number | null; tasks_completed: number | null; attempts: number | null; spend: string | null; reservations: string | null; checkpoint: string | null; control_arms: string[]; blockers: string[]; admitted_capabilities: string[]; software_status: string; scientific_release: string };
  subjects: Subject[]; values: EvidenceValue[]; archive: { id: string; subject_id: string; x: number | null; y: number | null; unit_x: string; unit_y: string; tier: Tier }[]; historical_corrections: HistoricalCorrection[];
  offline_dossier?: { reviewed: boolean; revision: string; current: boolean; revoked_claim_ids: string[]; source: string };
};

const tiers: Tier[] = ['measured', 'validated_model', 'unvalidated_model', 'synthetic', 'illustration', 'unavailable'];
const obj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const nullable = (v: unknown, type: 'string' | 'number') => v === null || typeof v === type;
const exactKeys = (v: Record<string, unknown>, keys: string[]) => Object.keys(v).every(k => keys.includes(k));
const safeSource = (s: unknown) => typeof s === 'string' && (s.startsWith('./') || s.startsWith('../') || s.startsWith('/') || /^https:\/\//i.test(s));
function validValue(v: unknown): v is EvidenceValue {
  if (!obj(v)) return false;
  if (!exactKeys(v, ['id','label','value','unit','uncertainty','tier','source','status','field','time_s','geometry_sha256'])) return false;
  const available = v.status === 'available';
  return typeof v.id === 'string' && typeof v.label === 'string' && (v.value === null || typeof v.value === 'string' || (typeof v.value === 'number' && Number.isFinite(v.value))) &&
    nullable(v.unit, 'string') && nullable(v.uncertainty, 'string') && tiers.includes(v.tier as Tier) && (v.source === null || safeSource(v.source)) &&
    ['available','missing','failed','revoked'].includes(String(v.status)) && (!available || (v.value !== null && typeof v.unit === 'string' && !!v.unit && typeof v.uncertainty === 'string' && !!v.uncertainty && safeSource(v.source) && v.tier !== 'unavailable'));
}
function validSubject(v: unknown): v is Subject {
  if (!obj(v)) return false;
  return exactKeys(v, ['id','geometry_sha256','geometry_source','ancestry','values','overlays']) && typeof v.id === 'string' && /^[a-f0-9]{64}$/i.test(String(v.geometry_sha256)) && (v.geometry_source === null || safeSource(v.geometry_source)) && Array.isArray(v.ancestry) && v.ancestry.every(id => typeof id === 'string') && Array.isArray(v.values) && v.values.every(validValue) && Array.isArray(v.overlays) && v.overlays.every(o => obj(o) && exactKeys(o, ['id','field','time_s','geometry_sha256','source']) && typeof o.id === 'string' && typeof o.field === 'string' && typeof o.time_s === 'number' && Number.isFinite(o.time_s) && o.geometry_sha256 === v.geometry_sha256 && safeSource(o.source));
}
export function isManifest(v: unknown): v is Manifest {
  if (!obj(v) || !exactKeys(v, ['schema_version','revision','generated_at','current','registry_event','campaign','subjects','values','archive','historical_corrections','offline_dossier']) || v.schema_version !== 1 || typeof v.revision !== 'string' || !v.revision || typeof v.generated_at !== 'string' || typeof v.current !== 'boolean' || !Array.isArray(v.values) || !v.values.every(validValue) || !Array.isArray(v.subjects) || !v.subjects.every(validSubject) || !Array.isArray(v.archive) || !Array.isArray(v.historical_corrections)) return false;
  if (!obj(v.campaign)) return false;
  const c = v.campaign;
  if (!exactKeys(c, ['tasks_total','tasks_completed','attempts','spend','reservations','checkpoint','control_arms','blockers','admitted_capabilities','software_status','scientific_release'])) return false;
  if (!(c.tasks_total === null || (typeof c.tasks_total === 'number' && Number.isFinite(c.tasks_total))) || !(c.tasks_completed === null || (typeof c.tasks_completed === 'number' && Number.isFinite(c.tasks_completed))) || !(c.attempts === null || (typeof c.attempts === 'number' && Number.isFinite(c.attempts))) || !['spend','reservations','checkpoint','software_status','scientific_release'].every(k => typeof c[k] === 'string' || c[k] === null) || !['control_arms','blockers','admitted_capabilities'].every(k => Array.isArray(c[k]) && (c[k] as unknown[]).every(x => typeof x === 'string'))) return false;
  if (!v.archive.every(p => obj(p) && exactKeys(p, ['id','subject_id','x','y','unit_x','unit_y','tier']) && typeof p.id === 'string' && typeof p.subject_id === 'string' && (p.x === null || (typeof p.x === 'number' && Number.isFinite(p.x))) && (p.y === null || (typeof p.y === 'number' && Number.isFinite(p.y))) && typeof p.unit_x === 'string' && typeof p.unit_y === 'string' && tiers.includes(p.tier as Tier))) return false;
  if (!v.historical_corrections.every(c => obj(c) && exactKeys(c, ['id','label','source']) && typeof c.id === 'string' && typeof c.label === 'string' && (c.source === null || safeSource(c.source)))) return false;
  if (v.registry_event !== undefined && (!obj(v.registry_event) || !exactKeys(v.registry_event, ['sequence','revoked_claim_ids']) || typeof v.registry_event.sequence !== 'number' || !Array.isArray(v.registry_event.revoked_claim_ids) || !v.registry_event.revoked_claim_ids.every(x => typeof x === 'string'))) return false;
  if (v.offline_dossier !== undefined && (!obj(v.offline_dossier) || !exactKeys(v.offline_dossier, ['reviewed','revision','current','revoked_claim_ids','source']) || typeof v.offline_dossier.reviewed !== 'boolean' || typeof v.offline_dossier.revision !== 'string' || typeof v.offline_dossier.current !== 'boolean' || !safeSource(v.offline_dossier.source) || !Array.isArray(v.offline_dossier.revoked_claim_ids) || !v.offline_dossier.revoked_claim_ids.every(x => typeof x === 'string'))) return false;
  return true;
}
export function resolveIdentity(m: Pick<Manifest, 'revision' | 'subjects'>, revision: string | null, subject: string | null): Subject | undefined {
  if (!revision || revision !== m.revision || !subject) return undefined;
  return m.subjects.find(s => s.id === subject);
}
export function applyRevocation(m: Manifest, sequence: number, ids: string[]): Manifest {
  const revoked = new Set(ids);
  return { ...m, current: false, registry_event: { sequence, revoked_claim_ids: [...revoked] },
    values: m.values.map(v => revoked.has(v.id) ? { ...v, status: 'revoked' } : v),
    subjects: m.subjects.map(s => ({ ...s, values: s.values.map(v => revoked.has(v.id) ? { ...v, status: 'revoked' } : v) })),
    offline_dossier: m.offline_dossier ? { ...m.offline_dossier, current: false, revoked_claim_ids: [...new Set([...m.offline_dossier.revoked_claim_ids, ...revoked])] } : undefined };
}
