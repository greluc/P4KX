use p4kx_cryxml::Xml;

use crate::ValueOrigin;

/// The deepest element nesting that is read.
pub(crate) const MAX_ELEMENT_DEPTH: usize = 128;

/// The most elements one implementation and its patch may hold.
pub(crate) const MAX_ELEMENTS: usize = 1 << 21;

/// An owned XML element of a text or `CryXml` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Element {
    pub(crate) name: String,
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) origins: Vec<(String, ValueOrigin)>,
    pub(crate) children: Vec<Self>,
}

impl Element {
    /// Creates an element read from the base file: every attribute is [`ValueOrigin::Base`].
    pub(crate) const fn read(
        name: String,
        attributes: Vec<(String, String)>,
        children: Vec<Self>,
    ) -> Self {
        Self {
            name,
            attributes,
            origins: Vec::new(),
            children,
        }
    }

    /// Returns where the attribute `name` got its value, ignoring ASCII case.
    pub(crate) fn origin_of(&self, name: &str) -> ValueOrigin {
        self.origins
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map_or(ValueOrigin::Base, |(_, origin)| *origin)
    }

    /// Records that the attribute `name` got its value from `origin`.
    pub(crate) fn mark(&mut self, name: &str, origin: ValueOrigin) {
        if let Some((_, old)) = self
            .origins
            .iter_mut()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
        {
            *old = origin;
        } else {
            self.origins.push((name.to_owned(), origin));
        }
    }

    /// Records `origin` for every attribute of the element and of everything below it.
    pub(crate) fn mark_all(&mut self, origin: ValueOrigin) {
        self.origins = self
            .attributes
            .iter()
            .map(|(name, _)| (name.clone(), origin))
            .collect();
        for child in &mut self.children {
            child.mark_all(origin);
        }
    }

    /// Returns the value of the attribute `name`, ignoring ASCII case.
    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Sets the attribute `name`, ignoring ASCII case, or appends it.
    pub(crate) fn set_attribute(&mut self, name: &str, value: &str) {
        if let Some((_, old)) = self
            .attributes
            .iter_mut()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
        {
            value.clone_into(old);
        } else {
            self.attributes.push((name.to_owned(), value.to_owned()));
        }
    }

    /// Returns the first child element named `name`, ignoring ASCII case.
    pub(crate) fn child(&self, name: &str) -> Option<&Self> {
        self.children
            .iter()
            .find(|child| child.name.eq_ignore_ascii_case(name))
    }

    /// Returns the child elements named `name`, ignoring ASCII case.
    pub(crate) fn children_named<'s, 'n>(
        &'s self,
        name: &'n str,
    ) -> impl Iterator<Item = &'s Self> + use<'s, 'n> {
        self.children
            .iter()
            .filter(move |child| child.name.eq_ignore_ascii_case(name))
    }
}

