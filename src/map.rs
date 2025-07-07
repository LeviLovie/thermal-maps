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

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<F: Clone + Eq + std::hash::Hash, T: Clone> Map<F, T> {
    pub fn get_closest_by(&self, dist: impl Fn(&F) -> f32) -> Option<T> {
        self.0
            .iter()
            .min_by(|(a, _), (b, _)| {
                dist(a)
                    .partial_cmp(&dist(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(_, v)| v.clone())
    }
}
