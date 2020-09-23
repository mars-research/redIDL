pub struct FlatMap<K, V> {
    keys: Vec<K>,
    values: Vec<V>
}

impl<K: std::cmp::Eq, V> FlatMap<K, V> {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new()
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let mut iter = self.keys.iter();
        match iter.position(|k| k == &key) {
            Some(_) => panic!(), // Type IDs are always unique
            None => {
                self.keys.push(key);
                self.values.push(value);
            }
        }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let mut iter = self.keys.iter();
        match iter.position(|k| k == &key) {
            Some(index) => Some(&self.values[index]),
            None => None
        }
    }
}
