//! Physical topology and explicit current operator authorization; no CPU modulo.
use crate::{ledger::Resources, BlobDigest, Result, RuntimeError};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalCore {
    pub package: u32,
    pub core: u32,
}
#[derive(Clone, Debug)]
pub struct Topology {
    pub cpus: BTreeMap<u32, PhysicalCore>,
    pub siblings: BTreeMap<u32, BTreeSet<u32>>,
    pub allowed: BTreeSet<u32>,
}
pub fn parse_cpu_list(text: &str) -> Result<BTreeSet<u32>> {
    let mut cpus = BTreeSet::new();
    for part in text.trim().split(',') {
        if part.is_empty() {
            return Err(RuntimeError::Invalid("empty_cpu_component"));
        }
        let fields: Vec<_> = part.split('-').collect();
        let first = fields[0]
            .parse::<u32>()
            .map_err(|_| RuntimeError::Invalid("cpu_list"))?;
        let last = if fields.len() == 1 {
            first
        } else if fields.len() == 2 {
            fields[1]
                .parse()
                .map_err(|_| RuntimeError::Invalid("cpu_list"))?
        } else {
            return Err(RuntimeError::Invalid("cpu_list"));
        };
        if first > last || last >= 4096 {
            return Err(RuntimeError::Invalid("cpu_range_budget"));
        }
        for cpu in first..=last {
            if !cpus.insert(cpu) {
                return Err(RuntimeError::Invalid("duplicate_cpu"));
            }
        }
    }
    Ok(cpus)
}
impl Topology {
    pub fn from_sysfs(root: &Path, allowed: BTreeSet<u32>) -> Result<Self> {
        let online = parse_cpu_list(&std::fs::read_to_string(root.join("online"))?)?;
        if allowed.is_empty() || !allowed.is_subset(&online) {
            return Err(RuntimeError::Invalid("allowed_cpuset_not_online"));
        }
        let mut cpus = BTreeMap::new();
        let mut siblings = BTreeMap::new();
        for cpu in &online {
            let path = root.join(format!("cpu{cpu}/topology"));
            let parse = |name| -> Result<u32> {
                std::fs::read_to_string(path.join(name))?
                    .trim()
                    .parse()
                    .map_err(|_| RuntimeError::Invalid("topology_id"))
            };
            cpus.insert(
                *cpu,
                PhysicalCore {
                    package: parse("physical_package_id")?,
                    core: parse("core_id")?,
                },
            );
            siblings.insert(
                *cpu,
                parse_cpu_list(&std::fs::read_to_string(path.join("thread_siblings_list"))?)?,
            );
        }
        for (cpu, group) in &siblings {
            if !group.contains(cpu) {
                return Err(RuntimeError::Invalid("topology_missing_self"));
            }
            for sibling in group.intersection(&online) {
                if cpus[cpu] != cpus[sibling] || siblings[sibling] != *group {
                    return Err(RuntimeError::Invalid("inconsistent_sibling_topology"));
                }
            }
        }
        Ok(Self {
            cpus,
            siblings,
            allowed,
        })
    }
    pub fn system() -> Result<Self> {
        Self::from_sysfs(Path::new("/sys/devices/system/cpu"), current_affinity()?)
    }
    pub fn physical_set(&self, cpus: &[u32]) -> Result<BTreeSet<PhysicalCore>> {
        let unique: BTreeSet<_> = cpus.iter().copied().collect();
        if unique.len() != cpus.len() || unique.is_empty() || !unique.is_subset(&self.allowed) {
            return Err(RuntimeError::Invalid(
                "cpus_outside_current_affinity_or_duplicate",
            ));
        }
        let cores: BTreeSet<_> = cpus
            .iter()
            .map(|cpu| {
                self.cpus
                    .get(cpu)
                    .copied()
                    .ok_or(RuntimeError::Invalid("missing_cpu_topology"))
            })
            .collect::<Result<_>>()?;
        if cores.len() != cpus.len() {
            return Err(RuntimeError::Invalid("smt_siblings_are_one_physical_core"));
        }
        Ok(cores)
    }
}
pub fn current_affinity() -> Result<BTreeSet<u32>> {
    // SAFETY: cpu_set_t is an integer bitset; zero is a valid empty set.
    let mut set: libc::cpu_set_t = unsafe {
        std::ptr::read_unaligned(
            [0u8; std::mem::size_of::<libc::cpu_set_t>()].as_ptr() as *const libc::cpu_set_t
        )
    };
    // SAFETY: set is a live writable cpu_set_t and its exact byte size is supplied; PID 0 selects this thread. Errors are checked before reading the mask.
    if unsafe { libc::sched_getaffinity(0, std::mem::size_of_val(&set), &mut set) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok((0..libc::CPU_SETSIZE)
        // SAFETY: cpu is bounded by CPU_SETSIZE and set is initialized by successful sched_getaffinity.
        .filter(|cpu| unsafe { libc::CPU_ISSET(*cpu as usize, &set) })
        .map(|cpu| cpu as u32)
        .collect())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLease {
    pub id: String,
    pub host: BlobDigest,
    pub cpus: Vec<u32>,
    pub expires_ms: u64,
    pub limits: Resources,
    pub gpu_authorized: bool,
    pub authority: BlobDigest,
}
/// Operator configuration is authenticated outside the worker payload channel.
pub struct Authorization {
    pub host: BlobDigest,
    pub cpus: BTreeSet<u32>,
    pub expires_ms: u64,
    pub limits: Resources,
    pub gpu_authorized: bool,
    pub source: BlobDigest,
}
pub struct AdmittedLease {
    record: ResourceLease,
    cores: BTreeSet<PhysicalCore>,
}
impl AdmittedLease {
    pub fn from_operator_config(
        record: ResourceLease,
        authority: &Authorization,
        topology: &Topology,
        now: u64,
        occupied: &BTreeSet<PhysicalCore>,
    ) -> Result<Self> {
        record.limits.check()?;
        joil_evidence::identity::check_id(&record.id).map_err(RuntimeError::Wire)?;
        if now >= record.expires_ms || record.expires_ms > authority.expires_ms {
            return Err(RuntimeError::ExpiredLease);
        }
        let selected: BTreeSet<_> = record.cpus.iter().copied().collect();
        if record.authority != authority.source
            || record.host != authority.host
            || !selected.is_subset(&authority.cpus)
            || !record.limits.fits(authority.limits)
            || (record.gpu_authorized && !authority.gpu_authorized)
        {
            return Err(RuntimeError::Invalid(
                "lease_outside_operator_authorization",
            ));
        }
        if record.expires_ms > 2_u64.pow(53)
            || record.limits.ram_bytes == 0
            || record.limits.scratch_bytes == 0
            || (!record.gpu_authorized
                && (record.limits.gpu_ms != 0 || record.limits.vram_bytes != 0))
        {
            return Err(RuntimeError::Invalid("lease_resource_vector"));
        }
        let cores = topology.physical_set(&record.cpus)?;
        if !cores.is_disjoint(occupied) {
            return Err(RuntimeError::Conflict("physical_core_already_allocated"));
        }
        Ok(Self { record, cores })
    }
    pub fn record(&self) -> &ResourceLease {
        &self.record
    }
    pub fn cores(&self) -> &BTreeSet<PhysicalCore> {
        &self.cores
    }
    pub fn check_task(&self, reserve: Resources, ranks: u32, threads: u32, now: u64) -> Result<()> {
        if now >= self.record.expires_ms {
            return Err(RuntimeError::ExpiredLease);
        }
        if ranks == 0
            || threads == 0
            || ranks
                .checked_mul(threads)
                .is_none_or(|n| n as usize > self.cores.len())
            || !reserve.fits(self.record.limits)
        {
            return Err(RuntimeError::Budget("task_exceeds_live_lease"));
        }
        Ok(())
    }
}

#[path = "lease/process.rs"]
mod process;
pub use self::process::*;
