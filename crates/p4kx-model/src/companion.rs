use std::error::Error;

/// The failure a [`CompanionResolver`] reports when a companion exists but cannot be read.
pub type CompanionError = Box<dyn Error + Send + Sync>;

/// A file a model asks its resolver for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompanionPath<'a> {
    /// A file in the model's own folder, by file name, e.g. the geometry companion `box.cgfm` of
    /// `box.cgf`.
    Sibling(&'a str),
    /// The material library the model names, as a path below the `Data` folder with `/` as
    /// separator and without the `.mtl` extension, e.g. `Objects/Ships/Gladius/gladius`. The
    /// model has checked that it is a plain path: no empty, `.` or `..` component, no drive.
    Material(&'a str),
}

/// Supplies the companion files of a model, so that reading a model needs no Archive.
pub trait CompanionResolver {
    /// Returns the bytes of the companion at `path`, or `None` when it does not exist.
    ///
    /// # Errors
    ///
    /// Returns the failure to read a companion that exists.
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError>;
}

impl<F> CompanionResolver for F
where
    F: FnMut(CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError>,
{
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError> {
        self(path)
    }
}
