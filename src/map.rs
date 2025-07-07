use std::cmp::Ord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map<F, T>(Vec<(F, T)>);

impl<F: Ord, T: Clone> Map<F, T> {
    pub fn new() -> Map<F, T> {
        Map(Vec::new())
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
