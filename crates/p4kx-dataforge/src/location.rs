use std::fmt::{self, Write as _};

/// One step from a struct down to a value: a field by name or an element of an array by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step<'f> {
    /// The field of this name.
    Field(&'f str),
    /// The element at this index of the array reached so far.
    Element(u32),
}

/// A place inside a Record: the fields and array elements from the Record down to a value.
///
/// Its text names the fields separated by `.`, with `[index]` after an array field, e.g.
/// `scene[1].parent`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location<'f> {
    steps: Box<[Step<'f>]>,
    text: Box<str>,
}

impl<'f> Location<'f> {
    /// Creates the location of `steps`.
    #[must_use]
    pub fn new(steps: &[Step<'f>]) -> Self {
        let mut text = String::new();
        for step in steps {
            match step {
                Step::Field(name) => {
                    if !text.is_empty() {
                        text.push('.');
                    }
                    text.push_str(name);
                }
                Step::Element(index) => {
                    let _infallible = write!(text, "[{index}]");
                }
            }
        }
        Self {
            steps: steps.into(),
            text: text.into(),
        }
    }

    /// Returns the steps from the Record down.
    #[must_use]
    pub fn steps(&self) -> &[Step<'f>] {
        &self.steps
    }

    /// Returns the text form, e.g. `scene[1].parent`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}
