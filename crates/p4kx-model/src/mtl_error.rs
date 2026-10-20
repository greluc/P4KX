use p4kx_cryxml::CryXmlError;
use thiserror::Error;

/// A failure to read a `.mtl` file.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MaterialError {
    /// The file is no readable `CryXml`.
    #[error("the file is not valid CryXml")]
    CryXml(#[from] CryXmlError),
    /// The file is no well-formed text XML.
    #[error("the file is not well-formed XML")]
    Xml(#[from] roxmltree::Error),
    /// The root element is no `Material`.
    #[error("the root element is {found}, not Material")]
    NotMaterial {
        /// The root element's name.
        found: String,
    },
    /// The file holds more of something than a material library may.
    #[error("the file holds {count} {what}, more than the limit of {limit}")]
    TooMany {
        /// What there is too much of.
        what: &'static str,
        /// How many there are.
        count: usize,
        /// The most allowed.
        limit: usize,
    },
}
