use crate::bytes::{put_u16, put_u32};

const DEFAULT_VERSION: u32 = 0x9A;
const MEDIA_ALIGNMENT: usize = 16;
const VORBIS_PLUGIN_ID: u32 = 0x0004_0001;

/// One object of a synthetic `HIRC` section, laid out as bank version 154 writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestHircObject {
    kind: u8,
    id: u32,
    body: Vec<u8>,
}

impl TestHircObject {
    /// A Sound object playing the media `source_id`; `storage` 0 embeds it, 1 prefetches and 2
    /// streams it.
    #[must_use]
    pub fn sound(id: u32, source_id: u32, storage: u8, in_memory_size: u32) -> Self {
        Self::sound_with(id, source_id, storage, in_memory_size, true, 0)
    }

    /// A Sound object like [sound](Self::sound) whose media is language-specific: the source
    /// bits mark it, so its WEM lies in a language folder.
    #[must_use]
    pub fn language_sound(id: u32, source_id: u32, storage: u8, in_memory_size: u32) -> Self {
        Self::sound_with(id, source_id, storage, in_memory_size, true, 1)
    }

    /// A Sound object as bank version 150 lays it out: the in-memory media size follows the source
    /// ID directly, without a `cacheID`.
    #[must_use]
    pub fn sound_without_cache_id(
        id: u32,
        source_id: u32,
        storage: u8,
        in_memory_size: u32,
    ) -> Self {
        Self::sound_with(id, source_id, storage, in_memory_size, false, 0)
    }

    fn sound_with(
        id: u32,
        source_id: u32,
        storage: u8,
        in_memory_size: u32,
        cache_id: bool,
        source_bits: u8,
    ) -> Self {
        let mut body = Vec::new();
        put_u32(&mut body, VORBIS_PLUGIN_ID);
        body.push(storage);
        put_u32(&mut body, source_id);
        if cache_id {
            put_u32(&mut body, source_id ^ 0x5A5A_5A5A);
        }
        put_u32(&mut body, in_memory_size);
        body.push(source_bits);
        body.extend_from_slice(&[0; 12]);
        Self { kind: 2, id, body }
    }

    /// An Action object of `action_type`, e.g. `0x0403` for Play, aimed at `target_id`.
    #[must_use]
    pub fn action(id: u32, action_type: u16, target_id: u32) -> Self {
        let mut body = Vec::new();
        put_u16(&mut body, action_type);
        put_u32(&mut body, target_id);
        body.extend_from_slice(&[0, 0, 0, 0, 0, 4, 0x49, 0x6D]);
        Self { kind: 3, id, body }
    }

    /// An Event object running `actions` in order, their count as a variable-length integer.
    #[must_use]
    pub fn event(id: u32, actions: &[u32]) -> Self {
        let mut body = variable_length(u32::try_from(actions.len()).unwrap_or(u32::MAX));
        for action in actions {
            put_u32(&mut body, *action);
        }
        Self { kind: 4, id, body }
    }

    /// An object of any `kind` whose body after the ID is `body`.
    #[must_use]
    pub fn raw(kind: u8, id: u32, body: impl Into<Vec<u8>>) -> Self {
        Self {
            kind,
            id,
            body: body.into(),
        }
    }
}

/// A synthetic Wwise soundbank: `BKHD`, then `DIDX` and `DATA` when it embeds WEMs, then `HIRC`,
/// `STID` and extra sections when given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestBank {
    version: u32,
    bank_id: u32,
    language_id: u32,
    wems: Vec<(u32, Vec<u8>)>,
    hirc: Option<Vec<TestHircObject>>,
    names: Option<Vec<(u32, String)>>,
    extra: Vec<([u8; 4], Vec<u8>)>,
}

impl TestBank {
    /// Creates an empty bank of version 154 with the ID `bank_id`.
    #[must_use]
    pub const fn new(bank_id: u32) -> Self {
        Self {
            version: DEFAULT_VERSION,
            bank_id,
            language_id: 0x1770_5D3E,
            wems: Vec::new(),
            hirc: None,
            names: None,
            extra: Vec::new(),
        }
    }

