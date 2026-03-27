use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SharedMap<K: Ord, V>(Rc<BTreeMap<K, V>>);

impl<K: Ord + Clone, V: Clone> SharedMap<K, V> {
    pub(crate) fn new() -> Self {
        Self(Rc::new(BTreeMap::new()))
    }

    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
        Rc::make_mut(&mut self.0).insert(key, value)
    }

    pub(crate) fn remove(&mut self, key: &K) -> Option<V> {
        Rc::make_mut(&mut self.0).remove(key)
    }

    pub(crate) fn entry(&mut self, key: K) -> std::collections::btree_map::Entry<'_, K, V> {
        Rc::make_mut(&mut self.0).entry(key)
    }

    pub(crate) fn iter(&self) -> std::collections::btree_map::Iter<'_, K, V> {
        self.0.iter()
    }

    pub(crate) fn keys(&self) -> std::collections::btree_map::Keys<'_, K, V> {
        self.0.keys()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub(crate) fn contains_key(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }

    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

impl<K: Ord + Clone, V: Clone> Default for SharedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SharedSet<K: Ord>(Rc<BTreeSet<K>>);

impl<K: Ord + Clone> SharedSet<K> {
    pub(crate) fn new() -> Self {
        Self(Rc::new(BTreeSet::new()))
    }

    pub(crate) fn contains(&self, key: &K) -> bool {
        self.0.contains(key)
    }

    pub(crate) fn insert(&mut self, key: K) -> bool {
        Rc::make_mut(&mut self.0).insert(key)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

impl<K: Ord + Clone> Default for SharedSet<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SharedVec<T>(Rc<Vec<T>>);

#[allow(dead_code)]
impl<T: Clone> SharedVec<T> {
    pub(crate) fn new() -> Self {
        Self(Rc::new(Vec::new()))
    }

    pub(crate) fn push(&mut self, value: T) {
        Rc::make_mut(&mut self.0).push(value);
    }

    pub(crate) fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    pub(crate) fn into_vec(self) -> Vec<T> {
        Rc::try_unwrap(self.0).unwrap_or_else(|rc| (*rc).clone())
    }
}

impl<T: Clone> Default for SharedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{SharedMap, SharedSet, SharedVec};
    use std::rc::Rc;

    #[test]
    fn shared_map_supports_basic_accessors() {
        let mut map = SharedMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert!(!map.contains_key(&1));

        assert_eq!(map.insert(1, "one"), None);
        assert_eq!(map.insert(2, "two"), None);

        assert_eq!(map.get(&1), Some(&"one"));
        assert_eq!(map.get(&2), Some(&"two"));
        assert!(map.contains_key(&1));
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
    }

    #[test]
    fn shared_map_clone_shares_until_mutation() {
        let mut original = SharedMap::new();
        original.insert(1, "one");
        let mut cloned = original.clone();

        assert!(Rc::ptr_eq(&original.0, &cloned.0));

        cloned.insert(2, "two");

        assert_eq!(original.get(&1), Some(&"one"));
        assert_eq!(original.get(&2), None);
        assert_eq!(cloned.get(&1), Some(&"one"));
        assert_eq!(cloned.get(&2), Some(&"two"));
        assert!(!Rc::ptr_eq(&original.0, &cloned.0));
    }

    #[test]
    fn shared_map_remove_and_entry_match_btreemap_behavior() {
        let mut map = SharedMap::new();
        map.entry("alpha").or_insert(1);
        map.entry("beta").or_default();
        map.entry("beta").and_modify(|value| *value += 2);

        assert_eq!(map.get(&"alpha"), Some(&1));
        assert_eq!(map.get(&"beta"), Some(&2));
        assert_eq!(map.remove(&"alpha"), Some(1));
        assert_eq!(map.remove(&"missing"), None);
        assert_eq!(map.get(&"alpha"), None);
    }

    #[test]
    fn shared_map_preserves_btree_order() {
        let mut map = SharedMap::new();
        map.insert(3, "three");
        map.insert(1, "one");
        map.insert(2, "two");

        let keys = map.keys().copied().collect::<Vec<_>>();
        let entries = map
            .iter()
            .map(|(key, value)| (*key, *value))
            .collect::<Vec<_>>();

        assert_eq!(keys, vec![1, 2, 3]);
        assert_eq!(entries, vec![(1, "one"), (2, "two"), (3, "three")]);
    }

    #[test]
    fn shared_set_supports_clone_and_ordering() {
        let mut original = SharedSet::new();
        assert!(original.is_empty());
        assert!(original.insert(3));
        assert!(original.insert(1));
        assert!(original.insert(2));
        assert!(!original.insert(2));
        assert!(original.contains(&1));
        assert_eq!(original.len(), 3);

        let mut cloned = original.clone();
        assert!(Rc::ptr_eq(&original.0, &cloned.0));

        assert!(cloned.insert(4));
        assert!(original.contains(&3));
        assert!(!original.contains(&4));
        assert!(cloned.contains(&4));
        assert!(!Rc::ptr_eq(&original.0, &cloned.0));

        let ordered = original.0.iter().copied().collect::<Vec<_>>();
        assert_eq!(ordered, vec![1, 2, 3]);
    }

    #[test]
    fn shared_vec_supports_clone_iteration_and_into_vec() {
        let mut original = SharedVec::new();
        assert!(original.is_empty());
        original.push("first");
        original.push("second");

        let mut cloned = original.clone();
        assert!(Rc::ptr_eq(&original.0, &cloned.0));
        assert_eq!(original.as_slice(), &["first", "second"]);
        assert_eq!(original.iter().copied().collect::<Vec<_>>(), vec!["first", "second"]);
        assert_eq!(original.len(), 2);

        cloned.push("third");

        assert_eq!(original.as_slice(), &["first", "second"]);
        assert_eq!(cloned.as_slice(), &["first", "second", "third"]);
        assert!(!Rc::ptr_eq(&original.0, &cloned.0));
        assert_eq!(cloned.into_vec(), vec!["first", "second", "third"]);
    }
}
