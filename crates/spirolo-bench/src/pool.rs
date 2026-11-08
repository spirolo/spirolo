//! M6's single deterministic OSS-dev sample, shared by the bench and adjudicators.
use crate::{ReportedFinding, Truth};
use std::collections::{BTreeMap, VecDeque};

/// One entry's fresh check and execution guards; metadata comes from the frozen manifest.
#[derive(Debug, Clone)]
pub struct PoolEntry {
    /// Corpus directory name, part of the identity across repositories.
    pub name: String,
    /// `oss` or `private`.
    pub source: String,
    /// `dev` or `held-out`.
    pub set: String,
    /// Findings from the installed clone's real check.
    pub reported: Vec<ReportedFinding>,
    /// The entry's execution guards and retired identities.
    pub truth: Truth,
}

/// Target per rule across the OSS dev set.
pub const POOL_TARGET: usize = 60;
/// Maximum findings per entry per rule.
pub const ENTRY_CAP: usize = 3;

fn stride<T: Clone>(rows: &[T], target: usize) -> Vec<T> {
    let count = rows.len().min(target);
    (0..count)
        .map(|i| rows[i * rows.len() / count].clone())
        .collect()
}

/// Draw once per rule, evenly across sorted entries and within each entry. Private and
/// held-out entries are excluded before their findings are examined.
///
/// Returned identities
/// include the entry name because findings from separate repositories can share an id.
#[must_use]
pub fn pooled_sample(entries: &[PoolEntry], rule: &str) -> Vec<(String, ReportedFinding)> {
    let mut eligible: Vec<_> = entries
        .iter()
        .filter(|e| e.source == "oss" && e.set == "dev")
        .collect();
    eligible.sort_by(|a, b| a.name.cmp(&b.name));
    if rule == "flake-candidate" {
        return pooled_flakes(&eligible);
    }
    let pools = eligible.iter().map(|e| {
        let mut findings: Vec<_> = e
            .reported
            .iter()
            .filter(|f| f.rule == rule && f.decided)
            .cloned()
            .collect();
        findings.sort_by(|a, b| a.id.cmp(&b.id));
        stride(&findings, ENTRY_CAP)
            .into_iter()
            .map(|f| (e.name.clone(), f))
            .collect::<Vec<_>>()
    });
    let spread: Vec<_> = pools.flatten().collect();
    stride(&spread, POOL_TARGET)
}

fn pooled_flakes(entries: &[&PoolEntry]) -> Vec<(String, ReportedFinding)> {
    let mut kinds: BTreeMap<String, BTreeMap<String, Vec<ReportedFinding>>> = BTreeMap::new();
    for entry in entries {
        let retired = entry.truth.retired_ids("flake-candidate");
        for finding in &entry.reported {
            if finding.rule != "flake-candidate"
                || !finding.decided
                || retired.contains(finding.id.as_str())
            {
                continue;
            }
            let Some(kind) = finding.evidence.get("kind") else {
                continue;
            };
            kinds
                .entry(kind.clone())
                .or_default()
                .entry(entry.name.clone())
                .or_default()
                .push(finding.clone());
        }
    }
    let quota = POOL_TARGET.div_ceil(kinds.len().max(1));
    let mut queues = Vec::new();
    for mut members in kinds.into_values() {
        let names: Vec<_> = members.keys().cloned().collect();
        let first = stride(&names, quota);
        let mut order = first.clone();
        order.extend(names.into_iter().filter(|n| !first.contains(n)));
        let pools: Vec<_> = order
            .into_iter()
            .map(|name| {
                let mut pool = members.remove(&name).expect("kind member");
                pool.sort_by(|a, b| {
                    crate::precision::seeded_rank(203, &a.id)
                        .cmp(&crate::precision::seeded_rank(203, &b.id))
                        .then_with(|| a.id.cmp(&b.id))
                });
                pool.into_iter()
                    .map(|f| (name.clone(), f))
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut queue = VecDeque::new();
        for round in 0..pools.iter().map(Vec::len).max().unwrap_or(0) {
            for pool in &pools {
                if let Some(f) = pool.get(round) {
                    queue.push_back(f.clone());
                }
            }
        }
        queues.push(queue);
    }
    let truths: BTreeMap<_, _> = entries
        .iter()
        .map(|e| (e.name.as_str(), &e.truth))
        .collect();
    let mut counts = BTreeMap::new();
    let mut drawn = Vec::new();
    for _ in 0..quota {
        for queue in &mut queues {
            if drawn.len() == POOL_TARGET {
                break;
            }
            take_flake(queue, &mut counts, &mut drawn, &truths, false);
        }
    }
    while drawn.len() < POOL_TARGET {
        let mut topped = false;
        for queue in &mut queues {
            if drawn.len() == POOL_TARGET {
                break;
            }
            topped |= take_flake(queue, &mut counts, &mut drawn, &truths, true);
        }
        if !topped {
            break;
        }
    }
    drawn
}

fn take_flake(
    queue: &mut VecDeque<(String, ReportedFinding)>,
    counts: &mut BTreeMap<String, usize>,
    drawn: &mut Vec<(String, ReportedFinding)>,
    truths: &BTreeMap<&str, &Truth>,
    executable: bool,
) -> bool {
    while let Some((entry, finding)) = queue.pop_front() {
        if counts.get(&entry).copied().unwrap_or(0) >= ENTRY_CAP {
            continue;
        }
        if executable && !truths[entry.as_str()].executes_for("flake-candidate", &finding.file) {
            continue;
        }
        *counts.entry(entry.clone()).or_default() += 1;
        drawn.push((entry, finding));
        return true;
    }
    false
}
