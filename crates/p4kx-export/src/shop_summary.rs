/// What a catalog holds of shops and how the registry entries were read and linked.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShopSummary {
    /// The shops in the catalog.
    pub shops: usize,
    /// The `ShopBackup` entries of the registry.
    pub registry_entries: usize,
    /// Whether the Archive holds no registry Entry at the known path.
    pub registry_missing: bool,
    /// Whether the registry Entry exists but cannot be read.
    pub registry_unreadable: bool,
    /// The entries left out because their `ID` and `SuperName` chains differ in length or hold an
    /// empty element.
    pub malformed: usize,
    /// The entries left out because an earlier entry has the same `ID`.
    pub duplicate: usize,
    /// The shops per star system, ordered by name.
    pub systems: Vec<(String, usize)>,
    /// The shops per kind, ordered by name; shops without a kind are not counted.
    pub kinds: Vec<(String, usize)>,
    /// The shop containers (socpak) found below `PU\Shops` and the shop folders of `PU\loc`.
    pub containers_listed: usize,
    /// The shop containers that cannot be opened or whose `entitycomponentshop.xml` cannot be read.
    pub containers_unreadable: usize,
    /// The shop containers whose `entitycomponentshop.xml` names at least one `SCShop` Entity.
    pub containers_with_shop: usize,
    /// The shops whose Entity exactly one kind and brand of container names.
    pub with_kind: usize,
    /// The shops whose Entity containers of different kind or brand name; kind and brand are `null`.
    pub mixed_kind: usize,
    /// The shops whose Entity no shop container names; kind and brand are `null`.
    pub without_container: usize,
    /// The shops with an item list.
    pub with_items: usize,
    /// The shops whose Entity the containers name with differing inventory files, or only some
    /// of them with one; they have no item list.
    pub inventory_ambiguous: usize,
    /// The shops whose named inventory file the Archive lacks.
    pub inventory_missing: usize,
    /// The shops whose named inventory file cannot be read.
    pub inventory_unreadable: usize,
    /// The shops none of whose containers has the component in a readable `.soc`, so nothing is
    /// known of their inventory; not the same as a component that names no file.
    pub inventory_unscanned: usize,
    /// The (container, Entity) pairs whose component was found in a `.soc`.
    pub soc_components: usize,
    /// The distinct inventory files that cannot be read.
    pub inventories_unreadable: usize,
    /// The distinct named inventory files the Archive lacks.
    pub inventories_missing: usize,
    /// The `ID` values of the distinct inventory files that were no GUID and left out.
    pub item_ids_dropped: usize,
    /// The commodity exchanges and commodity terminals.
    pub commodity_exchanges: usize,
    /// The shops linked to a starmap object by their location container.
    pub linked: usize,
    /// The shops without that link.
    pub unlinked: usize,
    /// The unlinked shops whose location name belongs to more than one starmap object.
    pub ambiguous_link: usize,
    /// The distinct location names of the unlinked shops, up to 200, ordered by name.
    pub unlinked_locations: Vec<String>,
}