/// Returns an error when the elements of the text XML `text` nest deeper than
/// [`MAX_ELEMENT_DEPTH`], before the parser recurses that deep.
pub(crate) fn check_depth(text: &str) -> Result<(), String> {
    let bytes = text.as_bytes();
    let mut depth = 0_usize;
    let mut at = 0;
    while let Some(offset) = bytes
        .get(at..)
        .and_then(|rest| rest.iter().position(|b| *b == b'<'))
    {
        at = at.saturating_add(offset);
        let rest = bytes.get(at..).unwrap_or_default();
        if rest.starts_with(b"<!--") {
            at = skip_past(bytes, at.saturating_add(4), b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            at = skip_past(bytes, at.saturating_add(9), b"]]>");
        } else if rest.starts_with(b"<?") {
            at = skip_past(bytes, at.saturating_add(2), b"?>");
        } else if rest.starts_with(b"<!") {
            at = skip_past(bytes, at.saturating_add(2), b">");
        } else {
            let end = tag_end(bytes, at);
            let tag = bytes.get(at..end).unwrap_or_default();
            if tag.starts_with(b"</") {
                depth = depth.saturating_sub(1);
            } else if !tag.ends_with(b"/>") {
                depth = depth.saturating_add(1);
                if depth > MAX_ELEMENT_DEPTH {
                    return Err(format!(
                        "its elements are nested deeper than {MAX_ELEMENT_DEPTH}"
                    ));
                }
            }
            at = end;
        }
    }
    Ok(())
}

/// Returns the offset after the first `needle` at or after `from`, or the end of `bytes`.
fn skip_past(bytes: &[u8], from: usize, needle: &[u8]) -> usize {
    bytes
        .get(from..)
        .and_then(|rest| {
            rest.windows(needle.len())
                .position(|window| window == needle)
        })
        .map_or(bytes.len(), |found| {
            from.saturating_add(found).saturating_add(needle.len())
        })
}

/// Returns the offset after the `>` that ends the tag starting at `from`, skipping quoted
/// attribute values.
fn tag_end(bytes: &[u8], from: usize) -> usize {
    let mut quote = None;
    for (offset, byte) in bytes.iter().enumerate().skip(from.saturating_add(1)) {
        match (quote, *byte) {
            (None, b'"' | b'\'') => quote = Some(*byte),
            (Some(open), b) if b == open => quote = None,
            (None, b'>') => return offset.saturating_add(1),
            _ => {}
        }
    }
    bytes.len()
}

/// The most attributes the elements of one implementation and its patch may hold.
const MAX_ATTRIBUTES: usize = 1 << 22;

/// The least bytes of names and values a document may be read into.
const MIN_BYTES: usize = 1 << 16;

/// What reading and patching one implementation may spend: elements, attributes, and bytes of
/// names and values copied, the last at four times the input read.
#[derive(Debug)]
pub(crate) struct Budget {
    elements: usize,
    attributes: usize,
    bytes: usize,
}

impl Budget {
    /// Creates a budget for reading `input` bytes.
    pub(crate) fn new(input: usize) -> Self {
        Self {
            elements: MAX_ELEMENTS,
            attributes: MAX_ATTRIBUTES,
            bytes: input.saturating_mul(4).max(MIN_BYTES),
        }
    }

    /// Adds the allowance for reading `input` more bytes.
    pub(crate) const fn grow(&mut self, input: usize) {
        self.bytes = self.bytes.saturating_add(input.saturating_mul(4));
    }

    fn take(left: &mut usize, count: usize, what: &str) -> Result<(), String> {
        *left = left
            .checked_sub(count)
            .ok_or_else(|| format!("it needs more {what} than are allowed"))?;
        Ok(())
    }

    fn element(&mut self, name: &str) -> Result<(), String> {
        Self::take(&mut self.elements, 1, "elements")?;
        Self::take(&mut self.bytes, name.len(), "bytes of names and values")
    }

    fn attribute(&mut self, name: &str, value: &str) -> Result<(), String> {
        Self::take(&mut self.attributes, 1, "attributes")?;
        Self::take(
            &mut self.bytes,
            name.len().saturating_add(value.len()),
            "bytes of names and values",
        )
    }

    /// Charges the cost of copying `element` without its children.
    pub(crate) fn charge_shallow(&mut self, element: &Element) -> Result<(), String> {
        self.element(&element.name)?;
        element
            .attributes
            .iter()
            .try_for_each(|(name, value)| self.attribute(name, value))
    }

    /// Charges the cost of copying `element` with everything below it.
    pub(crate) fn charge_copy(&mut self, element: &Element) -> Result<(), String> {
        self.element(&element.name)?;
        for (name, value) in &element.attributes {
            self.attribute(name, value)?;
        }
        element
            .children
            .iter()
            .try_for_each(|child| self.charge_copy(child))
    }

    /// Charges the cost of setting an attribute on an element with `attributes` attributes.
    pub(crate) fn charge_set(
        &mut self,
        attributes: usize,
        name: &str,
        value: &str,
    ) -> Result<(), String> {
        Self::take(&mut self.attributes, attributes.max(1), "attributes")?;
        Self::take(
            &mut self.bytes,
            name.len().saturating_add(value.len()),
            "bytes of names and values",
        )
    }
}

/// Returns the depth of `element`: 1 for an element without children.
pub(crate) fn depth(element: &Element) -> usize {
    element
        .children
        .iter()
        .map(depth)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

/// Returns the error for a document nested deeper than [`MAX_ELEMENT_DEPTH`].
pub(crate) fn too_deep() -> String {
    format!("its elements are nested deeper than {MAX_ELEMENT_DEPTH}")
}

/// Reads the root element of a text XML or `CryXml` document, spending `budget`.
pub(crate) fn parse(bytes: &[u8], budget: &mut Budget) -> Result<Element, String> {
    match p4kx_cryxml::read(bytes).map_err(|error| error.to_string())? {
        Xml::CryXml(document) => from_cry(document.root(), 0, budget),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text).map_err(|error| error.to_string())?;
            let text = text.trim_start_matches('\u{feff}');
            check_depth(text)?;
            let document = roxmltree::Document::parse(text).map_err(|error| error.to_string())?;
            from_text(document.root_element(), 0, budget)
        }
    }
}

fn from_cry(
    element: p4kx_cryxml::Element<'_>,
    depth: usize,
    budget: &mut Budget,
) -> Result<Element, String> {
    if depth > MAX_ELEMENT_DEPTH {
        return Err(too_deep());
    }
    budget.element(element.name())?;
    let mut attributes = Vec::new();
    for attribute in element.attributes() {
        budget.attribute(attribute.name(), attribute.value())?;
        attributes.push((attribute.name().to_owned(), attribute.value().to_owned()));
    }
    let children = element
        .children()
        .map(|child| from_cry(child, depth.saturating_add(1), budget))
        .collect::<Result<_, _>>()?;
    Ok(Element::read(
        element.name().to_owned(),
        attributes,
        children,
    ))
}

fn from_text(
    element: roxmltree::Node<'_, '_>,
    depth: usize,
    budget: &mut Budget,
) -> Result<Element, String> {
    if depth > MAX_ELEMENT_DEPTH {
        return Err(too_deep());
    }
    budget.element(element.tag_name().name())?;
    let mut attributes = Vec::new();
    for attribute in element.attributes() {
        budget.attribute(attribute.name(), attribute.value())?;
        attributes.push((attribute.name().to_owned(), attribute.value().to_owned()));
    }
    let children = element
        .children()
        .filter(roxmltree::Node::is_element)
        .map(|child| from_text(child, depth.saturating_add(1), budget))
        .collect::<Result<_, _>>()?;
    Ok(Element::read(
        element.tag_name().name().to_owned(),
        attributes,
        children,
    ))
}
