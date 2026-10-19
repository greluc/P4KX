use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Strings {
    pub(crate) bytes: Vec<u8>,
    offsets: HashMap<String, u32>,
}

impl Strings {
    pub(crate) fn offset(&mut self, text: &str) -> u32 {
        if let Some(offset) = self.offsets.get(text) {
            return *offset;
        }
        let offset = to_u32(self.bytes.len());
        self.bytes.extend_from_slice(text.as_bytes());
        self.bytes.push(0);
        self.offsets.insert(text.to_owned(), offset);
        offset
    }
}

/// Returns the 16 stored bytes of a GUID written in text as the hexadecimal number `guid`.
pub(crate) fn guid_bytes(guid: u128) -> [u8; 16] {
    let text = guid.to_be_bytes();
    let mut stored = [0; 16];
    for (position, byte) in text.iter().enumerate() {
        let target = if position < 8 {
            7 - position
        } else {
            23 - position
        };
        if let Some(slot) = stored.get_mut(target) {
            *slot = *byte;
        }
    }
    stored
}

pub(crate) fn names<'a>(names: impl Iterator<Item = &'a str>) -> HashMap<&'a str, u32> {
    names
        .enumerate()
        .map(|(index, name)| (name, to_u32(index)))
        .collect()
}

pub(crate) fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

pub(crate) fn to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
