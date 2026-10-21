use p4kx_cryxml::Xml;

/// The deepest element nesting that is read.
pub(crate) const MAX_ELEMENT_DEPTH: usize = 128;

/// The most elements one document may hold.
pub(crate) const MAX_ELEMENTS: usize = 1 << 21;

/// An owned XML element of a text or `CryXml` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Element {
    pub(crate) name: String,
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) children: Vec<Self>,
}

impl Element {
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

/// Reads the root element of a text XML or `CryXml` document.
pub(crate) fn parse(bytes: &[u8]) -> Result<Element, String> {
    let mut budget = MAX_ELEMENTS;
    match p4kx_cryxml::read(bytes).map_err(|error| error.to_string())? {
        Xml::CryXml(document) => from_cry(document.root(), 0, &mut budget),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text).map_err(|error| error.to_string())?;
            let text = text.trim_start_matches('\u{feff}');
            check_depth(text)?;
            let document = roxmltree::Document::parse(text).map_err(|error| error.to_string())?;
            from_text(document.root_element(), 0, &mut budget)
        }
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
            at = skip_past(bytes, at, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            at = skip_past(bytes, at, b"]]>");
        } else if rest.starts_with(b"<?") {
            at = skip_past(bytes, at, b"?>");
        } else if rest.starts_with(b"<!") {
            at = skip_past(bytes, at, b">");
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

fn spend(depth: usize, budget: &mut usize) -> Result<(), String> {
    if depth > MAX_ELEMENT_DEPTH {
        return Err(format!(
            "its elements are nested deeper than {MAX_ELEMENT_DEPTH}"
        ));
    }
    *budget = budget
        .checked_sub(1)
        .ok_or_else(|| format!("it holds more than {MAX_ELEMENTS} elements"))?;
    Ok(())
}

fn from_cry(
    element: p4kx_cryxml::Element<'_>,
    depth: usize,
    budget: &mut usize,
) -> Result<Element, String> {
    spend(depth, budget)?;
    let children = element
        .children()
        .map(|child| from_cry(child, depth.saturating_add(1), budget))
        .collect::<Result<_, _>>()?;
    Ok(Element {
        name: element.name().to_owned(),
        attributes: element
            .attributes()
            .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
            .collect(),
        children,
    })
}

fn from_text(
    element: roxmltree::Node<'_, '_>,
    depth: usize,
    budget: &mut usize,
) -> Result<Element, String> {
    spend(depth, budget)?;
    let children = element
        .children()
        .filter(roxmltree::Node::is_element)
        .map(|child| from_text(child, depth.saturating_add(1), budget))
        .collect::<Result<_, _>>()?;
    Ok(Element {
        name: element.tag_name().name().to_owned(),
        attributes: element
            .attributes()
            .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
            .collect(),
        children,
    })
}