    /// Sets the bank version `BKHD` declares.
    #[must_use]
    pub const fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Embeds `bytes` as the media `id`, at the next 16-byte boundary of `DATA`.
    #[must_use]
    pub fn wem(mut self, id: u32, bytes: impl Into<Vec<u8>>) -> Self {
        self.wems.push((id, bytes.into()));
        self
    }

    /// Adds an object to the `HIRC` section.
    #[must_use]
    pub fn object(mut self, object: TestHircObject) -> Self {
        self.hirc.get_or_insert_with(Vec::new).push(object);
        self
    }

    /// Adds a bank name to the `STID` section.
    #[must_use]
    pub fn name(mut self, bank_id: u32, name: &str) -> Self {
        self.names
            .get_or_insert_with(Vec::new)
            .push((bank_id, name.to_owned()));
        self
    }

    /// Appends a section that is written as given, after all others.
    #[must_use]
    pub fn section(mut self, id: [u8; 4], body: impl Into<Vec<u8>>) -> Self {
        self.extra.push((id, body.into()));
        self
    }

    /// Returns the bytes of the bank.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut header = Vec::new();
        put_u32(&mut header, self.version);
        put_u32(&mut header, self.bank_id);
        put_u32(&mut header, self.language_id);
        put_u32(&mut header, 0x10);
        put_u32(&mut header, 0x3FB);
        header.resize(0x2C, 0);
        put_section(&mut out, *b"BKHD", &header);
        if !self.wems.is_empty() {
            let (index, data) = self.media();
            put_section(&mut out, *b"DIDX", &index);
            put_section(&mut out, *b"DATA", &data);
        }
        if let Some(objects) = &self.hirc {
            put_section(&mut out, *b"HIRC", &hirc(objects));
        }
        if let Some(names) = &self.names {
            put_section(&mut out, *b"STID", &stid(names));
        }
        for (id, body) in &self.extra {
            put_section(&mut out, *id, body);
        }
        out
    }

    fn media(&self) -> (Vec<u8>, Vec<u8>) {
        let mut index = Vec::new();
        let mut data = Vec::new();
        for (position, (id, bytes)) in self.wems.iter().enumerate() {
            if position > 0 {
                data.resize(data.len().next_multiple_of(MEDIA_ALIGNMENT), 0);
            }
            put_u32(&mut index, *id);
            put_u32(&mut index, len_u32(&data));
            put_u32(&mut index, len_u32(bytes));
            data.extend_from_slice(bytes);
        }
        (index, data)
    }
}

fn hirc(objects: &[TestHircObject]) -> Vec<u8> {
    let mut body = Vec::new();
    put_u32(&mut body, len_u32(objects));
    for object in objects {
        body.push(object.kind);
        put_u32(&mut body, len_u32(&object.body).saturating_add(4));
        put_u32(&mut body, object.id);
        body.extend_from_slice(&object.body);
    }
    body
}

fn stid(names: &[(u32, String)]) -> Vec<u8> {
    let mut body = Vec::new();
    put_u32(&mut body, 1);
    put_u32(&mut body, len_u32(names));
    for (id, name) in names {
        put_u32(&mut body, *id);
        body.push(u8::try_from(name.len()).unwrap_or(u8::MAX));
        body.extend_from_slice(name.as_bytes());
    }
    body
}

fn put_section(out: &mut Vec<u8>, id: [u8; 4], body: &[u8]) {
    out.extend_from_slice(&id);
    put_u32(out, len_u32(body));
    out.extend_from_slice(body);
}

/// Encodes `value` as Wwise writes variable-length integers: 7 bits per byte, most significant
/// first, the high bit set on every byte but the last.
pub(crate) fn variable_length(value: u32) -> Vec<u8> {
    let mut groups = vec![u8::try_from(value & 0x7F).unwrap_or(0)];
    let mut rest = value >> 7;
    while rest > 0 {
        groups.push(u8::try_from(rest & 0x7F).unwrap_or(0) | 0x80);
        rest >>= 7;
    }
    groups.reverse();
    groups
}

fn len_u32<T>(items: &[T]) -> u32 {
    u32::try_from(items.len()).unwrap_or(u32::MAX)
}
