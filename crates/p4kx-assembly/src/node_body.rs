// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::resolver::LoadoutResolver;
use crate::{Attachment, BodyVariant, EntityNode, PortNode};

const HEAD_TYPE: &str = "Char_Head";

impl LoadoutResolver<'_> {
    /// Gives the worn Entities of a character the sub-geometry of the character's body.
    ///
    /// The body is the one the sub-type of the `Char_Head` Entity in the tree names. An Entity
    /// whose Record names geometry per body (`Male`, `Female`) and that no port flag gave a
    /// sub-geometry takes the one tagged for the body; one without a sub-geometry for the body
    /// has no geometry. A tree without a head is left as it is.
    pub(crate) fn dress_body(&mut self, root: &mut EntityNode) {
        let Some(body) = body_of(root.ports()) else {
            return;
        };
        self.dress_ports(&mut root.ports, body);
    }

    fn dress_ports(&mut self, ports: &mut [PortNode], body: BodyVariant) {
        for port in ports {
            if let Attachment::Entity(child) = &mut port.attachment {
                self.dress_entity(child, body);
                self.dress_ports(&mut child.ports, body);
            }
            self.dress_ports(&mut port.nested, body);
        }
    }

    fn dress_entity(&mut self, child: &mut EntityNode, body: BodyVariant) {
        let spec = self.spec(child.guid);
        let Ok(spec) = spec.as_ref() else {
            return;
        };
        let main = spec.geometry.as_ref().map(crate::Geometry::path);
        if child.geometry.as_ref().map(crate::Geometry::path) != main {
            return;
        }
        let per_body = spec.variants.iter().any(|variant| {
            BodyVariant::ALL
                .iter()
                .any(|other| variant.has_tag(other.tag()))
        });
        if !per_body {
            return;
        }
        let chosen = spec
            .variants
            .iter()
            .find(|variant| variant.has_tag(body.tag()))
            .map(|variant| variant.geometry().clone());
        child.geometry = chosen.map(|mut tagged| {
            if let Some(main) = &spec.geometry {
                if tagged.material.is_none() {
                    tagged.material.clone_from(&main.material);
                }
                if tagged.palette.is_none() {
                    tagged.palette.clone_from(&main.palette);
                }
            }
            tagged
        });
    }
}

fn body_of(ports: &[PortNode]) -> Option<BodyVariant> {
    ports.iter().find_map(|port| {
        if let Attachment::Entity(node) = port.attachment() {
            if node.attach_type().eq_ignore_ascii_case(HEAD_TYPE) {
                return BodyVariant::from_code(&node.attach_sub_type);
            }
            if let Some(found) = body_of(node.ports()) {
                return Some(found);
            }
        }
        body_of(port.nested())
    })
}
