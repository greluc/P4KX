use rayon::slice::ParallelSliceMut;

use crate::build::{RecordText, record_tree};
use crate::dataforge::out_of_range;
use crate::rows::RecordDef;
use crate::strings::{compare_paths, strip_xml};
use crate::{DataForge, DataForgeError, Guid, RecordTree, Struct};

/// One Record of a DataForge: its GUID, name, type and record path.
#[derive(Debug, Clone, Copy)]
pub struct Record<'f> {
    forge: &'f DataForge<'f>,
    index: u32,
    def: RecordDef,
}

impl<'f> Record<'f> {
    /// Returns the position of the Record in definition order, as [`DataForge::records`] lists it.
    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// Returns the GUID that identifies the Record.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        Guid::from_stored(self.def.guid)
    }

    /// Returns the name, e.g. `EntityClassDefinition.AEGS_Avenger`.
    ///
    /// # Errors
    ///
    /// Returns [`DataForgeError::InvalidString`] when the name offset is not a valid string.
    pub fn name(&self) -> Result<&'f str, DataForgeError> {
        self.forge.blob.get(self.def.name)
    }

    /// Returns the record path: the file the Record is defined in, without its `.xml` extension,
    /// e.g. `libs/foundry/records/entities/spaceships/aegs_avenger`. Several Records may share one.
    ///
    /// # Errors
    ///
    /// Returns [`DataForgeError::InvalidString`] when the file name offset is not a valid string.
    pub fn record_path(&self) -> Result<&'f str, DataForgeError> {
        self.forge.text.get(self.def.file_name).map(strip_xml)
    }

    /// Returns the Record tag version 8 stores with every Record, e.g. `SystemsDesign`, or `None` for
    /// version 6, which has no tags.
    ///
    /// # Errors
    ///
    /// Returns [`DataForgeError::InvalidString`] when the tag offset is not a valid string.
    pub fn tag(&self) -> Result<Option<&'f str>, DataForgeError> {
        self.def.tag.map(|tag| self.forge.blob.get(tag)).transpose()
    }

    /// Returns the struct the Record is an instance of.
    ///
    /// # Errors
    ///
    /// Returns [`DataForgeError::IndexOutOfRange`] when the struct does not exist.
    pub fn struct_type(&self) -> Result<Struct<'f>, DataForgeError> {
        self.forge.struct_at(self.def.struct_index).ok_or_else(|| {
            out_of_range(
                "Record",
                self.index as usize,
                "struct",
                self.def.struct_index.into(),
            )
        })
    }

    /// Returns the name of the Record's struct, its type.
    ///
    /// # Errors
    ///
    /// Returns a [`DataForgeError`] when the struct or its name cannot be read.
    pub fn type_name(&self) -> Result<&'f str, DataForgeError> {
        self.struct_type()?.name()
    }

    /// Builds the canonical Record tree: type, GUID, record path, name and every field in
    /// declaration order, the inherited ones first.
    ///
    /// A value that cannot be read becomes a [`Value::Error`](crate::Value::Error) at its field.
    ///
    /// # Errors
    ///
    /// Returns a [`DataForgeError`] when the Record's instance, its struct layout or a name cannot
    /// be read: a cycle or more than [`MAX_INHERITANCE_DEPTH`](crate::MAX_INHERITANCE_DEPTH)
    /// structs in the parent chain, fields that do not fill the declared size, an unknown data
    /// type or an inline struct that does not exist.
    pub fn tree(&self) -> Result<RecordTree<'f>, DataForgeError> {
        let text = RecordText {
            type_name: self.type_name()?,
            record_path: self.record_path()?,
            name: self.name()?,
        };
        record_tree(self.forge, self.index, self.def, text)
    }
}

impl<'a> DataForge<'a> {
    /// Returns every Record in definition order.
    pub fn records(&self) -> impl ExactSizeIterator<Item = Record<'_>> {
        self.records.iter_indexed().map(|(index, def)| Record {
            forge: self,
            index,
            def,
        })
    }

    /// Returns the Record with `guid`, or `None` when there is none.
    #[must_use]
    pub fn record_by_guid(&self, guid: Guid) -> Option<Record<'_>> {
        let index = self.guid_index.get_or_init(|| self.build_guid_index());
        let guid_of = |record: &u32| self.record_def(*record).map(|def| def.guid);
        let first = index.partition_point(|record| guid_of(record) < Some(guid.stored()));
        index
            .get(first)
            .and_then(|record| self.record(*record))
            .filter(|record| record.def.guid == guid.stored())
    }

    /// Returns the Records at a record path in definition order, ignoring ASCII case, the kind of
    /// separator and a trailing `.xml`.
    pub fn records_at_path<'s>(&'s self, path: &str) -> impl Iterator<Item = Record<'s>> + 's {
        let path = strip_xml(path);
        let index = self.path_index.get_or_init(|| self.build_path_index());
        let path_of = |record: &u32| self.path_of(*record).unwrap_or_default();
        let start = index.partition_point(|record| compare_paths(path_of(record), path).is_lt());
        let tail = index.get(start..).unwrap_or_default();
        let len = tail.partition_point(|record| compare_paths(path_of(record), path).is_eq());
        tail.get(..len)
            .unwrap_or_default()
            .iter()
            .filter_map(|record| self.record(*record))
    }

    pub(crate) fn record(&self, index: u32) -> Option<Record<'_>> {
        let def = self.record_def(index)?;
        Some(Record {
            forge: self,
            index,
            def,
        })
    }

    pub(crate) fn record_def(&self, index: u32) -> Option<RecordDef> {
        self.records.get(usize::try_from(index).ok()?)
    }

    pub(crate) fn path_of(&self, index: u32) -> Option<&'a str> {
        let def = self.record_def(index)?;
        self.text.get(def.file_name).ok().map(strip_xml)
    }

    /// Returns the indices of the Records with a readable record path, ordered by record path
    /// and then by definition order.
    pub(crate) fn path_order(&self) -> &[u32] {
        self.path_index.get_or_init(|| self.build_path_index())
    }

    fn build_guid_index(&self) -> Box<[u32]> {
        let mut keyed: Vec<([u8; 16], u32)> = self
            .records
            .iter_indexed()
            .map(|(index, def)| (def.guid, index))
            .collect();
        keyed.par_sort_unstable();
        keyed.into_iter().map(|(_, index)| index).collect()
    }

    fn build_path_index(&self) -> Box<[u32]> {
        let mut keyed: Vec<(&str, u32)> = self
            .records
            .iter_indexed()
            .filter_map(|(index, def)| {
                let path = self.text.get(def.file_name).ok()?;
                Some((strip_xml(path), index))
            })
            .collect();
        keyed.par_sort_unstable_by(|left, right| {
            compare_paths(left.0, right.0).then(left.1.cmp(&right.1))
        });
        keyed.into_iter().map(|(_, index)| index).collect()
    }
}
