//! Fleet summary over replicate runs that share one objective.
//!
//! A fleet root holds `runs/<arm>-<seed>/` run directories and `controls/R-<seed>.json`
//! equal-budget random controls. For each search seed this compares the seeded (`S`) and
//! de novo (`D`) runs with the random control (`R`) at equal evaluations, refuses arms of
//! one replicate that used different campaigns and replicates scored under different
//! objectives, and pools the elites of every run so a final run can start from all of them.
//! Other run directories (for example a resumed lineage) are summarised without pairing.

use crate::campaign::Campaign;
use crate::checkpoint::{self, Checkpoint};
use crate::control::ControlReport;
use crate::evolve::Individual;
use crate::genome::Genome;
use crate::render::charts::{comparison_chart, Line};
use crate::run::seed_individuals;
use crate::score::Score;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

pub const FORMAT: &str = "joil-forge-fleet-summary/1";

/// One arm of a replicate, or any other run, at its last saved state.
#[derive(Clone, Debug, Serialize)]
pub struct ArmSummary {
    pub source: String,
    pub campaign_sha256: String,
    /// Generations reached (`None` for a random control).
    pub generation: Option<usize>,
    pub evaluations: u64,
    pub best_genome_sha256: Option<String>,
    pub best: Option<Score>,
    pub niches: usize,
    pub feasible_niches: usize,
    /// Evaluations when the best design was first feasible: the first logged generation of a
    /// run (an upper bound when the initial population already held one) or the first
    /// control batch.
    pub first_feasible_evaluations: Option<u64>,
    #[serde(skip)]
    pub trajectory: Vec<(f64, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    FirstBetter,
    SecondBetter,
    Tie,
    /// One side is missing or the two spent different evaluation budgets.
    NotComparable,
}

#[derive(Clone, Debug, Serialize)]
pub struct Replicate {
    pub seed: u64,
    pub seeded: Option<ArmSummary>,
    pub de_novo: Option<ArmSummary>,
    pub random: Option<ArmSummary>,
    /// De novo search versus random sampling at equal evaluations.
    pub search_vs_random: Verdict,
    /// Seeded versus de novo search at equal evaluations.
    pub seeded_vs_de_novo: Verdict,
}

impl Replicate {
    fn new(seed: u64) -> Self {
        Self {
            seed,
            seeded: None,
            de_novo: None,
            random: None,
            search_vs_random: Verdict::NotComparable,
            seeded_vs_de_novo: Verdict::NotComparable,
        }
    }
}

/// One-sided exact sign test; ties and incomparable replicates count against the first side.
#[derive(Clone, Debug, Serialize)]
pub struct SignTest {
    pub replicates: usize,
    pub first_better: usize,
    pub second_better: usize,
    pub ties_or_not_comparable: usize,
    pub p_one_sided: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct FleetSummary {
    pub format: String,
    pub evidence_class: String,
    pub forge_version: String,
    pub model_version: String,
    pub replicates: Vec<Replicate>,
    pub search_vs_random: SignTest,
    pub seeded_vs_de_novo: SignTest,
    pub other_runs: Vec<ArmSummary>,
    pub overall_best: Option<ArmSummary>,
    pub note: String,
}

fn rank(a: &Option<Score>, b: &Option<Score>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.compare(y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn verdict(first: &Option<ArmSummary>, second: &Option<ArmSummary>) -> Verdict {
    let (Some(a), Some(b)) = (first, second) else {
        return Verdict::NotComparable;
    };
    if a.evaluations != b.evaluations {
        return Verdict::NotComparable;
    }
    match rank(&a.best, &b.best) {
        Ordering::Less => Verdict::FirstBetter,
        Ordering::Greater => Verdict::SecondBetter,
        Ordering::Equal => Verdict::Tie,
    }
}

/// P(X >= k) for X ~ Binomial(n, 1/2).
pub fn sign_test_p(k: usize, n: usize) -> f64 {
    let mut c = 1.0_f64;
    let mut tail = 0.0;
    for i in 0..=n {
        if i >= k {
            tail += c;
        }
        c = c * (n - i) as f64 / (i + 1) as f64;
    }
    tail / 2f64.powi(n as i32)
}

fn sign_test(vs: &[Verdict]) -> SignTest {
    let first = vs.iter().filter(|v| **v == Verdict::FirstBetter).count();
    let second = vs.iter().filter(|v| **v == Verdict::SecondBetter).count();
    SignTest {
        replicates: vs.len(),
        first_better: first,
        second_better: second,
        ties_or_not_comparable: vs.len() - first - second,
        p_one_sided: sign_test_p(first, vs.len()),
    }
}

fn run_summary(name: &str, cp: &Checkpoint) -> Result<ArmSummary, String> {
    if cp.campaign.sha256() != cp.manifest.campaign_sha256 {
        return Err(format!(
            "{name}: checkpoint campaign does not match its manifest"
        ));
    }
    let s = &cp.state;
    Ok(ArmSummary {
        source: name.to_string(),
        campaign_sha256: cp.manifest.campaign_sha256.clone(),
        generation: Some(s.generation),
        evaluations: s.evaluations,
        best_genome_sha256: s.best.as_ref().map(|b| b.hash.clone()),
        best: s.best.as_ref().map(|b| b.score.clone()),
        niches: s.archive.cells.len(),
        feasible_niches: s.archive.feasible(),
        first_feasible_evaluations: s
            .history
            .iter()
            .find(|g| g.best_feasible)
            .map(|g| g.evaluations),
        trajectory: s
            .history
            .iter()
            .filter_map(|g| g.best_fitness.map(|f| (g.evaluations as f64, f)))
            .collect(),
    })
}

fn control_summary(name: &str, r: &ControlReport) -> ArmSummary {
    ArmSummary {
        source: name.to_string(),
        campaign_sha256: r.campaign_sha256.clone(),
        generation: None,
        evaluations: r.evaluations,
        best_genome_sha256: r.best.as_ref().map(|b| b.hash.clone()),
        best: r.best.as_ref().map(|b| b.score.clone()),
        niches: r.trajectory.last().map(|t| t.niches).unwrap_or(0),
        feasible_niches: r.trajectory.last().map(|t| t.feasible_niches).unwrap_or(0),
        first_feasible_evaluations: r
            .trajectory
            .iter()
            .find(|t| t.feasible_so_far > 0)
            .map(|t| t.evaluations),
        trajectory: r
            .trajectory
            .iter()
            .filter_map(|t| t.best_fitness.map(|f| (t.evaluations as f64, f)))
            .collect(),
    }
}

/// The campaign with its search settings removed: equal for runs scored alike.
fn objective(c: &Campaign) -> Campaign {
    Campaign {
        name: String::new(),
        seed: 0,
        closure_seed: Some(c.closure_seed()),
        population: 0,
        islands: 0,
        ..c.clone()
    }
}

/// A run or control file name split into its arm letter and search seed.
fn arm_seed(name: &str) -> Option<(char, u64)> {
    let (arm, seed) = name.split_once('-')?;
    let arm = match arm {
        "S" => 'S',
        "D" => 'D',
        "R" => 'R',
        _ => return None,
    };
    Some((arm, seed.parse().ok()?))
}

fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    v.sort();
    Ok(v)
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Every run directory under `root/runs` with its checkpoint, by name.
fn load_runs(root: &Path) -> Result<Vec<(String, Checkpoint)>, String> {
    let mut out = Vec::new();
    for p in sorted_entries(&root.join("runs"))? {
        if checkpoint::checkpoint_path(&p).exists() {
            out.push((file_name(&p), checkpoint::load(&p)?));
        }
    }
    Ok(out)
}

pub fn summarize(root: &Path) -> Result<FleetSummary, String> {
    let runs = load_runs(root)?;
    let mut reps: BTreeMap<u64, Replicate> = BTreeMap::new();
    let mut other_runs = Vec::new();
    let mut shared: Option<(String, Campaign)> = None;
    let mut versions: Option<(String, String)> = None;
    for (name, cp) in &runs {
        let v = (
            cp.manifest.forge_version.clone(),
            cp.manifest.model_version.clone(),
        );
        if versions.get_or_insert_with(|| v.clone()) != &v {
            return Err(format!(
                "{name}: forge or model version differs from the fleet"
            ));
        }
        let summary = run_summary(name, cp)?;
        let Some((arm, seed)) = arm_seed(name) else {
            other_runs.push(summary);
            continue;
        };
        if cp.campaign.seed != seed {
            return Err(format!(
                "{name}: campaign seed {} does not match the run name",
                cp.campaign.seed
            ));
        }
        let obj = objective(&cp.campaign);
        match &shared {
            None => shared = Some((name.clone(), obj)),
            Some((first, o)) if *o != obj => {
                return Err(format!(
                    "{name}: scored under a different objective from {first}"
                ))
            }
            Some(_) => {}
        }
        let rep = reps.entry(seed).or_insert_with(|| Replicate::new(seed));
        match arm {
            'S' => rep.seeded = Some(summary),
            'D' => rep.de_novo = Some(summary),
            _ => {
                return Err(format!(
                    "{name}: a run directory cannot be a random control"
                ))
            }
        }
    }
    for p in sorted_entries(&root.join("controls"))? {
        let name = file_name(&p);
        let Some(stem) = name.strip_suffix(".json") else {
            continue;
        };
        let Some(('R', seed)) = arm_seed(stem) else {
            continue;
        };
        let text = std::fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
        let r: ControlReport =
            serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", p.display()))?;
        reps.entry(seed)
            .or_insert_with(|| Replicate::new(seed))
            .random = Some(control_summary(stem, &r));
    }
    for rep in reps.values_mut() {
        let digests: HashSet<&str> = [&rep.seeded, &rep.de_novo, &rep.random]
            .into_iter()
            .flatten()
            .map(|a| a.campaign_sha256.as_str())
            .collect();
        if digests.len() > 1 {
            return Err(format!(
                "replicate {}: its arms used different campaigns",
                rep.seed
            ));
        }
        rep.search_vs_random = verdict(&rep.de_novo, &rep.random);
        rep.seeded_vs_de_novo = verdict(&rep.seeded, &rep.de_novo);
    }
    let replicates: Vec<Replicate> = reps.into_values().collect();
    let svr: Vec<Verdict> = replicates.iter().map(|r| r.search_vs_random).collect();
    let svd: Vec<Verdict> = replicates.iter().map(|r| r.seeded_vs_de_novo).collect();
    let overall_best = replicates
        .iter()
        .flat_map(|r| [&r.seeded, &r.de_novo])
        .flatten()
        .chain(other_runs.iter())
        .filter(|a| a.best.is_some())
        .min_by(|a, b| rank(&a.best, &b.best).then_with(|| a.source.cmp(&b.source)))
        .cloned();
    let (forge_version, model_version) = versions.unwrap_or_default();
    Ok(FleetSummary {
        format: FORMAT.into(),
        evidence_class: joil_forge_judges::EVIDENCE_CLASS.into(),
        forge_version,
        model_version,
        replicates,
        search_vs_random: sign_test(&svr),
        seeded_vs_de_novo: sign_test(&svd),
        other_runs,
        overall_best,
        note: "Screen-model search statistics over replicate runs that share one objective; \
               not physical evidence. Rankings use the engine's order: feasible before \
               infeasible, then higher fitness, else lower violation. Ties and incomparable \
               replicates count against the first side of each sign test."
            .into(),
    })
}

type Pick = fn(&Replicate) -> &Option<ArmSummary>;

/// Best fitness against evaluations for every replicate, one panel per arm.
pub fn chart(s: &FleetSummary) -> String {
    let arms: [(&str, Pick); 3] = [
        ("Seeded from the last best (S)", |r| &r.seeded),
        ("De novo search (D)", |r| &r.de_novo),
        ("Random sampling at equal budget (R)", |r| &r.random),
    ];
    let labels: Vec<String> = s.replicates.iter().map(|r| r.seed.to_string()).collect();
    let panels: Vec<(&str, Vec<Line>)> = arms
        .iter()
        .map(|(title, get)| {
            let lines = s
                .replicates
                .iter()
                .zip(&labels)
                .map(|(r, l)| {
                    (
                        l.as_str(),
                        get(r)
                            .as_ref()
                            .map(|a| a.trajectory.clone())
                            .unwrap_or_default(),
                    )
                })
                .collect();
            (*title, lines)
        })
        .collect();
    comparison_chart(
        "Best weighted fitness found so far, per replicate seed",
        "Evaluations. Lines start once a replicate's best is feasible; every arm of a seed shares one campaign.",
        &panels,
    )
}

/// Union of the seed candidates (archive elites, best, island leaders) of every `S`/`D` run,
/// deduplicated. Runs take turns, each in its own best-first order, so every run
/// contributes its leaders before any run contributes its tail.
pub fn pool(root: &Path) -> Result<Vec<Genome>, String> {
    let runs = load_runs(root)?;
    let lists: Vec<Vec<Individual>> = runs
        .iter()
        .filter(|(name, _)| matches!(arm_seed(name), Some(('S' | 'D', _))))
        .map(|(_, cp)| seed_individuals(cp).into_iter().cloned().collect())
        .collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let longest = lists.iter().map(Vec::len).max().unwrap_or(0);
    for k in 0..longest {
        for l in &lists {
            if let Some(ind) = l.get(k) {
                if seen.insert(ind.hash.clone()) {
                    out.push(ind.genome.clone());
                }
            }
        }
    }
    Ok(out)
}
