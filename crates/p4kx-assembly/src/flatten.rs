// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    Attachment, EntityNode, Geometry, Issue, IssueKind, MAX_ASSEMBLY_DEPTH, PortNode, Target,
};

/// One part of an Assembly before its model is read: an Entity of the Loadout tree with its port.
pub(crate) struct Draft<'t> {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) port: Option<&'t PortNode>,
    pub(crate) node: &'t EntityNode,
    pub(crate) parent: Option<usize>,
    pub(crate) geometry: Option<&'t Geometry>,
}

/// The parts of a Loadout tree in order, every part after its parent, and what could not be
/// turned into one.
pub(crate) struct Flat<'t> {
    pub(crate) drafts: Vec<Draft<'t>>,
    pub(crate) issues: Vec<Issue>,
}

/// Lists the Entities of the tree below `root`, depth first in port order, at most
/// `max_parts` of them and at most [`MAX_ASSEMBLY_DEPTH`] levels deep.
pub(crate) fn flatten(root: &EntityNode, max_parts: usize) -> Flat<'_> {
    let mut walker = Walker {
        flat: Flat {
            drafts: vec![Draft {
                name: root.class_name().to_owned(),
                path: String::new(),
                port: None,
                node: root,
                parent: None,
                geometry: root.geometry(),
            }],
            issues: Vec::new(),
        },
        full: false,
        max_parts,
    };
    walker.ports(root.ports(), 0, "", 1);
    walker.flat
}

struct Walker<'t> {
    flat: Flat<'t>,
    full: bool,
    max_parts: usize,
}

impl<'t> Walker<'t> {
    fn ports(&mut self, ports: &'t [PortNode], owner: usize, prefix: &str, depth: usize) {
        for port in ports {
            if self.full {
                return;
            }
            let path = if prefix.is_empty() {
                port.name().to_owned()
            } else {
                format!("{prefix}/{}", port.name())
            };
            if depth > MAX_ASSEMBLY_DEPTH {
                self.issue(
                    &path,
                    port.name(),
                    IssueKind::TooDeep {
                        limit: MAX_ASSEMBLY_DEPTH,
                    },
                );
                continue;
            }
            match port.attachment() {
                Attachment::Entity(child) => self.attach(port, child, owner, &path, depth),
                Attachment::Unresolved(unresolved) => {
                    let name = match unresolved.target() {
                        Target::Reference(guid) => guid.to_string(),
                        Target::ClassName(name) => name.clone(),
                        Target::Unreadable => String::new(),
                    };
                    self.issue(
                        &path,
                        &name,
                        IssueKind::Unresolved(unresolved.error().clone()),
                    );
                }
                Attachment::Empty => {}
            }
            self.ports(port.nested(), owner, &path, depth + 1);
        }
    }

    fn attach(
        &mut self,
        port: &'t PortNode,
        child: &'t EntityNode,
        owner: usize,
        path: &str,
        depth: usize,
    ) {
        if self.flat.drafts.len() >= self.max_parts {
            self.full = true;
            self.issue(
                path,
                child.class_name(),
                IssueKind::TooManyParts {
                    limit: self.max_parts,
                },
            );
            return;
        }
        let index = self.flat.drafts.len();
        self.flat.drafts.push(Draft {
            name: format!("{}/{}", port.name(), child.class_name()),
            path: path.to_owned(),
            port: Some(port),
            node: child,
            parent: Some(owner),
            geometry: child.geometry(),
        });
        self.ports(child.ports(), index, path, depth + 1);
    }

    fn issue(&mut self, path: &str, name: &str, kind: IssueKind) {
        self.flat.issues.push(Issue {
            path: path.to_owned(),
            name: name.to_owned(),
            kind,
        });
    }
}
