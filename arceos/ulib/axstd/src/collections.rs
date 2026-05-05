#[doc(no_inline)]
pub use alloc::collections::*;

#[doc(no_inline)]
pub use alloc::collections::{btree_map as hash_map, btree_set as hash_set};

pub type HashMap<K, V> = BTreeMap<K, V>;
pub type HashSet<T> = BTreeSet<T>;
