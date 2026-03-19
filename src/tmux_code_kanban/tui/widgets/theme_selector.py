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
        border: solid $primary;
        padding: 0 1;
    }
    
    ThemeSelector #theme-title {
        height: 1;
        background: $primary;
        color: $text;
        content-align: center middle;
        text-style: bold;
        margin: 0 -1;
    }
    
    ThemeSelector #theme-search {
        height: 1;
        margin: 1 0 0 0;
        display: none;
    }
    
    ThemeSelector #theme-search.visible {
        display: block;
    }
    
    ThemeSelector #theme-list {
        height: auto;
        max-height: 15;
        margin: 1 0;
        border: none;
    }
    
    ThemeSelector #theme-list:focus {
        border: none;
    }
    
    ThemeSelector #theme-help {
        height: 1;
        color: $text-muted;
        content-align: center middle;
    }
    """
    
    BINDINGS = [
        Binding("escape", "cancel", "Cancel"),
        Binding("slash", "search", "Search"),
        Binding("j,down", "move_down", "Down"),
        Binding("k,up", "move_up", "Up"),
        Binding("enter", "select", "Select"),
        Binding("1", "jump_1", "Jump 1"),
        Binding("2", "jump_2", "Jump 2"),
        Binding("3", "jump_3", "Jump 3"),
        Binding("4", "jump_4", "Jump 4"),
        Binding("5", "jump_5", "Jump 5"),
        Binding("6", "jump_6", "Jump 6"),
        Binding("7", "jump_7", "Jump 7"),
        Binding("8", "jump_8", "Jump 8"),
        Binding("9", "jump_9", "Jump 9"),
    ]
    
    # Available themes
    THEMES = [
        ("default", "textual-dark", "Default dark theme"),
        ("dark", "textual-dark", "Dark theme"),
        ("light", "textual-light", "Light theme"),
        ("dracula", "dracula", "Dracula dark theme"),
        ("nord", "nord", "Nord color palette"),
        ("gruvbox", "gruvbox", "Gruvbox retro"),
        ("catppuccin", "catppuccin-mocha", "Catppuccin mocha"),
        ("tokyo-night", "tokyo-night", "Tokyo Night"),
        ("monokai", "monokai", "Monokai classic"),
        ("solarized-dark", "solarized-dark", "Solarized dark"),
        ("solarized-light", "solarized-light", "Solarized light"),
        ("rose-pine", "rose-pine", "Rose Pine"),
    ]
    
    def __init__(self, current_theme: str = "default", *args, **kwargs):
        """Initialize theme selector.
        
        Args:
            current_theme: Currently selected theme name
        """
        super().__init__(*args, **kwargs)
        self.current_theme = current_theme
        self.filtered_themes = self.THEMES.copy()
        self.search_mode = False
        self.selected_index = 0
    
    def compose(self):
        """Compose the theme selector."""
        with Vertical():
            yield Static("Select Theme", id="theme-title")
            yield Input(placeholder="Search themes...", id="theme-search")
            yield DataTable(id="theme-list")
            yield Static("↑/k ↓/j  |  Enter to select  |  Esc to cancel", id="theme-help")
    
    def on_mount(self) -> None:
        """Initialize the theme list."""
        table = self.query_one("#theme-list", DataTable)
        table.cursor_type = "row"
        table.show_header = False
        table.add_columns("Name", "Description")
        
        self._populate_list()
        
        # Focus the table
        table.focus()
    
    def _populate_list(self) -> None:
        """Populate the theme list."""
        table = self.query_one("#theme-list", DataTable)
        table.clear()
        
        for idx, (name, textual_name, desc) in enumerate(self.filtered_themes):
            # Mark current theme
            display_name = f"● {name}" if name == self.current_theme else f"  {name}"
            table.add_row(display_name, desc)
        
        # Restore selection
        if self.selected_index < len(self.filtered_themes):
            table.move_cursor(row=self.selected_index)
    
    def _get_selected_theme(self) -> tuple | None:
        """Get the currently selected theme."""
        table = self.query_one("#theme-list", DataTable)
        cursor_row = table.cursor_row
        
        if 0 <= cursor_row < len(self.filtered_themes):
            return self.filtered_themes[cursor_row]
        return None
    
    def action_move_down(self) -> None:
        """Move selection down."""
        table = self.query_one("#theme-list", DataTable)
        if table.cursor_row < len(self.filtered_themes) - 1:
            table.move_cursor(row=table.cursor_row + 1)
            self.selected_index = table.cursor_row
    
    def action_move_up(self) -> None:
        """Move selection up."""
        table = self.query_one("#theme-list", DataTable)
        if table.cursor_row > 0:
            table.move_cursor(row=table.cursor_row - 1)
            self.selected_index = table.cursor_row
    
    def action_jump(self, index: int) -> None:
        """Jump to theme by index."""
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
        """Toggle search mode."""
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
        """Handle search input changes."""
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
        """Select the current theme."""
        theme = self._get_selected_theme()
        if theme:
            name, textual_name, _ = theme
            self.dismiss(name)
    
    def action_cancel(self) -> None:
        """Cancel theme selection."""
        self.dismiss(None)
    
    def on_data_table_row_selected(self, event: DataTable.RowSelected) -> None:
        """Handle row selection via click/enter."""
        self.action_select()
