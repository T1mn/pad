"""Settings modal for configuring the application."""

from textual.screen import ModalScreen
from textual.widgets import Static, DataTable, Input, Switch
from textual.containers import Vertical
from textual.reactive import reactive
from textual.binding import Binding
from textual.message import Message


class SettingsModal(ModalScreen[dict]):
    """Modal screen for application settings."""
    
    DEFAULT_CSS = """
    SettingsModal {
        align: center middle;
    }
    
    SettingsModal > Vertical {
        width: 60;
        height: auto;
        max-height: 25;
        background: $surface;
        border: solid $primary;
        padding: 0 1;
    }
    
    SettingsModal #settings-title {
        height: 1;
        background: $primary;
        color: $text;
        content-align: center middle;
        text-style: bold;
        margin: 0 -1;
    }
    
    SettingsModal #settings-search {
        height: 1;
        margin: 1 0 0 0;
        display: none;
    }
    
    SettingsModal #settings-search.visible {
        display: block;
    }
    
    SettingsModal #settings-list {
        height: auto;
        max-height: 18;
        margin: 1 0;
        border: none;
    }
    
    SettingsModal #settings-list:focus {
        border: none;
    }
    
    SettingsModal #settings-help {
        height: 1;
        color: $text-muted;
        content-align: center middle;
        margin: 0 -1 -1 -1;
        background: $surface-darken-1;
    }
    
    SettingsModal .setting-value {
        color: $accent;
        text-style: bold;
    }
    
    SettingsModal .setting-desc {
        color: $text-muted;
    }
    """
    
    BINDINGS = [
        Binding("escape", "close", "Close"),
        Binding("slash", "search", "Search"),
        Binding("j,down", "move_down", "Down"),
        Binding("k,up", "move_up", "Up"),
        Binding("enter", "edit", "Edit"),
        Binding("1", "jump_1", "Jump 1"),
        Binding("2", "jump_2", "Jump 2"),
        Binding("3", "jump_3", "Jump 3"),
        Binding("4", "jump_4", "Jump 4"),
        Binding("5", "jump_5", "Jump 5"),
    ]
    
    class ThemeChanged(Message):
        """Message sent when theme is changed."""
        def __init__(self, theme: str) -> None:
            self.theme = theme
            super().__init__()
    
    class AutoRefreshChanged(Message):
        """Message sent when auto refresh is toggled."""
        def __init__(self, enabled: bool) -> None:
            self.enabled = enabled
            super().__init__()
    
    def __init__(
        self,
        current_theme: str = "default",
        auto_refresh: bool = True,
        refresh_interval: int = 60,
        *args,
        **kwargs
    ):
        """Initialize settings modal.
        
        Args:
            current_theme: Current theme name
            auto_refresh: Whether auto refresh is enabled
            refresh_interval: Refresh interval in seconds
        """
        super().__init__(*args, **kwargs)
        self.current_theme = current_theme
        self.auto_refresh = auto_refresh
        self.refresh_interval = refresh_interval
        self.search_mode = False
        self.selected_index = 0
        
        # Build settings list
        self._all_settings = self._build_settings()
        self.filtered_settings = self._all_settings.copy()
    
    def _build_settings(self) -> list:
        """Build the settings list."""
        return [
            {
                "id": "theme",
                "name": "Theme",
                "value": self.current_theme,
                "description": "Color scheme for the UI",
                "editable": True,
                "type": "select",
            },
            {
                "id": "auto_refresh",
                "name": "Auto Refresh",
                "value": "On" if self.auto_refresh else "Off",
                "description": "Automatically refresh panel list",
                "editable": True,
                "type": "toggle",
            },
            {
                "id": "refresh_interval",
                "name": "Refresh Interval",
                "value": f"{self.refresh_interval}s",
                "description": "Seconds between automatic refreshes",
                "editable": False,
                "type": "number",
            },
            {
                "id": "version",
                "name": "Version",
                "value": "0.2.0",
                "description": "Current version of tmux-code-kanban",
                "editable": False,
                "type": "text",
            },
        ]
    
    def compose(self):
        """Compose the settings modal."""
        with Vertical():
            yield Static("⚙ Settings (F1)", id="settings-title")
            yield Input(placeholder="Search settings...", id="settings-search")
            yield DataTable(id="settings-list")
            yield Static("↑/k ↓/j  |  Enter to edit  |  / to search  |  Esc to close", id="settings-help")
    
    def on_mount(self) -> None:
        """Initialize the settings list."""
        table = self.query_one("#settings-list", DataTable)
        table.cursor_type = "row"
        table.show_header = True
        table.add_columns("Setting", "Value", "Description")
        
        self._populate_list()
        
        # Focus the table
        table.focus()
    
    def _populate_list(self) -> None:
        """Populate the settings list."""
        table = self.query_one("#settings-list", DataTable)
        table.clear()
        
        for idx, setting in enumerate(self.filtered_settings):
            name = setting["name"]
            value = setting["value"]
            desc = setting["description"]
            
            # Style the value
            value_display = f"[accent]{value}[/accent]"
            
            table.add_row(name, value_display, desc)
        
        # Restore selection
        if self.selected_index < len(self.filtered_settings):
            table.move_cursor(row=self.selected_index)
    
    def _get_selected_setting(self) -> dict | None:
        """Get the currently selected setting."""
        table = self.query_one("#settings-list", DataTable)
        cursor_row = table.cursor_row
        
        if 0 <= cursor_row < len(self.filtered_settings):
            return self.filtered_settings[cursor_row]
        return None
    
    def action_move_down(self) -> None:
        """Move selection down."""
        table = self.query_one("#settings-list", DataTable)
        if table.cursor_row < len(self.filtered_settings) - 1:
            table.move_cursor(row=table.cursor_row + 1)
            self.selected_index = table.cursor_row
    
    def action_move_up(self) -> None:
        """Move selection up."""
        table = self.query_one("#settings-list", DataTable)
        if table.cursor_row > 0:
            table.move_cursor(row=table.cursor_row - 1)
            self.selected_index = table.cursor_row
    
    def action_jump(self, index: int) -> None:
        """Jump to setting by index."""
        if index < len(self.filtered_settings):
            table = self.query_one("#settings-list", DataTable)
            table.move_cursor(row=index)
            self.selected_index = index
    
    def action_jump_1(self) -> None: self.action_jump(0)
    def action_jump_2(self) -> None: self.action_jump(1)
    def action_jump_3(self) -> None: self.action_jump(2)
    def action_jump_4(self) -> None: self.action_jump(3)
    def action_jump_5(self) -> None: self.action_jump(4)
    
    def action_search(self) -> None:
        """Toggle search mode."""
        self.search_mode = not self.search_mode
        search_input = self.query_one("#settings-search", Input)
        
        if self.search_mode:
            search_input.add_class("visible")
            search_input.focus()
        else:
            search_input.remove_class("visible")
            search_input.value = ""
            self.filtered_settings = self._all_settings.copy()
            self._populate_list()
            table = self.query_one("#settings-list", DataTable)
            table.focus()
    
    def on_input_changed(self, event: Input.Changed) -> None:
        """Handle search input changes."""
        if not self.search_mode:
            return
        
        query = event.value.lower()
        if query:
            self.filtered_settings = [
                s for s in self._all_settings
                if query in s["name"].lower() or query in s["description"].lower()
            ]
        else:
            self.filtered_settings = self._all_settings.copy()
        
        self.selected_index = 0
        self._populate_list()
    
    def action_edit(self) -> None:
        """Edit the current setting."""
        setting = self._get_selected_setting()
        if not setting or not setting["editable"]:
            return
        
        setting_id = setting["id"]
        
        if setting_id == "theme":
            # Push theme selector screen
            from .theme_selector import ThemeSelector
            self.app.push_screen(
                ThemeSelector(current_theme=self.current_theme),
                self._on_theme_selected
            )
        elif setting_id == "auto_refresh":
            # Toggle auto refresh
            self.auto_refresh = not self.auto_refresh
            setting["value"] = "On" if self.auto_refresh else "Off"
            self._populate_list()
            # Notify parent
            self.post_message(self.AutoRefreshChanged(self.auto_refresh))
    
    def _on_theme_selected(self, theme_name: str | None) -> None:
        """Handle theme selection result."""
        if theme_name:
            self.current_theme = theme_name
            # Update the setting display
            for s in self._all_settings:
                if s["id"] == "theme":
                    s["value"] = theme_name
            self._populate_list()
            # Notify parent
            self.post_message(self.ThemeChanged(theme_name))
    
    def action_close(self) -> None:
        """Close settings modal."""
        result = {
            "theme": self.current_theme,
            "auto_refresh": self.auto_refresh,
            "refresh_interval": self.refresh_interval,
        }
        self.dismiss(result)
