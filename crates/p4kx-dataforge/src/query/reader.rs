use std::ops::ControlFlow;
use std::sync::Arc;

use crate::build::{element_bytes, pointer, words};
use crate::query::MAX_PATH_VALUES;
use crate::query::schema::ancestors_include;
use crate::scalar::read_scalar;
use crate::{DataForge, DataType, Value, ValueError, ValueKind};

/// Where a scalar lies in the bytes a path has reached, and how it is read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Column {
    pub(crate) data_type: DataType,
    pub(crate) target: u16,
    pub(crate) start: u64,
    pub(crate) end: u64,
}

impl Column {
    /// Reads the value from `bytes`, or `None` when they are too short.
    pub(crate) fn read<'f>(&self, forge: &'f DataForge<'f>, bytes: &[u8]) -> Option<Value<'f>> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        let bytes = bytes.get(start..end)?;
        Some(read_scalar(forge, self.data_type, self.target, bytes).unwrap_or(Value::Null))
    }
}

/// How a compiled path reads its values from the instance of a Record.
#[derive(Debug, Clone)]
pub(crate) enum Reader {
    /// A scalar at a fixed place in the instance, reached through fields and inline structs only.
    Column(Column),
    /// A path through pointers or array elements.
    Walk(Arc<Walk>),
}

/// The pointer and array steps of a path and the scalar read behind the last of them.
#[derive(Debug)]
pub(crate) struct Walk {
    pub(crate) steps: Box<[Step]>,
    pub(crate) leaf: Column,
    pub(crate) multi: bool,
}

/// One step that leaves the bytes reached so far, at the offset `at` in them.
#[derive(Debug, Clone)]
pub(crate) enum Step {
    /// A strong pointer to an instance of `declared` or of a struct deriving from it.
    Pointer { at: u64, declared: Box<[usize]> },
    /// An array header and the elements it selects.
    Array {
        at: u64,
        elements: Elements,
        select: Select,
    },
}

/// What the elements of an array are.
#[derive(Debug, Clone)]
pub(crate) enum Elements {
    /// Values in the pool of their kind.
    Values(ValueKind),
    /// Instances of the struct at this index.
    Instances(u32),
    /// Strong pointers to instances of `declared` or of a struct deriving from it.
    Pointers(Box<[usize]>),
}

/// Which elements of an array a step selects.
#[derive(Debug, Clone)]
pub(crate) enum Select {
    Index(u32),
    All,
    Type(Box<[usize]>),
}

/// The elements of a path reached so far in one Record, counted against [`MAX_PATH_VALUES`].
struct Budget(usize);

impl Reader {
    /// Returns the reader with every offset into the Record's instance moved by `base`.
    pub(crate) fn shifted(&self, base: u64) -> Self {
        let shift = |column: Column| Column {
            start: column.start.saturating_add(base),
            end: column.end.saturating_add(base),
            ..column
        };
        match self {
            Self::Column(column) => Self::Column(shift(*column)),
            Self::Walk(walk) => {
                let mut steps = walk.steps.clone();
                if let Some(Step::Pointer { at, .. } | Step::Array { at, .. }) = steps.first_mut() {
                    *at = at.saturating_add(base);
                }
                Self::Walk(Arc::new(Walk {
                    steps,
                    leaf: walk.leaf,
                    multi: walk.multi,
                }))
            }
        }
    }

    /// Returns the cell of the path in the Record instance `bytes`: its value, or for a
    /// multi-valued path every value it reaches as a [`Value::Array`]; `None` when a fixed column
    /// lies beyond the instance. Only a multi-valued path allocates, for its list.
    pub(crate) fn cell<'f>(&self, forge: &'f DataForge<'f>, bytes: &[u8]) -> Option<Value<'f>> {
        let walk = match self {
            Self::Column(column) => return column.read(forge, bytes),
            Self::Walk(walk) => walk,
        };
        if !walk.multi {
            let mut single = Value::Null;
            let reached = walk.visit(forge, bytes, &mut |value| {
                single = value;
                ControlFlow::Break(())
            });
            return Some(reached.map_or_else(Value::Error, |()| single));
        }
        let mut values = Vec::new();
        let reached = walk.visit(forge, bytes, &mut |value| {
            values.push(value);
            ControlFlow::Continue(())
        });
        Some(reached.map_or_else(Value::Error, |()| Value::Array(values.into_boxed_slice())))
    }

