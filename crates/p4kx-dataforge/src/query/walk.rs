use std::ops::ControlFlow;

use crate::build::{element_bytes, pointer, words};
use crate::query::MAX_PATH_VALUES;
use crate::query::reader::{Elements, Select, Step, Walk};
use crate::query::schema::ancestors_include;
use crate::{DataForge, Value, ValueError, ValueKind};

/// The bytes a path reads at one point: an instance of a struct or one value of a pool.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Place<'f> {
    /// An instance of the struct at `struct_index`.
    Instance { struct_index: u32, bytes: &'f [u8] },
    /// The value at `element` in the pool of `kind`.
    Element {
        kind: ValueKind,
        element: u32,
        bytes: &'f [u8],
    },
}

impl<'f> Place<'f> {
    /// Returns the bytes of the instance or value.
    pub(crate) const fn bytes(&self) -> &'f [u8] {
        match self {
            Self::Instance { bytes, .. } | Self::Element { bytes, .. } => bytes,
        }
    }

    /// Returns the error that the bytes end before `needed`.
    pub(crate) fn too_short(&self, forge: &DataForge<'_>, needed: u64) -> ValueError {
        match *self {
            Self::Instance {
                struct_index,
                bytes,
            } => ValueError::InstanceTooShort {
                struct_index,
                size: bytes.len() as u64,
                needed,
            },
            Self::Element { kind, element, .. } => ValueError::ArrayOutOfRange {
                first: element,
                count: 1,
                len: forge.value_count(kind) as u64,
                pool: kind.name(),
            },
        }
    }

    /// Returns the `len` bytes at `at`, or the error that the place ends before them.
    fn field(&self, forge: &DataForge<'_>, at: u64, len: u64) -> Result<&'f [u8], ValueError> {
        let end = at.checked_add(len);
        end.and_then(|end| {
            let range = usize::try_from(at).ok()?..usize::try_from(end).ok()?;
            self.bytes().get(range)
        })
        .ok_or_else(|| self.too_short(forge, end.unwrap_or(u64::MAX)))
    }
}

/// What paths have used of their limits while they are read in one Record: one shared by every
/// column of its row, and one for each comparison of its condition.
#[derive(Debug)]
pub(crate) struct WalkState {
    budget: usize,
}

impl WalkState {
    /// Returns the state of a Record no path has read yet.
    pub(crate) const fn new() -> Self {
        Self {
            budget: MAX_PATH_VALUES,
        }
    }

    /// Counts `elements` more array elements against [`MAX_PATH_VALUES`].
    fn charge(&mut self, elements: usize) -> Result<(), ValueError> {
        self.budget = self
            .budget
            .checked_sub(elements)
            .ok_or(ValueError::PathTooWide {
                limit: MAX_PATH_VALUES,
            })?;
        Ok(())
    }
}

