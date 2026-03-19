"""Theme selector modal for choosing color themes."""

from textual.screen import ModalScreen
from textual.widgets import Static, DataTable, Input
from textual.containers import Vertical
from textual.reactive import reactive
from textual.binding import Binding


class ThemeSelector(ModalScreen[str]):
    """Modal screen for selecting a color theme."""
    
    DEFAULT_CSS = """
    ThemeSelector {
        align: center middle;
    }
    
    ThemeSelector > Vertical {
        width: 40;
        height: auto;
        max-height: 20;
        background: $surface;
        padding: 0;
    }
    
    ThemeSelector #theme-header {
        height: 3;
        background: transparent;
        content-align: center middle;
        margin: 1 0 0 0;
    }
    
    ThemeSelector #theme-title {
        color: $text;
        text-style: bold;
    }
    
    ThemeSelector #theme-subtitle {
        color: $text-muted;
    }
    
    ThemeSelector #theme-search {
        height: 1;
        margin: 0 2 1 2;
        display: none;
        border: none;
        border-bottom: solid $primary-darken-2;
        background: transparent;
    }
    
    ThemeSelector #theme-search.visible {
        display: block;
    }
    
    ThemeSelector #theme-search:focus {
        border-bottom: solid $accent;
    }
    
    ThemeSelector #theme-list {
        height: auto;
        max-height: 12;
        margin: 0 2;
        border: none;
        background: transparent;
    }
    
    ThemeSelector #theme-list:focus {
        border: none;
    }
    
    ThemeSelector #theme-list > .datatable--header {
        display: none;
    }
    
    ThemeSelector #theme-list > .datatable--row {
        height: 1;
        background: transparent;
    }
    
    ThemeSelector #theme-list > .datatable--row-hover {
        background: $primary-darken-3;
    }
    
    ThemeSelector #theme-list > .datatable--row-selected {
        background: $accent-darken-2;
        color: $text;
    }
    
    ThemeSelector #theme-help {
        height: 1;
        color: $text-muted;
        content-align: center middle;
        margin: 1 0 1 0;
    }
    """
    
    BINDINGS = [
        Binding("escape", "cancel", "Cancel", priority=True),
        Binding("left", "cancel", "Back", priority=True),
        Binding("slash", "search", "Search", priority=True),
        Binding("j,down", "move_down", "Down", priority=True),
        Binding("k,up", "move_up", "Up", priority=True),
        Binding("enter", "select", "Select", priority=True),
        Binding("1", "jump_1", "Jump 1", priority=True),
        Binding("2", "jump_2", "Jump 2", priority=True),
        Binding("3", "jump_3", "Jump 3", priority=True),
        Binding("4", "jump_4", "Jump 4", priority=True),
        Binding("5", "jump_5", "Jump 5", priority=True),
        Binding("6", "jump_6", "Jump 6", priority=True),
        Binding("7", "jump_7", "Jump 7", priority=True),
        Binding("8", "jump_8", "Jump 8", priority=True),
        Binding("9", "jump_9", "Jump 9", priority=True),
    ]
    
    THEMES = [
        ("default", "textual-dark", "Default"),
        ("dracula", "dracula", "Dracula"),
        ("nord", "nord", "Nord"),
        ("gruvbox", "gruvbox", "Gruvbox"),
        ("catppuccin", "catppuccin-mocha", "Catppuccin"),
        ("tokyo-night", "tokyo-night", "Tokyo Night"),
        ("monokai", "monokai", "Monokai"),
        ("solarized-dark", "solarized-dark", "Solarized"),
        ("rose-pine", "rose-pine", "Rose Pine"),
    ]
    
    def __init__(self, current_theme: str = "default", *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.current_theme = current_theme
        self.filtered_themes = self.THEMES.copy()
        self.search_mode = False
        self.selected_index = 0
    
    def compose(self):
        with Vertical():
            with Vertical(id="theme-header"):
                yield Static("🎨  Theme", id="theme-title")
                yield Static("Select color scheme", id="theme-subtitle")
            yield Input(placeholder="Search...", id="theme-search")
            yield DataTable(id="theme-list")
            yield Static("j/k move · enter select · / search · esc cancel", id="theme-help")
    
    def on_mount(self) -> None:
        table = self.query_one("#theme-list", DataTable)
        table.cursor_type = "row"
        table.show_header = False
        table.add_columns("Name", "Description")
        
        self._populate_list()
        table.focus()
    
    def _populate_list(self) -> None:
        table = self.query_one("#theme-list", DataTable)
        table.clear()
        
        for name, textual_name, desc in self.filtered_themes:
            if name == self.current_theme:
                display_name = f"✓ [accent]{name}[/accent]"
            else:
                display_name = f"  {name}"
            table.add_row(display_name, desc)
        
        if self.selected_index < len(self.filtered_themes):
            table.move_cursor(row=self.selected_index)
    
    def _get_selected_theme(self) -> tuple | None:
        table = self.query_one("#theme-list", DataTable)
        cursor_row = table.cursor_row
        
        if 0 <= cursor_row < len(self.filtered_themes):
            return self.filtered_themes[cursor_row]
        return None
    
    def action_move_down(self) -> None:
        table = self.query_one("#theme-list", DataTable)
        if table.cursor_row < len(self.filtered_themes) - 1:
            table.move_cursor(row=table.cursor_row + 1)
            self.selected_index = table.cursor_row
    
    def action_move_up(self) -> None:
        table = self.query_one("#theme-list", DataTable)
        if table.cursor_row > 0:
            table.move_cursor(row=table.cursor_row - 1)
            self.selected_index = table.cursor_row
    
    def action_jump(self, index: int) -> None:
        if index < len(self.filtered_themes):
            table = self.query_one("#theme-list", DataTable)
            table.move_cursor(row=index)
            self.selected_index = index
    
    def action_jump_1(self) -> None: self.action_jump(0)
    def action_jump_2(self) -> None: self.action_jump(1)
    def action_jump_3(self) -> None: self.action_jump(2)
    def action_jump_4(self) -> None: self.action_jump(3)
    def action_jump_5(self) -> None: self.action_jump(4)
    def action_jump_6(self) -> None: self.action_jump(5)
    def action_jump_7(self) -> None: self.action_jump(6)
    def action_jump_8(self) -> None: self.action_jump(7)
    def action_jump_9(self) -> None: self.action_jump(8)
    
    def action_search(self) -> None:
        self.search_mode = not self.search_mode
        search_input = self.query_one("#theme-search", Input)
        
        if self.search_mode:
            search_input.add_class("visible")
            search_input.focus()
        else:
            search_input.remove_class("visible")
            search_input.value = ""
            self.filtered_themes = self.THEMES.copy()
            self._populate_list()
            table = self.query_one("#theme-list", DataTable)
            table.focus()
    
    def on_input_changed(self, event: Input.Changed) -> None:
        if not self.search_mode:
            return
        
        query = event.value.lower()
        if query:
            self.filtered_themes = [
                t for t in self.THEMES
                if query in t[0].lower() or query in t[2].lower()
            ]
        else:
            self.filtered_themes = self.THEMES.copy()
        
        self.selected_index = 0
        self._populate_list()
    
    def action_select(self) -> None:
        theme = self._get_selected_theme()
        if theme:
            name, textual_name, _ = theme
            self.dismiss(name)
    
    def action_cancel(self) -> None:
        """Cancel theme selection."""
        self.dismiss(None)
    
    def on_data_table_row_selected(self, event: DataTable.RowSelected) -> None:
        self.action_select()
