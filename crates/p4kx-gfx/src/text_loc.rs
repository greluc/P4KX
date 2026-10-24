// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

const PARAMETER_SEPARATOR: &str = ",P=";
const PLACEHOLDERS: [&str; 5] = ["%ls", "%s", "%S", "%d", "%i"];
const MAX_PARAMETERS: usize = 16;

/// A localisation key a text field holds, with the parameters that follow `,P=`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reference<'a> {
    pub(crate) key: &'a str,
    pub(crate) parameters: Vec<&'a str>,
}

/// Splits `@key,P=first,P=second`; `None` when `text` is no reference.
pub(crate) fn reference(text: &str) -> Option<Reference<'_>> {
    let rest = text.trim().strip_prefix('@')?;
    let mut parts = rest.split(PARAMETER_SEPARATOR);
    let key = parts.next().map(str::trim).filter(|key| !key.is_empty())?;
    Some(Reference {
        key,
        parameters: parts.take(MAX_PARAMETERS).collect(),
    })
}

/// Fills the printf-style placeholders of `template` in order with `parameters`; a placeholder
/// without a parameter stays.
pub(crate) fn fill(template: &str, parameters: &[&str]) -> String {
    let mut parameters = parameters.iter();
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((at, placeholder)) = PLACEHOLDERS
        .iter()
        .filter_map(|&placeholder| rest.find(placeholder).map(|at| (at, placeholder)))
        .min_by_key(|&(at, placeholder)| (at, std::cmp::Reverse(placeholder.len())))
    {
        out.push_str(rest.get(..at).unwrap_or_default());
        match parameters.next() {
            Some(parameter) => out.push_str(parameter),
            None => out.push_str(placeholder),
        }
        rest = rest.get(at + placeholder.len()..).unwrap_or_default();
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{fill, reference};

    #[test]
    fn a_reference_splits_into_key_and_parameters() {
        let found = reference(" @ui_speed,P=120,P=m/s ").unwrap_or_else(|| unreachable!("key"));
        assert_eq!(found.key, "ui_speed");
        assert_eq!(found.parameters, ["120", "m/s"]);
        assert!(reference("speed").is_none() && reference("@").is_none());
    }

    #[test]
    fn placeholders_are_filled_in_order() {
        assert_eq!(fill("%ls of %d (%s)", &["a", "7"]), "a of 7 (%s)");
        assert_eq!(fill("plain", &["x"]), "plain");
    }
}
