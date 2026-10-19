const NAME_LEN: usize = 128;

/// Returns the material library a `MtlName` chunk names: the text of its first 128 bytes up to
/// the first NUL, or `None` when that is empty.
pub(crate) fn material_library(data: &[u8]) -> Option<String> {
    let field = data.get(..NAME_LEN).unwrap_or(data);
    let name = field.split(|&byte| byte == 0).next().unwrap_or(field);
    let name = String::from_utf8_lossy(name);
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}
