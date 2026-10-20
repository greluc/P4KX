use anyhow::{Result, bail};

const KEY_LEN: usize = 16;
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The spellings of one 16-byte key that the exact search looks for.
pub(crate) struct Needles {
    raw: [u8; KEY_LEN],
    hex: Vec<u8>,
    decimal: Vec<u8>,
    base64: Vec<Vec<u8>>,
}

impl Needles {
    /// Parses 32 hexadecimal digits; the error never repeats the text.
    pub(crate) fn from_hex(text: &str) -> Result<Self> {
        let digits: Vec<u8> = text.trim().bytes().collect();
        if digits.len() != KEY_LEN * 2 || !digits.iter().all(u8::is_ascii_hexdigit) {
            bail!("the key given for the search is not 32 hexadecimal digits");
        }
        let mut raw = [0_u8; KEY_LEN];
        for (byte, pair) in raw.iter_mut().zip(digits.chunks(2)) {
            let text = std::str::from_utf8(pair)?;
            *byte = u8::from_str_radix(text, 16)?;
        }
        Ok(Self::from_bytes(raw))
    }

    fn from_bytes(raw: [u8; KEY_LEN]) -> Self {
        let hex = raw
            .iter()
            .flat_map(|byte| format!("{byte:02x}").into_bytes())
            .collect();
        let decimal = raw
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",")
            .into_bytes();
        let base64 = (0..3)
            .filter_map(|skip| {
                let tail = raw.get(skip..)?;
                let whole = tail.len() / 3 * 3;
                Some(encode_base64(tail.get(..whole)?).into_bytes())
            })
            .collect();
        Self {
            raw,
            hex,
            decimal,
            base64,
        }
    }

    /// Returns the spellings of the key found in `bytes`, as names that do not reveal it.
    pub(crate) fn find_in(&self, bytes: &[u8]) -> Vec<&'static str> {
        let mut found = Vec::new();
        if contains(bytes, &self.raw) {
            found.push("the raw bytes");
        }
        let hex = hex_digits(bytes);
        if contains(&hex, &self.hex) {
            found.push("hexadecimal digits or a list of hexadecimal bytes");
        }
        let numbers = without_separators(bytes);
        if contains(&numbers, &self.decimal) {
            found.push("a list of decimal bytes");
        }
        if self.base64.iter().any(|needle| contains(bytes, needle)) {
            found.push("base64");
        }
        found
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// Lower-cases the hexadecimal digits of `bytes` and drops everything else, including `0x`.
fn hex_digits(bytes: &[u8]) -> Vec<u8> {
    let mut digits = Vec::with_capacity(bytes.len() / 2);
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let prefix = byte == b'0' && matches!(bytes.get(index + 1), Some(b'x' | b'X'));
        if prefix {
            index += 2;
            continue;
        }
        if byte.is_ascii_hexdigit() {
            digits.push(byte.to_ascii_lowercase());
        }
        index += 1;
    }
    digits
}

fn without_separators(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace() && !b"[](){}".contains(byte))
        .collect()
}

fn encode_base64(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() / 3 * 4);
    for &[a, b, c] in bytes.as_chunks::<3>().0 {
        let bits = (u32::from(a) << 16) | (u32::from(b) << 8) | u32::from(c);
        for shift in [18, 12, 6, 0] {
            let index = usize::try_from((bits >> shift) & 0x3F).unwrap_or(0);
            text.push(char::from(BASE64.get(index).copied().unwrap_or(b'A')));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::Needles;

    fn sample() -> String {
        (0..16_u8)
            .map(|byte| format!("{:02x}", byte * 7 + 3))
            .collect::<Vec<_>>()
            .concat()
    }

    fn found(needles: &Needles, text: &[u8]) -> bool {
        !needles.find_in(text).is_empty()
    }

    fn needles() -> Needles {
        Needles::from_hex(&sample()).expect("sample key")
    }

    #[test]
    fn every_spelling_is_found() {
        let hex = sample();
        let bytes: Vec<u8> = (0..16_u8).map(|byte| byte * 7 + 3).collect();
        let listed = bytes
            .iter()
            .map(|byte| format!("0x{byte:02X}"))
            .collect::<Vec<_>>()
            .join(", ");
        let decimal = format!(
            "[{}]",
            bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
        let tail = super::encode_base64(&bytes[..15]);
        let texts: [(&str, Vec<u8>); 5] = [
            ("raw", [b"xx".as_slice(), &bytes, b"yy"].concat()),
            (
                "hex",
                format!("key = \"{}\"", hex.to_uppercase()).into_bytes(),
            ),
            ("list", format!("[{listed}]").into_bytes()),
            ("decimal", decimal.into_bytes()),
            ("base64", format!("k: {tail}").into_bytes()),
        ];
        for (name, text) in texts {
            assert!(found(&needles(), &text), "{name} is found");
        }
    }

    #[test]
    fn unrelated_text_is_not_a_match() {
        assert!(
            !found(&needles(), b"fn main() { println!(1, 2, 3); }"),
            "no match"
        );
    }

    #[test]
    fn a_bad_key_is_refused_without_repeating_it() {
        let error = Needles::from_hex("zz").err().expect("refused").to_string();
        assert!(!error.contains("zz"), "{error}");
    }
}
