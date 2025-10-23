impl<K, V, S> RepeatedCollection<(K, V)> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline]
    fn new_reserved(capacity: usize) -> Self {
        HashMap::with_capacity_and_hasher(capacity, S::default())
    }

    #[inline]
    fn push(&mut self, (key, value): (K, V)) {
        self.insert(key, value);
    }
}

// -----------------------------------------------------------------------------
// BTreeMap<K, V>
// -----------------------------------------------------------------------------
impl<K, V> RepeatedCollection<(K, V)> for BTreeMap<K, V>
where
    K: Ord,
{
    #[inline]
    fn new_reserved(_capacity: usize) -> Self {
        BTreeMap::new()
    }

    #[inline]
    fn push(&mut self, (key, value): (K, V)) {
        self.insert(key, value);
    }
}
