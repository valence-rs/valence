use std::collections::BTreeMap;

use valence_server::registry::tags::RegistryMap;
use valence_server::Ident;

pub(crate) fn default_tags() -> RegistryMap {
    let mut map = RegistryMap::new();

    let mut banner_map = BTreeMap::new();
    map.insert(
        Ident::new_unchecked("minecraft:banner_pattern".to_owned()),
        banner_map,
    );

    map
}
