use schemars::JsonSchema;
use serde::Serialize;

/// The class of an item: an option of the DataForge enum `EItemClass` other than `UNDEFINED`.
///
/// No DataForge field carries it; it is read from the `Class:` line of the item's English
/// description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
pub enum ItemClass {
    /// `Civilian`.
    Civilian,
    /// `Competition`.
    Competition,
    /// `Industrial`.
    Industrial,
    /// `Military`.
    Military,
    /// `Stealth`.
    Stealth,
}

impl ItemClass {
    /// Every item class, in the order of `EItemClass`.
    pub const ALL: [Self; 5] = [
        Self::Civilian,
        Self::Competition,
        Self::Industrial,
        Self::Military,
        Self::Stealth,
    ];

    /// Returns the name, as the catalog writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Civilian => "Civilian",
            Self::Competition => "Competition",
            Self::Industrial => "Industrial",
            Self::Military => "Military",
            Self::Stealth => "Stealth",
        }
    }

    /// Returns the item class named `name`, ignoring ASCII case.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|class| class.name().eq_ignore_ascii_case(name))
    }
}

/// How the item classes of a catalog's items were read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemClassSummary {
    /// The items whose `Class:` line names an [`ItemClass`].
    pub classified: usize,
    /// The items whose `Class:` line holds another value, e.g. the damage type of an FPS weapon;
    /// their class is `null`.
    pub other: usize,
    /// The items without a `Class:` line or without an English description; their class is
    /// `null`.
    pub missing: usize,
}

/// What the `Class:` line of an item's English description says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClassLine {
    /// The line names an item class.
    Known(ItemClass),
    /// The line holds another value.
    Other,
    /// There is no line.
    Missing,
}

impl ClassLine {
    /// Reads the first line of `description` that starts with `Class:`, after leading white
    /// space; its value is trimmed of white space.
    pub(crate) fn read(description: Option<&str>) -> Self {
        let Some(value) = description.and_then(|text| {
            text.lines()
                .find_map(|line| line.trim_start().strip_prefix("Class:"))
        }) else {
            return Self::Missing;
        };
        ItemClass::from_name(value.trim()).map_or(Self::Other, Self::Known)
    }

    /// Returns the item class the line names.
    pub(crate) const fn class(self) -> Option<ItemClass> {
        match self {
            Self::Known(class) => Some(class),
            Self::Other | Self::Missing => None,
        }
    }
}

impl ItemClassSummary {
    /// Counts `line`.
    pub(crate) const fn count(&mut self, line: ClassLine) {
        let counter = match line {
            ClassLine::Known(_) => &mut self.classified,
            ClassLine::Other => &mut self.other,
            ClassLine::Missing => &mut self.missing,
        };
        *counter = counter.saturating_add(1);
    }
}
