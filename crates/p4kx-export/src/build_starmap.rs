use crate::StarmapSummary;
use crate::collected::Collected;
use crate::crafting_warning::CraftingWarning;
use crate::starmap_report::ContainerCounts;

/// Counts the starmap objects of `collected` and what reading their containers found, and warns
/// about containers that left objects without a position.
pub(crate) fn add_starmap(
    collected: &mut Collected<'_>,
    counts: ContainerCounts,
) -> StarmapSummary {
    let objects = collected.starmap_objects.len();
    let placed = collected
        .starmap_objects
        .iter()
        .filter(|object| object.position.is_some())
        .count();
    let summary = StarmapSummary {
        objects,
        placed,
        unplaced: objects.saturating_sub(placed),
        ambiguous: counts.ambiguous,
        systems: counts.systems,
        unreadable_containers: counts.unreadable,
        missing_containers: counts.missing,
        malformed_children: counts.malformed,
    };
    let unplaced_containers = counts.unreadable.saturating_add(counts.malformed);
    if objects > 0 && unplaced_containers > 0 {
        collected
            .warnings
            .push(CraftingWarning::UnplacedStarmapContainers(
                unplaced_containers,
            ));
    }
    summary
}
