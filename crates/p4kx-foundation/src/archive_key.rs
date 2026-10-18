/// The public AES-128 key that `CryEngine` uses for encrypted Entries of `Data.p4k` Archives.
///
/// Encrypted Entries are AES-128-CBC with this key and [`ARCHIVE_IV`].
pub const ARCHIVE_KEY: [u8; 16] = [
    0x5E, 0x7A, 0x20, 0x02, 0x30, 0x2E, 0xEB, 0x1A, 0x3B, 0xB6, 0x17, 0xC3, 0x0F, 0xDE, 0x1E, 0x47,
];

/// The AES-128-CBC IV of encrypted Entries: all zeros.
pub const ARCHIVE_IV: [u8; 16] = [0; 16];
