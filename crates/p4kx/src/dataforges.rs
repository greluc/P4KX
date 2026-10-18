use anyhow::{Context, Result};
use p4kx_comparison::{Archive, DataForge, DataForgeEntry};

/// Reads the DataForge Entries of both Archives at the same time.
pub(crate) fn read_both(old: &Archive, new: &Archive) -> Result<(DataForgeEntry, DataForgeEntry)> {
    std::thread::scope(|scope| {
        let new_entry = scope.spawn(|| read(new));
        let old_entry = read(old);
        let new_entry = new_entry
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        Ok((old_entry?, new_entry?))
    })
}

fn read(archive: &Archive) -> Result<DataForgeEntry> {
    DataForgeEntry::read(archive)
        .with_context(|| format!("cannot read the DataForge of {}", archive.path().display()))
}

/// Opens the DataForges of both Entries.
pub(crate) fn open_both<'e>(
    old: &'e DataForgeEntry,
    new: &'e DataForgeEntry,
) -> Result<(DataForge<'e>, DataForge<'e>)> {
    let open = |entry: &'e DataForgeEntry, side: &str| {
        entry.open().with_context(|| {
            format!(
                "cannot open the DataForge {} of the {side} Archive",
                entry.path()
            )
        })
    };
    Ok((open(old, "old")?, open(new, "new")?))
}
