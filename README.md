# stacked_type_map
[![version]][crate] [![downloads]][crate] [![docs]][docsrs] [![licence]][licence_link]

This map doesn't use any allocation or hashing.

## Example
```rust
use stacked_type_map::{StackedMap, Map, Removed};

let map = StackedMap;
assert_eq!(map.len(), 0);
let map = map.insert(1).insert(2).insert(3);
assert_eq!(map.get::<String>(), None);
assert_eq!(map.get::<i32>(), Some(&3));
assert_eq!(map.len(), 1);
let map2 = map.clone();
let map3 = map.clone();
assert!(matches!(map.remove::<String>(), Removed::NotFound(_)));
assert!(matches!(
    map2.remove::<i32>(),
    Removed::Removed { map: _, value: 3 }
));
let map = map3.insert(());
let map = map.insert("hi");
assert_eq!(
    map.type_id_iter().collect::<Vec<_>>(),
    vec![TypeId::of::<&'static str>(), TypeId::of::<()>()]
);
```
[crate]: https://crates.io/crates/stacked_type_map
[version]: https://img.shields.io/crates/v/stacked_type_map
[downloads]: https://img.shields.io/crates/d/stacked_type_map
[docs]: https://docs.rs/mio/badge.svg
[docsrs]: https://docs.rs/stacked_type_map
[licence]: https://img.shields.io/crates/l/stacked_type_map
[licence_link]: https://github.com/Stock84-dev/stacked_type_map/blob/main/LICENSE
