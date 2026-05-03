#[derive(Clone, Copy)]
pub struct LayoutWeights {
    pub tree: u16,
    pub index_map: u16,
    pub changes: u16,
}

impl Default for LayoutWeights {
    fn default() -> Self {
        Self {
            tree: 5,
            index_map: 3,
            changes: 2,
        }
    }
}
