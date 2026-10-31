// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::character_path::is_world_prop_path;
use crate::placement::is_character_item;
use crate::resolver::LoadoutResolver;
use crate::tree::GeometryChoice;
use crate::{Attachment, BodyVariant, EntityNode, MaterialVariant, PortNode};

const HEAD_TYPE: &str = "Char_Head";
const BODY_TYPE: &str = "Char_Body";

/// The body a character wears and the head Entity whose sub-type names it.
#[derive(Debug, Clone, Copy)]
pub struct CharacterBody<'n> {
    variant: BodyVariant,
    head: &'n EntityNode,
}

impl<'n> CharacterBody<'n> {
    /// Returns the body variant the head's sub-type names.
    #[must_use]
    pub const fn variant(&self) -> BodyVariant {
        self.variant
    }

    /// Returns the head Entity that names the body.
    #[must_use]
    pub const fn head(&self) -> &'n EntityNode {
        self.head
    }
}

impl EntityNode {
    /// Returns the body of the character in the tree below this Entity: the one the sub-type of
    /// the first `Char_Head` Entity names; `None` for a tree without a head or with a head whose
    /// sub-type names no body.
    #[must_use]
    pub fn character_body(&self) -> Option<CharacterBody<'_>> {
        let head = find_head(self.ports())?;
        Some(CharacterBody {
            variant: variant_of(head)?,
            head,
        })
    }
}

impl LoadoutResolver<'_> {
    /// Gives the worn Entities of a character the sub-geometry of the character's body.
    ///
    /// The body is the one the sub-type of the `Char_Head` Entity in the tree names; a head
    /// that names none (the generic head of most NPCs) leaves it to the `Char_Body` Entity: its
    /// sub-type, else the folder of its model. An Entity whose item type starts with `Char_`
    /// (armour, clothing, head parts), whose Record names geometry per body (`Male`, `Female`)
    /// and that no port flag gave a sub-geometry takes the one tagged for the body; one without a
    /// sub-geometry for the body has no geometry, and neither has one whose only geometry is a
    /// world prop. A tree without a body is left as it is.
    pub(crate) fn dress_body(&mut self, root: &mut EntityNode) {
        let Some(body) = wearer_body(root) else {
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
        if child.choice != GeometryChoice::Main || !is_character_item(child.attach_type()) {
            return;
        }
        let spec = self.spec(child.guid);
        let Ok(spec) = spec.as_ref() else {
            return;
        };
        let per_body = spec.variants.iter().any(|variant| {
            BodyVariant::ALL
                .iter()
                .any(|other| variant.has_tag(other.tag()))
        });
        if !per_body {
            if child
                .geometry
                .as_ref()
                .is_some_and(|main| is_world_prop_path(main.path()))
            {
                child.choice = GeometryChoice::Body(body);
                child.fallback = None;
                child.geometry = None;
            }
            return;
        }
        child.choice = GeometryChoice::Body(body);
        child.fallback = None;
        child.geometry = spec
            .variants
            .iter()
            .find(|variant| variant.has_tag(body.tag()))
            .map(|variant| {
                let mut worn = variant.geometry().clone();
                worn.material = worn.material.or_else(|| {
                    MaterialVariant::resolve(&spec.material_variants, &[body.tag().to_owned()])
                        .map(str::to_owned)
                });
                worn
            });
    }
}

/// Returns whether `item_type` is the item type of a head, `Char_Head`.
pub(crate) const fn is_head_type(item_type: &str) -> bool {
    item_type.eq_ignore_ascii_case(HEAD_TYPE)
}

/// Returns the first Entity below `ports`, depth first, whose item type `is_type` accepts.
fn find_item(ports: &[PortNode], is_type: fn(&str) -> bool) -> Option<&EntityNode> {
    ports.iter().find_map(|port| {
        if let Attachment::Entity(node) = port.attachment() {
            if is_type(node.attach_type()) {
                return Some(&**node);
            }
            if let Some(found) = find_item(node.ports(), is_type) {
                return Some(found);
            }
        }
        find_item(port.nested(), is_type)
    })
}

/// Returns the first Entity of the item type `Char_Head` below `ports`, depth first.
pub(crate) fn find_head(ports: &[PortNode]) -> Option<&EntityNode> {
    find_item(ports, is_head_type)
}

/// Returns the body a character wears: the one its head names, else the one of its `Char_Body`
/// Entity, by sub-type or by the folder of its model.
fn wearer_body(root: &EntityNode) -> Option<BodyVariant> {
    root.character_body()
        .map(|found| found.variant)
        .or_else(|| {
            let body = find_item(root.ports(), is_body_type)?;
            BodyVariant::from_code(&body.attach_sub_type)
                .or_else(|| BodyVariant::of_model_path(body.geometry()?.path()))
        })
}

/// Returns whether `item_type` is the item type of a body, `Char_Body`.
const fn is_body_type(item_type: &str) -> bool {
    item_type.eq_ignore_ascii_case(BODY_TYPE)
}

/// Returns the body the sub-type of the head Entity `head` names.
pub(crate) fn variant_of(head: &EntityNode) -> Option<BodyVariant> {
    BodyVariant::from_code(&head.attach_sub_type)
}
