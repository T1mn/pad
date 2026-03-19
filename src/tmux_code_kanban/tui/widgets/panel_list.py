"""Panel list widget for displaying code panels."""

from typing import List

from textual.widgets import DataTable
from textual.message import Message
from textual.reactive import reactive
from textual.timer import Timer

from ...models import CodePanel, CodeType


class PanelList(DataTable):
    """Widget for displaying the list of code panels."""

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
    _panels: List[CodePanel] = []
    _selected_index: int = 0
    
    # Pulse animation for active state
    _pulse_state: bool = False
    _pulse_timer: Timer = None

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
            "Type",     # Code type emoji
            "Status",   # Active/Idle
            "Location", # session:window.pane
            "Directory",# Working directory (shortened)
            "Git",      # Branch@commit
        )
        
        # Start pulse animation timer (slower, gentler pulse)
        self._pulse_timer = self.set_interval(0.6, self._update_pulse)

    def update_panels(self, panels: List[CodePanel], selected_index: int = 0) -> None:
        """Update the displayed panels.
        
        Args:
            panels: List of code panels to display
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
    
    def _update_pulse(self) -> None:
        """Update pulse animation for active panels."""
        # Only update if there are active panels visible
        has_active = any(p.is_active for p in self._panels)
        if has_active and self.row_count > 0:
            self._pulse_state = not self._pulse_state
            # Use update_cell to only update status column without clearing table
            self._update_active_cells_efficient()
    
    def _update_active_cells_efficient(self) -> None:
        """Efficiently update only the active status cells.
        
        This uses update_cell instead of clear/add_row to preserve cursor position.
        """
        # Status column is at index 2
        STATUS_COL = 2
        
        for row_idx, panel in enumerate(self._panels):
            if row_idx >= self.row_count:
                break
            
            # Only update if panel is active
            if panel.is_active:
                # Toggle between bright and dim for pulse effect
                status = "[bold yellow]⚡[/bold yellow]" if self._pulse_state else "[dim yellow]⚡[/dim yellow]"
                try:
                    self.update_cell_at((row_idx, STATUS_COL), status)
                except Exception:
                    pass
    
    def _get_status_icon(self, panel: CodePanel) -> str:
        """Get status icon for a panel.
        
        Returns:
            Lightning bolt if active, circle if idle
        """
        if panel.is_active:
            return "[bold yellow]⚡[/bold yellow]"
        return "[dim]○[/dim]"
    
    def on_focus(self) -> None:
        """Handle focus event."""
        # Visual feedback when focused - use tint instead of border
        self.styles.background_tint = "green 10%"
    
    def on_blur(self) -> None:
        """Handle blur event."""
        # Remove tint when not focused
        self.styles.background_tint = "transparent"

    def _format_row(self, idx: int, panel: CodePanel) -> tuple:
        """Format a panel as a table row.
        
        Args:
            idx: Row index (0-based, displayed as 1-based)
            panel: Code panel to format
            
        Returns:
            Tuple of column values
        """
        # Index (1-9 displayed, 0 for others)
        display_idx = str(idx + 1) if idx < 9 else ""
        
        # Code type with emoji
        type_emojis = {
            CodeType.CLAUDE: "🟣C",
            CodeType.CODEX: "🔵X",
            CodeType.KIMI: "🟢K",
            CodeType.UNKNOWN: "⚪?",
        }
        type_display = type_emojis.get(panel.code_type, "⚪?")
        
        # Status - lightning for active, circle for idle
        status = self._get_status_icon(panel)
        
        # Location: session:window.pane
        location = f"{panel.session}:{panel.window}.{panel.pane}"
        
        # Directory: shortened to last 2 components
        directory = self._shorten_path(panel.working_dir)
        
        # Git: branch@commit(+changes)
        git_info = self._format_git(panel)
        
        return (display_idx, type_display, status, location, directory, git_info)

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

    def _format_git(self, panel: CodePanel) -> str:
        """Format git info for display.
        
        Args:
            panel: Code panel
            
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
