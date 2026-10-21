/// Returns an index that is its own ancestor in the forest `parents` describes, or `None` when
/// every chain ends at a root.
pub(crate) fn find_cycle(parents: &[Option<usize>]) -> Option<usize> {
    const UNSEEN: u8 = 0;
    const ON_PATH: u8 = 1;
    const ROOTED: u8 = 2;
    let mut state = vec![UNSEEN; parents.len()];
    let mut path = Vec::new();
    for start in 0..parents.len() {
        let mut at = Some(start);
        while let Some(index) = at {
            match state.get(index).copied() {
                Some(UNSEEN) => {}
                Some(ON_PATH) => return Some(index),
                _ => break,
            }
            if let Some(slot) = state.get_mut(index) {
                *slot = ON_PATH;
            }
            path.push(index);
            at = parents.get(index).copied().flatten();
        }
        for &index in &path {
            if let Some(slot) = state.get_mut(index) {
                *slot = ROOTED;
            }
        }
        path.clear();
    }
    None
}
