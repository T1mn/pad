use super::super::app::{App, Focus};
use super::super::layout::LayoutWeights;

const MIN_WEIGHT: u16 = 1;

impl App {
    pub fn grow_focused_section(&mut self) {
        adjust_layout(&mut self.layout_weights, self.focus, true);
    }

    pub fn shrink_focused_section(&mut self) {
        adjust_layout(&mut self.layout_weights, self.focus, false);
    }

    pub fn reset_layout(&mut self) {
        self.layout_weights = LayoutWeights::default();
    }
}

fn adjust_layout(weights: &mut LayoutWeights, focus: Focus, grow: bool) {
    if grow {
        add_to_focus(weights, focus, 1);
        shrink_largest_other(weights, focus);
    } else if focused_weight(weights, focus) > MIN_WEIGHT {
        add_to_focus(weights, focus, -1);
        grow_largest_other(weights, focus);
    }
}

fn focused_weight(weights: &LayoutWeights, focus: Focus) -> u16 {
    match focus {
        Focus::Tree => weights.tree,
        Focus::IndexMap => weights.index_map,
        Focus::Changes => weights.changes,
    }
}

fn add_to_focus(weights: &mut LayoutWeights, focus: Focus, delta: i16) {
    let value = match focus {
        Focus::Tree => &mut weights.tree,
        Focus::IndexMap => &mut weights.index_map,
        Focus::Changes => &mut weights.changes,
    };
    *value = (*value as i16 + delta).max(MIN_WEIGHT as i16) as u16;
}

fn shrink_largest_other(weights: &mut LayoutWeights, focus: Focus) {
    match largest_other(weights, focus, true) {
        Some(Focus::Tree) => weights.tree -= 1,
        Some(Focus::IndexMap) => weights.index_map -= 1,
        Some(Focus::Changes) => weights.changes -= 1,
        None => {}
    }
}

fn grow_largest_other(weights: &mut LayoutWeights, focus: Focus) {
    match largest_other(weights, focus, false) {
        Some(Focus::Tree) => weights.tree += 1,
        Some(Focus::IndexMap) => weights.index_map += 1,
        Some(Focus::Changes) => weights.changes += 1,
        None => {}
    }
}

fn largest_other(weights: &LayoutWeights, focus: Focus, must_be_shrinkable: bool) -> Option<Focus> {
    [Focus::Tree, Focus::IndexMap, Focus::Changes]
        .into_iter()
        .filter(|candidate| *candidate != focus)
        .filter(|candidate| !must_be_shrinkable || focused_weight(weights, *candidate) > MIN_WEIGHT)
        .max_by_key(|candidate| focused_weight(weights, *candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_focused_section_by_stealing_from_largest_other() {
        let mut weights = LayoutWeights::default();
        adjust_layout(&mut weights, Focus::IndexMap, true);
        assert_eq!(
            (weights.tree, weights.index_map, weights.changes),
            (4, 4, 2)
        );
    }

    #[test]
    fn shrinks_focused_section_and_keeps_total_stable() {
        let mut weights = LayoutWeights::default();
        adjust_layout(&mut weights, Focus::Tree, false);
        assert_eq!(
            (weights.tree, weights.index_map, weights.changes),
            (4, 4, 2)
        );
    }

    #[test]
    fn reset_layout_restores_defaults() {
        let mut app = App::new(std::env::temp_dir(), None);
        app.focus_index_map();
        app.grow_focused_section();
        app.reset_layout();
        assert_eq!(
            (
                app.layout_weights.tree,
                app.layout_weights.index_map,
                app.layout_weights.changes,
            ),
            (5, 3, 2)
        );
    }
}
