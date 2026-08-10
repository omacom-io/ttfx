//! Insertion-ordered string-keyed map with Python-dict iteration semantics.
//! Used everywhere upstream iterates dict values (plan.md §4.3): motion.paths,
//! animation.scenes, effect-level dicts. Small maps use a compact linear scan;
//! larger maps add a lookup index while retaining the canonical entry order.

use std::cell::Cell;
use std::collections::HashMap;

const INDEX_THRESHOLD: usize = 8;
const NO_CACHED_LOOKUP: usize = usize::MAX;

#[derive(Debug, Clone)]
pub struct OrderedMap<V> {
    entries: Vec<(String, V)>,
    index: Option<Box<HashMap<String, usize>>>,
    last_lookup: Cell<usize>,
}

impl<V> OrderedMap<V> {
    pub fn new() -> Self {
        OrderedMap { entries: Vec::new(), index: None, last_lookup: Cell::new(NO_CACHED_LOOKUP) }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.position(key).is_some()
    }

    /// Python dict semantics: overwriting an existing key keeps its position.
    pub fn insert(&mut self, key: String, value: V) {
        if let Some(position) = self.position(&key) {
            self.entries[position].1 = value;
            return;
        }
        let position = self.entries.len();
        if let Some(index) = &mut self.index {
            index.insert(key.clone(), position);
        }
        self.entries.push((key, value));
        self.last_lookup.set(position);
        if self.entries.len() == INDEX_THRESHOLD {
            let index = self
                .entries
                .iter()
                .enumerate()
                .map(|(position, (key, _))| (key.clone(), position))
                .collect();
            self.index = Some(Box::new(index));
        }
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.position(key).map(|position| &self.entries[position].1)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let position = self.position(key)?;
        Some(&mut self.entries[position].1)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// Python dict.pop(key, None): removes the entry, preserving the order of
    /// the remaining entries.
    pub fn remove(&mut self, key: &str) -> Option<V> {
        let pos = self.position(key)?;
        if let Some(index) = &mut self.index {
            index.remove(key);
            for indexed_position in index.values_mut() {
                if *indexed_position > pos {
                    *indexed_position -= 1;
                }
            }
        }
        self.last_lookup.set(NO_CACHED_LOOKUP);
        Some(self.entries.remove(pos).1)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_lookup.set(NO_CACHED_LOOKUP);
        if let Some(index) = &mut self.index {
            index.clear();
        }
    }

    fn position(&self, key: &str) -> Option<usize> {
        let cached = self.last_lookup.get();
        if cached < self.entries.len() && self.entries[cached].0 == key {
            return Some(cached);
        }
        let position = match &self.index {
            Some(index) => index.get(key).copied(),
            None => self.entries.iter().position(|(entry_key, _)| entry_key == key),
        };
        self.last_lookup.set(position.unwrap_or(NO_CACHED_LOOKUP));
        position
    }
}

impl<V> Default for OrderedMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_maps_preserve_order_and_lookup_after_mutation() {
        let mut map = OrderedMap::new();
        for value in 0..12 {
            map.insert(value.to_string(), value);
        }
        map.insert("3".to_string(), 30);
        assert_eq!(map.get("3"), Some(&30));
        assert_eq!(map.keys().map(String::as_str).collect::<Vec<_>>()[..5], ["0", "1", "2", "3", "4"]);

        assert_eq!(map.remove("4"), Some(4));
        assert_eq!(map.get("5"), Some(&5));
        assert!(!map.contains_key("4"));

        map.clear();
        map.insert("fresh".to_string(), 99);
        assert_eq!(map.get("fresh"), Some(&99));
    }
}