/// What a step reaches: the place the path continues at, or a value that ends it.
enum Reached<'f> {
    Place(Place<'f>),
    Value(Value<'f>),
}

/// The bytes of a pointer and of an array header.
const HEADER_LEN: u64 = 8;

impl Walk {
    /// Calls `emit` with each value the path reaches in order; a null pointer or an index out of
    /// range reaches null and an unreadable step an error value.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::PathTooWide`] when the paths of the Record reach more than
    /// [`MAX_PATH_VALUES`] elements together.
    pub(crate) fn visit<'f>(
        &self,
        forge: &'f DataForge<'f>,
        place: Place<'f>,
        state: &mut WalkState,
        emit: &mut dyn FnMut(Value<'f>) -> ControlFlow<()>,
    ) -> Result<(), ValueError> {
        self.step(forge, 0, place, state, emit).map(|_| ())
    }

    fn step<'f>(
        &self,
        forge: &'f DataForge<'f>,
        index: usize,
        place: Place<'f>,
        state: &mut WalkState,
        emit: &mut dyn FnMut(Value<'f>) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, ValueError> {
        let Some(step) = self.steps.get(index) else {
            return Ok(emit(self.leaf.read(forge, place)));
        };
        let next = index.saturating_add(1);
        let (at, elements, select) = match step {
            Step::Pointer { at, declared } => {
                let reached = match place.field(forge, *at, HEADER_LEN) {
                    Ok(bytes) => deref(forge, bytes, declared),
                    Err(error) => Reached::Value(Value::Error(error)),
                };
                return match reached {
                    Reached::Place(target) => self.step(forge, next, target, state, emit),
                    Reached::Value(value) => Ok(emit(value)),
                };
            }
            Step::Array {
                at,
                elements,
                select,
            } => (*at, elements, select),
        };
        let header = place.field(forge, at, HEADER_LEN);
        let [count, first] = match header {
            Ok(bytes) => words(bytes),
            Err(error) => return Ok(emit(Value::Error(error))),
        };
        if let Err(error) = check_range(forge, elements, first, count) {
            return Ok(emit(Value::Error(error)));
        }
        let indices = match select {
            Select::Index(element) if *element < count => *element..element.saturating_add(1),
            Select::Index(_) => return Ok(emit(Value::Null)),
            Select::All | Select::Type(_) => 0..count,
        };
        state.charge(indices.len())?;
        for element in indices {
            let flow = match element_of(forge, elements, select, first.saturating_add(element)) {
                Some(Reached::Place(target)) => self.step(forge, next, target, state, emit)?,
                Some(Reached::Value(value)) => emit(value),
                None => ControlFlow::Continue(()),
            };
            if flow.is_break() {
                return Ok(flow);
            }
        }
        Ok(ControlFlow::Continue(()))
    }
}

/// Follows a strong pointer to an instance of `declared` or of a struct deriving from it.
fn deref<'f>(forge: &'f DataForge<'f>, bytes: &[u8], declared: &[usize]) -> Reached<'f> {
    let Some((struct_index, instance_index)) = pointer(bytes) else {
        return Reached::Value(Value::Null);
    };
    let Some(target) = forge.instance(struct_index, instance_index) else {
        return Reached::Value(Value::Error(ValueError::InvalidPointer {
            struct_index,
            instance_index,
        }));
    };
    if !derives(forge, struct_index, declared) {
        let declared = declared.first().copied().unwrap_or_default();
        return Reached::Value(Value::Error(ValueError::UnexpectedPointerType {
            struct_index,
            declared: u32::try_from(declared).unwrap_or(u32::MAX),
        }));
    }
    Reached::Place(Place::Instance {
        struct_index,
        bytes: target,
    })
}

fn derives(forge: &DataForge<'_>, struct_index: u32, targets: &[usize]) -> bool {
    usize::try_from(struct_index).is_ok_and(|index| ancestors_include(forge, index, targets))
}

/// Checks that the elements from `first` lie inside the pool or instance block they are read
/// from.
fn check_range(
    forge: &DataForge<'_>,
    elements: &Elements,
    first: u32,
    count: u32,
) -> Result<(), ValueError> {
    let (len, pool) = match elements {
        Elements::Values(kind) => (forge.value_count(*kind) as u64, kind.name()),
        Elements::Pointers(_) => (
            forge.value_count(ValueKind::StrongPointer) as u64,
            ValueKind::StrongPointer.name(),
        ),
        Elements::Instances(target) => match forge.instance_counts.get(*target as usize) {
            Some(count) => (u64::from(*count), "instance block"),
            None => {
                return Err(ValueError::UnknownStruct {
                    index: u16::try_from(*target).unwrap_or(u16::MAX),
                });
            }
        },
    };
    if u64::from(first).saturating_add(u64::from(count)) > len {
        return Err(ValueError::ArrayOutOfRange {
            first,
            count,
            len,
            pool,
        });
    }
    Ok(())
}

/// Returns what the element at `element` in its pool or block reaches, or `None` when
/// `[TypeName]` does not select it.
fn element_of<'f>(
    forge: &'f DataForge<'f>,
    elements: &Elements,
    select: &Select,
    element: u32,
) -> Option<Reached<'f>> {
    let wanted = match select {
        Select::Type(wanted) => Some(&wanted[..]),
        Select::Index(_) | Select::All => None,
    };
    Some(match elements {
        Elements::Values(kind) => Reached::Place(Place::Element {
            kind: *kind,
            element,
            bytes: element_bytes(forge, *kind, element),
        }),
        Elements::Instances(target) => {
            if wanted.is_some_and(|wanted| !derives(forge, *target, wanted)) {
                return None;
            }
            forge.instance(*target, element).map_or(
                Reached::Value(Value::Error(ValueError::InvalidPointer {
                    struct_index: *target,
                    instance_index: element,
                })),
                |bytes| {
                    Reached::Place(Place::Instance {
                        struct_index: *target,
                        bytes,
                    })
                },
            )
        }
        Elements::Pointers(declared) => {
            let bytes = element_bytes(forge, ValueKind::StrongPointer, element);
            if let Some(wanted) = wanted {
                let (struct_index, instance_index) = pointer(bytes)?;
                if forge.instance(struct_index, instance_index).is_some()
                    && !derives(forge, struct_index, wanted)
                {
                    return None;
                }
            }
            deref(forge, bytes, declared)
        }
    })
}
