use p4kx_extraction::cryxml;

use crate::starmap_transform::Rotation;

/// An element of text XML or `CryXml`, read alike.
pub(crate) trait Node<'d>: Copy {
    fn name(self) -> &'d str;
    fn attributes(self) -> impl Iterator<Item = (&'d str, &'d str)>;
    fn children(self) -> impl Iterator<Item = Self>;
}

impl<'d> Node<'d> for cryxml::Element<'d> {
    fn name(self) -> &'d str {
        cryxml::Element::name(self)
    }

    fn attributes(self) -> impl Iterator<Item = (&'d str, &'d str)> {
        cryxml::Element::attributes(self).map(|attribute| (attribute.name(), attribute.value()))
    }

    fn children(self) -> impl Iterator<Item = Self> {
        cryxml::Element::children(self)
    }
}

impl<'d> Node<'d> for roxmltree::Node<'d, '_> {
    fn name(self) -> &'d str {
        self.tag_name().name()
    }

    fn attributes(self) -> impl Iterator<Item = (&'d str, &'d str)> {
        roxmltree::Node::attributes(&self).map(|attribute| (attribute.name(), attribute.value()))
    }

    fn children(self) -> impl Iterator<Item = Self> {
        roxmltree::Node::children(&self).filter(roxmltree::Node::is_element)
    }
}

/// Returns the value of the attribute `name` of `node`, ignoring ASCII case.
pub(crate) fn attribute<'d>(node: impl Node<'d>, name: &str) -> Option<&'d str> {
    node.attributes()
        .find(|(have, _)| have.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
}

/// Returns the element named `name` that is a child of `node`, or else the first one below it in
/// document order, ignoring ASCII case.
pub(crate) fn first_below<'d, N: Node<'d>>(node: N, name: &str) -> Option<N> {
    if let Some(direct) = node
        .children()
        .find(|child| child.name().eq_ignore_ascii_case(name))
    {
        return Some(direct);
    }
    let mut pending: Vec<N> = node.children().collect();
    pending.reverse();
    while let Some(next) = pending.pop() {
        if next.name().eq_ignore_ascii_case(name) {
            return Some(next);
        }
        let mark = pending.len();
        pending.extend(next.children());
        if let Some(added) = pending.get_mut(mark..) {
            added.reverse();
        }
    }
    None
}

/// Returns the position and the rotation of a Child: `pos` as `x,y,z` (zero when absent) and
/// `rot` as the quaternion `w,x,y,z` (none when absent); `None` for a value that is malformed.
pub(crate) fn pose<'d>(child: impl Node<'d>) -> Option<([f64; 3], Rotation)> {
    let position = match attribute(child, "pos") {
        Some(text) => numbers::<3>(text)?,
        None => [0.0; 3],
    };
    let rotation = match attribute(child, "rot") {
        Some(text) => Rotation::from_parts(numbers::<4>(text)?)?,
        None => Rotation::IDENTITY,
    };
    Some((position, rotation))
}

fn numbers<const N: usize>(text: &str) -> Option<[f64; N]> {
    let mut parts = text.split(',');
    let mut values = [0.0; N];
    for slot in &mut values {
        let value: f64 = parts.next()?.trim().parse().ok()?;
        if !value.is_finite() {
            return None;
        }
        *slot = value;
    }
    parts.next().is_none().then_some(values)
}
