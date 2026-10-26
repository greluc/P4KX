// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::{Deserialize, Serialize};

use crate::body_reader::{BodyReader, unexpected};
use crate::body_writer::{Put, count_as};
use crate::{ChfError, Guid};

/// The deepest item port nesting read or written; the root is level 1.
pub const MAX_PORT_DEPTH: usize = 32;

const PORT_LEN: usize = 28;

/// One port of the item tree a Character wears, with the item attached to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemPort {
    /// CRC-32C of the lower-cased port name, e.g. `head_itemport`.
    pub port: u32,
    /// GUID of the item attached to the port.
    pub item: Guid,
    /// A count the binary layout keeps on the last port of each level, kept as read.
    pub tail_count: u32,
    /// The ports of the attached item.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Self>,
}

impl ItemPort {
    /// Reads the port count, the tail count behind it and the tree they describe, which is
    /// absent when the count is zero.
    pub(crate) fn read_tree(reader: &mut BodyReader<'_>) -> Result<(Option<Self>, u32), ChfError> {
        let count_offset = reader.offset();
        let stored = reader.u32()?;
        let tail_count = reader.u32()?;
        if stored == 0 {
            return Ok((None, tail_count));
        }
        let root = Self::read(reader, 1)?;
        let actual = root.count();
        if u64::from(stored) != actual {
            return Err(unexpected(
                count_offset,
                "item port count",
                actual.into(),
                stored.into(),
            ));
        }
        Ok((Some(root), tail_count))
    }

    fn read(reader: &mut BodyReader<'_>, depth: usize) -> Result<Self, ChfError> {
        if depth > MAX_PORT_DEPTH {
            return Err(ChfError::TooDeep {
                max: MAX_PORT_DEPTH,
            });
        }
        let port = reader.u32()?;
        let item = reader.guid()?;
        let child_count = reader.u32()?;
        let tail_count = reader.u32()?;
        let child_count = reader.count(child_count.into(), PORT_LEN)?;
        let mut children = Vec::with_capacity(child_count);
        for _ in 0..child_count {
            children.push(Self::read(reader, depth + 1)?);
        }
        Ok(Self {
            port,
            item,
            tail_count,
            children,
        })
    }

    /// Returns the number of ports in this tree, this one included.
    #[must_use]
    pub fn count(&self) -> u64 {
        let mut count = 0_u64;
        let mut pending = vec![self];
        while let Some(port) = pending.pop() {
            count += 1;
            pending.extend(&port.children);
        }
        count
    }

    /// Writes the port count, the tail count behind it and the tree.
    pub(crate) fn write_tree(
        root: Option<&Self>,
        tail_count: u32,
        out: &mut Vec<u8>,
    ) -> Result<(), ChfError> {
        let count =
            u32::try_from(root.map_or(0, Self::count)).map_err(|_| ChfError::Unwritable {
                reason: "the item port tree holds more ports than its count field".to_owned(),
            })?;
        out.put_u32(count);
        out.put_u32(tail_count);
        root.map_or(Ok(()), |root| root.write(out, 1))
    }

    fn write(&self, out: &mut Vec<u8>, depth: usize) -> Result<(), ChfError> {
        if depth > MAX_PORT_DEPTH {
            return Err(ChfError::TooDeep {
                max: MAX_PORT_DEPTH,
            });
        }
        out.put_u32(self.port);
        out.put_guid(self.item);
        out.put_u32(count_as(self.children.len(), "item port children")?);
        out.put_u32(self.tail_count);
        for child in &self.children {
            child.write(out, depth + 1)?;
        }
        Ok(())
    }
}
