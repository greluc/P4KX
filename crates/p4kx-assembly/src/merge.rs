use std::collections::HashMap;

/// Returns the key a port name is matched by: the name in ASCII lower case.
pub(crate) fn port_key(name: &str) -> String {
    name.to_ascii_lowercase()
}

/// Replaces the first item of each port `overrides` names, ignoring ASCII case, and appends the
/// rest in order; `port` returns the port name of an item.
pub(crate) fn merge_by_port<T>(
    items: &mut Vec<T>,
    overrides: impl IntoIterator<Item = T>,
    port: impl Fn(&T) -> &str,
) {
    let mut index: HashMap<String, usize> = HashMap::with_capacity(items.len());
    for (at, item) in items.iter().enumerate() {
        index.entry(port_key(port(item))).or_insert(at);
    }
    for item in overrides {
        let key = port_key(port(&item));
        if let Some(old) = index.get(&key).and_then(|at| items.get_mut(*at)) {
            *old = item;
        } else {
            index.insert(key, items.len());
            items.push(item);
        }
    }
}