    /// Returns whether some value of the path holds `test`: `Some(true)` when one does, `None`
    /// when none does but one is unknown, `Some(false)` otherwise, also when there is none.
    pub(crate) fn any<'f>(
        &self,
        forge: &'f DataForge<'f>,
        bytes: &[u8],
        test: impl Fn(&Value<'f>) -> Option<bool>,
    ) -> Option<bool> {
        let walk = match self {
            Self::Column(column) => return test(&column.read(forge, bytes)?),
            Self::Walk(walk) => walk,
        };
        let mut result = Some(false);
        let reached = walk.visit(forge, bytes, &mut |value| match test(&value) {
            Some(true) => {
                result = Some(true);
                ControlFlow::Break(())
            }
            Some(false) => ControlFlow::Continue(()),
            None => {
                result = None;
                ControlFlow::Continue(())
            }
        });
        match (reached, result) {
            (_, Some(true)) => Some(true),
            (Err(_), _) => None,
            (Ok(()), result) => result,
        }
    }
}

impl Walk {
    /// Calls `emit` with each value the path reaches in order; a null pointer or an index out of
    /// range reaches null and an unreadable step an error value.
    ///
    /// # Errors
    ///
    /// Returns [`ValueError::PathTooWide`] when the path reaches more than
    /// [`MAX_PATH_VALUES`] elements.
    fn visit<'f>(
        &self,
        forge: &'f DataForge<'f>,
        bytes: &[u8],
        emit: &mut dyn FnMut(Value<'f>) -> ControlFlow<()>,
    ) -> Result<(), ValueError> {
        let mut budget = Budget(MAX_PATH_VALUES);
        self.step(forge, 0, bytes, &mut budget, emit).map(|_| ())
    }

    fn step<'f>(
        &self,
        forge: &'f DataForge<'f>,
        index: usize,
        bytes: &[u8],
        budget: &mut Budget,
        emit: &mut dyn FnMut(Value<'f>) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, ValueError> {
        let Some(step) = self.steps.get(index) else {
            return Ok(emit(self.leaf.read(forge, bytes).unwrap_or(Value::Null)));
        };
        let next = index.saturating_add(1);
        match step {
            Step::Pointer { at, declared } => match deref(forge, field(bytes, *at), declared) {
                Reached::Bytes(target) => self.step(forge, next, target, budget, emit),
                Reached::Value(value) => Ok(emit(value)),
            },
            Step::Array {
                at,
                elements,
                select,
            } => {
                let [count, first] = words(field(bytes, *at));
                if let Err(error) = check_range(forge, elements, first, count) {
                    return Ok(emit(Value::Error(error)));
                }
                let indices = match select {
                    Select::Index(element) if *element < count => {
                        *element..element.saturating_add(1)
                    }
                    Select::Index(_) => return Ok(emit(Value::Null)),
                    Select::All | Select::Type(_) => 0..count,
                };
                budget.0 = budget
                    .0
                    .checked_sub(indices.len())
                    .ok_or(ValueError::PathTooWide {
                        limit: MAX_PATH_VALUES,
                    })?;
                for element in indices {
                    let reached =
                        element_of(forge, elements, select, first.saturating_add(element));
                    let flow = match reached {
                        Some(Reached::Bytes(target)) => {
                            self.step(forge, next, target, budget, emit)?
                        }
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
    }
}

/// What a step reaches: the bytes the path continues in, or a value that ends it.
enum Reached<'f> {
    Bytes(&'f [u8]),
    Value(Value<'f>),
}

fn field(bytes: &[u8], at: u64) -> &[u8] {
    usize::try_from(at)
        .ok()
        .and_then(|at| bytes.get(at..))
        .unwrap_or_default()
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
    Reached::Bytes(target)
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
        Elements::Values(kind) => Reached::Bytes(element_bytes(forge, *kind, element)),
        Elements::Instances(target) => {
            if wanted.is_some_and(|wanted| !derives(forge, *target, wanted)) {
                return None;
            }
            forge.instance(*target, element).map_or(
                Reached::Value(Value::Error(ValueError::InvalidPointer {
                    struct_index: *target,
                    instance_index: element,
                })),
                Reached::Bytes,
            )
        }
        Elements::Pointers(declared) => {
            let bytes = element_bytes(forge, ValueKind::StrongPointer, element);
            match (wanted, pointer(bytes)) {
                (None, _) => deref(forge, bytes, declared),
                (Some(_), None) => return None,
                (Some(wanted), Some((struct_index, _))) => {
                    if !derives(forge, struct_index, wanted) {
                        return None;
                    }
                    deref(forge, bytes, declared)
                }
            }
        }
    })
}
