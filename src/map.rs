use std::cmp::Ord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map<F, T>(Vec<(F, T)>);

impl<F: Ord, T: Clone> Map<F, T> {
    pub fn new() -> Map<F, T> {
        Map(Vec::new())
    }

    pub fn push(&mut self, key: F, value: T) {
        if let Some(pos) = self.0.iter().position(|(k, _)| k == &key) {
            self.0[pos].1 = value;
        } else {
            self.0.push((key, value));
        }
    }

    pub fn get_closest(&self, key: &F) -> Option<T>
    where
        F: PartialOrd,
    {
        self.0
            .iter()
            .filter(|(k, _)| k <= key)
            .max_by_key(|(k, _)| k)
            .map(|(_, v)| v.clone())
    }
}
