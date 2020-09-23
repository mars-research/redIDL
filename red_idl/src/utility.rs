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

    pub fn insert(&mut self, key: K, value: V) -> bool {
        let mut iter = self.keys.iter();
        match iter.position(|k| k == &key) {
            Some(_) => false,
            None => {
                self.keys.push(key);
                self.values.push(value);
                true
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

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let mut iter = self.keys.iter();
        match iter.position(|k| k == &key) {
            Some(index) => Some(&mut self.values[index]),
            None => None
        }
    }
}
