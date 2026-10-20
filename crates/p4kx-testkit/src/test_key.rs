use p4kx_foundation::ArchiveKey;

/// The key synthetic Archives are encrypted with; it is not the key of any real Archive.
#[must_use]
pub const fn test_key() -> ArchiveKey {
    ArchiveKey::from_bytes(*b"p4kx-testkit-key")
}

/// A second key, to test that a wrong key is detected.
#[must_use]
pub const fn other_test_key() -> ArchiveKey {
    ArchiveKey::from_bytes(*b"p4kx-other--key!")
}
