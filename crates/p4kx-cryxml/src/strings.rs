use std::borrow::Cow;

use crate::error::CryXmlError;

/// The string data of a `CryXml` file: zero-terminated strings addressed by offset.
#[derive(Clone, Copy)]
pub(crate) struct Strings<'a> {
    data: &'a [u8],
}

impl<'a> Strings<'a> {
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns the string at `offset`, borrowed when it is valid UTF-8 and with invalid
    /// sequences replaced otherwise.
    pub(crate) fn get(self, offset: u32) -> Result<Cow<'a, str>, CryXmlError> {
        usize::try_from(offset)
            .ok()
            .and_then(|start| self.data.get(start..))
            .and_then(|tail| tail.get(..tail.iter().position(|&byte| byte == 0)?))
            .map(String::from_utf8_lossy)
            .ok_or(CryXmlError::BadString { offset })
    }
}
