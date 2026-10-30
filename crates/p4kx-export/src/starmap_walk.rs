// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use p4kx_extraction::{CancelToken, Guid};

use crate::starmap_containers::Limits;
use crate::starmap_index::Index;
use crate::starmap_parse::{Body, Child, Level, Pose};
use crate::starmap_report::{Placement, StarmapReport, clip};
use crate::starmap_transform::Transform;

/// The most lines the report keeps per star system of the containers read, and of the issues.
const REPORT_LINES: usize = 300;

/// The longest line the report keeps.
const LINE_LEN: usize = 300;

#[derive(Default)]
pub(crate) struct SystemRead {
    pub(crate) placed: Vec<(Guid, Placement)>,
    pub(crate) unreadable: usize,
    pub(crate) missing: usize,
    pub(crate) malformed: usize,
    pub(crate) named: usize,
    pub(crate) opened: Vec<usize>,
    pub(crate) deepest: usize,
    pub(crate) report: StarmapReport,
    pub(crate) containers: Vec<Opened>,
}

/// A container the walk opened and the starmap record it hangs below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Opened {
    pub(crate) system: Arc<str>,
    pub(crate) entry: usize,
    pub(crate) under: Option<Guid>,
}

/// The walk through one star system, over containers that are already parsed.
pub(crate) struct System<'a> {
    index: &'a Index<'a>,
    bodies: &'a HashMap<usize, Body>,
    cancel: Option<&'a CancelToken>,
    name: Arc<str>,
    read: SystemRead,
    limits: Limits,
    budget: usize,
    open: Vec<usize>,
    seen: [usize; 3],
}

impl<'a> System<'a> {
    pub(crate) fn run(
        index: &'a Index<'a>,
        bodies: &'a HashMap<usize, Body>,
        name: &Arc<str>,
        entry: usize,
        cancel: Option<&'a CancelToken>,
        limits: Limits,
    ) -> SystemRead {
        let mut system = Self {
            index,
            bodies,
            cancel,
            name: Arc::clone(name),
            read: SystemRead::default(),
            limits,
            budget: limits.children,
            open: Vec::new(),
            seen: [0; 3],
        };
        system.container(entry, Transform::IDENTITY, None, 0);
        system.read
    }

    /// Keeps `line` in the report of the issues while there is room.
    fn issue(&mut self, line: &str) {
        if self.read.report.issues.len() < REPORT_LINES {
            self.read
                .report
                .issues
                .push(line.chars().take(LINE_LEN).collect());
        }
    }

    /// Counts a Child, or a limit, that left objects without a position, and reports it.
    fn skipped(&mut self, line: &str) {
        self.read.malformed += 1;
        self.issue(line);
    }

    /// Notes that the container at `entry`, `external` containers below the system container,
    /// was opened and hangs below the record `under`.
    fn opened(&mut self, entry: usize, under: Option<Guid>, external: usize) {
        self.read.opened.push(entry);
        self.read.containers.push(Opened {
            system: Arc::clone(&self.name),
            entry,
            under,
        });
        self.read.deepest = self.read.deepest.max(external);
    }

    /// Walks the container at `entry` placed at `at`; its Children name `under` as the record
    /// they hang below.
    fn container(&mut self, entry: usize, at: Transform, under: Option<Guid>, external: usize) {
        if self.cancel.is_some_and(CancelToken::is_cancelled) {
            return;
        }
        let label = self
            .index
            .entries
            .get(entry)
            .map_or_else(String::new, |socpak| socpak.path().to_owned());
        if self.open.contains(&entry) {
            self.skipped(&format!("{label}: names a container that is open above it"));
            return;
        }
        let bodies = self.bodies;
        let Some(body) = bodies.get(&entry) else {
            self.read.unreadable += 1;
            self.issue(&format!("{label}: not read"));
            return;
        };
        let before = self.seen;
        match body {
            Body::Unreadable(reason) => {
                self.read.unreadable += 1;
                self.issue(&format!("{label}: {reason}"));
                return;
            }
            Body::Unparsed => {
                self.opened(entry, under, external);
                self.read.unreadable += 1;
                self.issue(&format!("{label}: the XML cannot be parsed"));
                return;
            }
            Body::Parsed { root, fallback } => {
                self.opened(entry, under, external);
                if let Some(taken) = fallback {
                    self.issue(&format!(
                        "{label}: no XML named like the container, read {} instead",
                        clip(taken)
                    ));
                }
                self.open.push(entry);
                self.children(root, at, under, external, &label);
                self.open.pop();
            }
        }
        if self.seen[1] > before[1] {
            self.read.named += 1;
            if self.read.report.containers.len() < REPORT_LINES {
                self.read.report.containers.push(format!(
                    "{label}: {} children, {} name a record, {} external",
                    self.seen[0] - before[0],
                    self.seen[1] - before[1],
                    self.seen[2] - before[2]
                ));
            }
        }
    }

    fn children(
        &mut self,
        level: &Level,
        parent: Transform,
        under: Option<Guid>,
        external: usize,
        label: &str,
    ) {
        if level.capped {
            let limit = self.limits.depth;
            self.skipped(&format!("{label}: Children nested deeper than {limit}"));
            return;
        }
        if level.extra_lists {
            self.issue(&format!(
                "{label}: more than one ChildObjectContainers, only the first is read"
            ));
        }
        for child in &level.children {
            if self.budget == 0 {
                let limit = self.limits.children;
                self.skipped(&format!("{label}: more than {limit} Children in a system"));
                return;
            }
            self.budget -= 1;
            self.seen[0] += 1;
            let world = match &child.pose {
                Pose::Valid { position, rotation } => {
                    let world = parent.place(*position, *rotation);
                    if !world.is_sane() {
                        self.skipped(&format!(
                            "{label}: out of range: pos {position:?} places {:?} of {}",
                            world.position,
                            clip(&child.entity_name)
                        ));
                        continue;
                    }
                    world
                }
                Pose::Malformed { pos, rot, entity } => {
                    self.skipped(&format!(
                        "{label}: malformed pos {pos} rot {rot} of {entity}"
                    ));
                    continue;
                }
            };
            let record = self.note(child, world, under);
            let below = record.or(under);
            if let Some(path) = &child.external {
                self.follow(path, world, below, external, label);
            }
            self.children(&child.below, world, below, external, label);
        }
        if level.truncated {
            let limit = self.limits.children;
            self.skipped(&format!(
                "{label}: more than {limit} Children in the containers of the Archive"
            ));
        }
    }

    /// Walks the external container `path` below a Child placed at `world`.
    fn follow(
        &mut self,
        path: &str,
        world: Transform,
        below: Option<Guid>,
        external: usize,
        label: &str,
    ) {
        self.seen[2] += 1;
        if external >= self.limits.external {
            let limit = self.limits.external;
            self.skipped(&format!(
                "{label}: external containers nested deeper than {limit}: {}",
                clip(path)
            ));
        } else if let Some(entry) = self.index.find(path) {
            self.container(entry, world, below, external + 1);
        } else {
            self.read.missing += 1;
            self.issue(&format!(
                "{label}: the Archive holds no container {}",
                clip(path)
            ));
        }
    }

    /// Records the placement of `child` when it names a starmap record and returns the record.
    fn note(&mut self, child: &Child, world: Transform, under: Option<Guid>) -> Option<Guid> {
        let record = child.record?;
        self.seen[1] += 1;
        self.read.placed.push((
            record,
            Placement {
                system: Arc::clone(&self.name),
                position: world.position,
                entity_name: child.entity_name.clone(),
                placed_under: under,
            },
        ));
        Some(record)
    }
}
