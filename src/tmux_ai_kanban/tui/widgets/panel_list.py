"""Panel list widget for displaying AI panels."""

from typing import List

from textual.widgets import DataTable
from textual.message import Message
from textual.reactive import reactive
from textual.timer import Timer

from ...models import AIPanel, AIType

# Spinner characters for active state
SPINNER_CHARS = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]


class PanelList(DataTable):
    """Widget for displaying the list of AI panels."""

    DEFAULT_CSS = """
    PanelList {
        height: 100%;
        border: none;
        background: $surface;
    }
    """

    class Selected(Message):
        """Message sent when a panel is selected."""
        
        def __init__(self, index: int) -> None:
            self.index = index
            super().__init__()

    # Track current panels for selection mapping
    _panels: List[AIPanel] = []
    _selected_index: int = 0
    
    # Spinner animation
    _spinner_index: int = 0
    _spinner_timer: Timer = None

    def __init__(self, *args, **kwargs):
        """Initialize the panel list."""
        super().__init__(*args, **kwargs)
        self.cursor_type = "row"
        self.zebra_stripes = True
        self.show_header = True

    def on_mount(self) -> None:
        """Setup columns on mount."""
        self.add_columns(
            "#",        # Index for quick jump
            "AI",       # AI type emoji
            "Status",   # Active/Idle
            "Location", # session:window.pane
            "Directory",# Working directory (shortened)
            "Git",      # Branch@commit
        )
        
        # Start spinner animation timer
        self._spinner_timer = self.set_interval(0.1, self._update_spinner)

    def update_panels(self, panels: List[AIPanel], selected_index: int = 0) -> None:
        """Update the displayed panels.
        
        Args:
            panels: List of AI panels to display
            selected_index: Currently selected index
        """
        self._panels = panels
        self._selected_index = selected_index
        
        # Clear and repopulate
        self.clear()
        
        for idx, panel in enumerate(panels):
            row = self._format_row(idx, panel)
            self.add_row(*row)
        
        # Update selection
        if panels and 0 <= selected_index < len(panels):
            self.move_cursor(row=selected_index)
    
    def _update_spinner(self) -> None:
        """Update spinner animation for active panels."""
        # Only update if there are active panels visible
        has_active = any(p.is_active for p in self._panels)
        if has_active:
            self._spinner_index = (self._spinner_index + 1) % len(SPINNER_CHARS)
            # Refresh the table to show new spinner state
            self._refresh_spinner_cells()
    
    def _refresh_spinner_cells(self) -> None:
        """Refresh only the status cells with spinners."""
        # For now, just re-render the whole table (simplest approach)
        # In production, you might want to update only changed cells
        if self._panels:
            self.clear()
            for idx, panel in enumerate(self._panels):
                row = self._format_row(idx, panel)
                self.add_row(*row)
            if 0 <= self._selected_index < len(self._panels):
                self.move_cursor(row=self._selected_index)
    
    def _get_status_icon(self, panel: AIPanel) -> str:
        """Get status icon for a panel.
        
        Returns:
            Spinner char if active, checkmark if idle
        """
        if panel.is_active:
            return SPINNER_CHARS[self._spinner_index]
        return "✓"
    
    def on_focus(self) -> None:
        """Handle focus event."""
        # Visual feedback when focused - use tint instead of border
        self.styles.background_tint = "green 10%"
    
    def on_blur(self) -> None:
        """Handle blur event."""
        # Remove tint when not focused
        self.styles.background_tint = "transparent"
    


    def _format_row(self, idx: int, panel: AIPanel) -> tuple:
        """Format a panel as a table row.
        
        Args:
            idx: Row index (0-based, displayed as 1-based)
            panel: AI panel to format
            
        Returns:
            Tuple of column values
        """
        # Index (1-9 displayed, 0 for others)
        display_idx = str(idx + 1) if idx < 9 else ""
        
        # AI type with emoji
        ai_emojis = {
            AIType.CLAUDE: "🟣C",
            AIType.CODEX: "🔵X",
            AIType.KIMI: "🟢K",
            AIType.UNKNOWN: "⚪?",
        }
        ai_display = ai_emojis.get(panel.ai_type, "⚪?")
        
        # Status - spinner for active, checkmark for idle
        status = self._get_status_icon(panel)
        
        # Location: session:window.pane
        location = f"{panel.session}:{panel.window}.{panel.pane}"
        
        # Directory: shortened to last 2 components
        directory = self._shorten_path(panel.working_dir)
        
        # Git: branch@commit(+changes)
        git_info = self._format_git(panel)
        
        return (display_idx, ai_display, status, location, directory, git_info)

    def _shorten_path(self, path: str, max_len: int = 20) -> str:
        """Shorten a path for display.
        
        Args:
            path: Full path
            max_len: Maximum length
            
        Returns:
            Shortened path like ~/p/project
        """
        if not path:
            return ""
        
        # Expand home
        import os
        home = os.path.expanduser("~")
        if path.startswith(home):
            path = "~" + path[len(home):]
        
        # If short enough, return as-is
        if len(path) <= max_len:
            return path
        
        # Try to keep last 2 components
        parts = path.split("/")
        if len(parts) >= 2:
            short = "~/.../" + "/".join(parts[-2:])
            if len(short) <= max_len:
                return short
        
        # Fallback: truncate from start
        if len(path) > max_len:
            return "..." + path[-(max_len-3):]
        return path

    def _format_git(self, panel: AIPanel) -> str:
        """Format git info for display.
        
        Args:
            panel: AI panel
            
        Returns:
            Formatted git string like main@a1b2c3(+2)
        """
        if not panel.git_info:
            return ""
        
        git = panel.git_info
        if git.status.value == "no_git":
            return ""
        
        branch = git.branch or "?"
        # Truncate long branch names
        if len(branch) > 12:
            branch = branch[:10] + ".."
        
        commit = git.short_commit or "?"
        
        if git.changed_files > 0:
            return f"{branch}@{commit}(+{git.changed_files})"
        return f"{branch}@{commit}"

    def on_data_table_row_selected(self, event: DataTable.RowSelected) -> None:
        """Handle row selection."""
        self._selected_index = event.cursor_row
        self.post_message(self.Selected(event.cursor_row))

    def on_data_table_row_highlighted(self, event: DataTable.RowHighlighted) -> None:
        """Handle row highlight (for keyboard navigation)."""
        self._selected_index = event.cursor_row
        self.post_message(self.Selected(event.cursor_row))
