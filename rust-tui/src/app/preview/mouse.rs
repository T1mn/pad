use super::super::{App, PreviewMouseSelection};

impl App {
    pub fn begin_preview_mouse_selection(&mut self, column: u16, row: u16) {
        self.preview.mouse_selection = Some(PreviewMouseSelection {
            anchor_column: column,
            anchor_row: row,
            current_column: column,
            current_row: row,
        });
        self.dirty = true;
    }

    pub fn update_preview_mouse_selection(&mut self, column: u16, row: u16) -> bool {
        let Some(selection) = self.preview.mouse_selection.as_mut() else {
            return false;
        };

        if selection.current_column == column && selection.current_row == row {
            return false;
        }

        selection.current_column = column;
        selection.current_row = row;
        self.dirty = true;
        true
    }

    pub fn clear_preview_mouse_selection(&mut self) -> bool {
        if self.preview.mouse_selection.take().is_some() {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn finish_preview_mouse_selection(&mut self) -> Option<PreviewMouseSelection> {
        let selection = self.preview.mouse_selection.take();
        if selection.is_some() {
            self.dirty = true;
        }
        selection
    }
}
