use std::path::{Component, Path, PathBuf};

/// Returns the path below the target an Entry is written to, or `None` when the Entry path is
/// empty, absolute, carries a drive or stream prefix, names a Windows device, or could otherwise
/// leave the target.
pub(crate) fn relative_path(entry_path: &str) -> Option<PathBuf> {
    let mut relative = PathBuf::new();
    for part in entry_path.split(['\\', '/']) {
        if !is_plain_name(part) {
            return None;
        }
        relative.push(part);
    }
    Some(relative)
}

/// Returns whether two Entry paths are equal when ASCII case and the separator kind are ignored.
pub(crate) fn same_entry_path(left: &str, right: &str) -> bool {
    let separator = |byte: u8| byte == b'\\' || byte == b'/';
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .all(|(a, b)| a.eq_ignore_ascii_case(&b) || (separator(a) && separator(b)))
}

fn is_plain_name(part: &str) -> bool {
    let mut components = Path::new(part).components();
    !part.is_empty()
        && !part.ends_with(['.', ' '])
        && !part.contains([':', '\0'])
        && !is_device_name(part)
        && matches!(
            (components.next(), components.next()),
            (Some(Component::Normal(_)), None)
        )
}

fn is_device_name(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part).trim_end();
    let upper = stem.to_ascii_uppercase();
    match upper.as_bytes() {
        b"CON" | b"PRN" | b"AUX" | b"NUL" | b"CONIN$" | b"CONOUT$" => true,
        [b'C', b'O', b'M', digit] | [b'L', b'P', b'T', digit] => digit.is_ascii_digit(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::relative_path;
    use std::path::PathBuf;

    #[test]
    fn plain_entry_paths_map_to_nested_relative_paths() {
        assert_eq!(
            relative_path("Data\\Libs/a.xml"),
            Some(["Data", "Libs", "a.xml"].iter().collect::<PathBuf>()),
            "both separators split the path"
        );
    }

    #[test]
    fn names_with_inner_dots_are_plain() {
        assert!(
            relative_path("Data\\..a\\b..c.xml").is_some(),
            "dots inside a name are fine"
        );
        assert!(
            relative_path("console\\com10.txt").is_some(),
            "names that only start like a device are fine"
        );
    }

    #[test]
    fn unsafe_entry_paths_are_rejected() {
        for path in [
            "..",
            "a\\..\\..\\b",
            ".\\a",
            "\\a",
            "a\\\\b",
            "C:\\a",
            "C:a",
            "a\\b:stream",
            "a\\b.",
            "a\\b ",
            "a\\\0b",
            "a\\NUL",
            "nul.txt",
            "a\\Com1.log",
            "lpt9",
            "CONOUT$",
            "",
        ] {
            assert_eq!(relative_path(path), None, "{path:?} is rejected");
        }
    }
}
