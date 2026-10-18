use std::io;

use p4kx_dataforge::{RecordJson, RecordTree, RecordXml};

/// The file format of a Record export with one file per Record: how a Record tree becomes the
/// bytes of its file.
pub trait RecordFileFormat: Default + Send {
    /// The extension of every file, with its dot, e.g. `.xml`.
    const EXTENSION: &'static str;

    /// Appends the content of the file of `tree` to `out`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    fn write_file(&mut self, tree: &RecordTree<'_>, out: &mut Vec<u8>) -> io::Result<()>;
}

/// Record XML (`docs/record-xml.md`).
impl RecordFileFormat for RecordXml {
    const EXTENSION: &'static str = ".xml";

    fn write_file(&mut self, tree: &RecordTree<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        self.write(tree, out)
    }
}

/// Record JSON (`docs/record-json.md`) in the layout of the writer, pretty by default.
impl RecordFileFormat for RecordJson {
    const EXTENSION: &'static str = ".json";

    fn write_file(&mut self, tree: &RecordTree<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        self.write(tree, out)
    }
}
