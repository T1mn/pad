"""TUI Application for tmux-ai-kanban."""

import asyncio
import hashlib
import subprocess
import re
from concurrent.futures import ThreadPoolExecutor
from typing import List, Optional

from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical
from textual.reactive import reactive
from textual.timer import Timer
from textual.widgets import Static, DataTable, Input, Label
from textual.binding import Binding

from ..models import AIType, AIPanel
from ..detector import scan_ai_panels, filter_panels
from ..tmux_client import is_tmux_running, capture_pane, switch_client, run_tmux

from .widgets.panel_list import PanelList
from .widgets.preview import PreviewPanel
from .widgets.status_bar import StatusBar
from .widgets.search_box import SearchBox


class KanbanApp(App):
    """Tmux AI Kanban TUI Application."""

    CSS_PATH = "styles.tcss"
    
    BINDINGS = [
        Binding("q,ctrl+c", "quit", "Quit", priority=True),
        Binding("r", "refresh", "Refresh", priority=True),
        Binding("slash", "search", "Search", priority=True),
        Binding("escape", "cancel_search", "Cancel", priority=True),
        Binding("enter", "attach_popup", "Attach (Popup)", priority=True),
        Binding("s", "attach_session", "Attach (Session)", priority=True),
        Binding("j,down", "move_down", "Down", priority=True),
        Binding("k,up", "move_up", "Up", priority=True),
        Binding("1", "jump_to_1", "Jump 1", priority=True),
        Binding("2", "jump_to_2", "Jump 2", priority=True),
        Binding("3", "jump_to_3", "Jump 3", priority=True),
        Binding("4", "jump_to_4", "Jump 4", priority=True),
        Binding("5", "jump_to_5", "Jump 5", priority=True),
        Binding("6", "jump_to_6", "Jump 6", priority=True),
        Binding("7", "jump_to_7", "Jump 7", priority=True),
        Binding("8", "jump_to_8", "Jump 8", priority=True),
        Binding("9", "jump_to_9", "Jump 9", priority=True),
        Binding("g,home", "move_top", "Top", priority=True),
        Binding("G,end", "move_bottom", "Bottom", priority=True),
        Binding("plus", "next_page", "Next", priority=True),
        Binding("minus", "prev_page", "Prev", priority=True),
    ]

    # Reactive state
    panels: reactive[List[AIPanel]] = reactive([])
    filtered_panels: reactive[List[AIPanel]] = reactive([])
    selected_index: reactive[int] = reactive(0)
    search_query: reactive[str] = reactive("")
    is_searching: reactive[bool] = reactive(False)
    is_refreshing: reactive[bool] = reactive(False)
    
    # Content tracking for smart refresh
    _content_hashes: dict = {}
    _refresh_timer: Optional[Timer] = None
    _content_timer: Optional[Timer] = None
    
    # Thread pool for background operations
    _executor: Optional[ThreadPoolExecutor] = None

    def __init__(
        self,
        filter_ai: Optional[AIType] = None,
        refresh_interval: int = 60,
        *args,
        **kwargs
    ):
        """Initialize the Kanban App.
        
        Args:
            filter_ai: Optional AI type filter
            refresh_interval: Panel list refresh interval in seconds
        """
        super().__init__(*args, **kwargs)
        self.filter_ai = filter_ai
        self.refresh_interval = refresh_interval
        self._pane_list_version = 0

    def compose(self) -> ComposeResult:
        """Compose the UI layout."""
        # Search box (hidden by default, docked to bottom via CSS)
        yield SearchBox(id="search-container")
        
        # Main content area
        with Horizontal(id="main-content"):
            # Left panel - List
            with Vertical(id="panel-list-container"):
                yield Static("🔍 AI Panels", id="panel-list-title")
                yield PanelList(id="panel-table")
            
            # Right panel - Preview
            with Vertical(id="preview-container"):
                yield Static("Select a panel to preview", id="preview-title")
                yield PreviewPanel(id="preview-content")
        
        # Bottom status bar
        yield StatusBar(id="status-bar")

    async def on_mount(self) -> None:
        """Initialize on mount."""
        self.title = "Tmux AI Kanban"
        
        # Initialize thread pool
        self._executor = ThreadPoolExecutor(max_workers=2)
        
        # Initial scan (async)
        await self.action_refresh()
        
        # Setup timers for smart refresh
        # Check panel list every refresh_interval seconds
        self._refresh_timer = self.set_interval(
            self.refresh_interval,
            self._check_panel_list_update,
        )
        
        # Check content changes every 2 seconds
        self._content_timer = self.set_interval(2, self._check_content_update)
        
        # Focus the panel list after a short delay to ensure UI is ready
        self.set_timer(0.1, self._focus_panel_list)
    
    def _focus_panel_list(self) -> None:
        """Focus the panel list for keyboard navigation."""
        try:
            panel_list = self.query_one("#panel-table", PanelList)
            panel_list.focus()
            self.log("PanelList focused")
        except Exception as e:
            self.log(f"Failed to focus PanelList: {e}")
    
    def watch_focused(self, focused) -> None:
        """Watch for focus changes and log them."""
        if focused:
            self.log(f"Focus changed to: {focused.__class__.__name__} ({focused.id})")
        else:
            self.log("Focus cleared")

    async def _check_panel_list_update(self) -> None:
        """Check if panel list has changed (new/removed panes)."""
        # Skip if already refreshing
        if self.is_refreshing:
            return
        
        # Run in background thread
        loop = asyncio.get_event_loop()
        try:
            new_panels = await loop.run_in_executor(self._executor, self._fetch_panels_sync)
            
            # Compare by pane_id
            old_ids = {p.pane_id for p in self.panels}
            new_ids = {p.pane_id for p in new_panels}
            
            if old_ids != new_ids:
                self.log("Panel list changed, refreshing...")
                self.panels = new_panels
                self._apply_filter()
        except Exception:
            pass  # Silently fail for background check

    def _check_content_update(self) -> None:
        """Check if selected pane content has changed."""
        if not self.filtered_panels or self.selected_index >= len(self.filtered_panels):
            return
        
        panel = self.filtered_panels[self.selected_index]
        content = capture_pane(panel.pane_id, start_line=-50)
        
        # Calculate content hash
        content_hash = hashlib.md5(content.encode()).hexdigest()
        
        # Check if changed
        old_hash = self._content_hashes.get(panel.pane_id)
        if old_hash != content_hash:
            self._content_hashes[panel.pane_id] = content_hash
            # Update preview
            preview = self.query_one("#preview-content", PreviewPanel)
            preview.update_content(content, panel)

    def _fetch_panels(self) -> List[AIPanel]:
        """Fetch AI panels from tmux."""
        self.is_refreshing = True
        try:
            panels = scan_ai_panels()
            if self.filter_ai:
                panels = filter_panels(panels, self.filter_ai)
            return panels
        finally:
            self.is_refreshing = False

    def _apply_filter(self) -> None:
        """Apply search filter to panels."""
        if not self.search_query:
            self.filtered_panels = self.panels.copy()
        else:
            query = self.search_query.lower()
            self.filtered_panels = [
                p for p in self.panels
                if (query in p.session.lower() or
                    query in p.window.lower() or
                    query in p.working_dir.lower() or
                    (p.git_info and p.git_info.branch and query in p.git_info.branch.lower()))
            ]
        
        # Update UI
        self._update_panel_list()
        self._update_status_bar()
        
        # Reset selection if out of bounds
        if self.selected_index >= len(self.filtered_panels):
            self.selected_index = max(0, len(self.filtered_panels) - 1)
        
        # Update preview for selected panel
        self._update_preview()
        
        # Re-focus panel list to ensure keyboard navigation works
        panel_list = self.query_one("#panel-table", PanelList)
        if not self.is_searching:
            panel_list.focus()

    def _update_panel_list(self) -> None:
        """Update the panel list widget."""
        panel_list = self.query_one("#panel-table", PanelList)
        panel_list.update_panels(self.filtered_panels, self.selected_index)

    def _update_preview(self) -> None:
        """Update the preview widget."""
        if not self.filtered_panels or self.selected_index >= len(self.filtered_panels):
            preview = self.query_one("#preview-content", PreviewPanel)
            preview.clear()
            title = self.query_one("#preview-title", Static)
            title.update("No panels available")
            return
        
        panel = self.filtered_panels[self.selected_index]
        
        # Update title
        title = self.query_one("#preview-title", Static)
        git_info = ""
        if panel.git_info and panel.git_info.branch:
            git_info = f" [{panel.git_info.branch}]"
        title_text = f"📁 {panel.full_id}{git_info} - {panel.working_dir}"
        title.update(title_text[:self.size.width - 10])
        
        # Update preview content
        content = capture_pane(panel.pane_id, start_line=-50)
        preview = self.query_one("#preview-content", PreviewPanel)
        preview.update_content(content, panel)
        
        # Cache hash for smart refresh
        self._content_hashes[panel.pane_id] = hashlib.md5(content.encode()).hexdigest()

    def _update_status_bar(self) -> None:
        """Update the status bar."""
        status_bar = self.query_one("#status-bar", StatusBar)
        status_bar.update_status(
            total=len(self.panels),
            filtered=len(self.filtered_panels),
            selected=self.selected_index,
            refreshing=self.is_refreshing,
            searching=self.is_searching,
        )

    # Action handlers
    def action_quit(self) -> None:
        """Quit the application."""
        self.exit()

    async def action_refresh(self) -> None:
        """Manually refresh panels (async)."""
        self.is_refreshing = True
        self._update_status_bar()
        
        # Run refresh in background thread to avoid UI blocking
        loop = asyncio.get_event_loop()
        try:
            panels = await loop.run_in_executor(self._executor, self._fetch_panels_sync)
            self.panels = panels
            self._apply_filter()
            self.notify(f"Refreshed {len(panels)} panels", severity="information")
        except Exception as e:
            self.notify(f"Refresh failed: {e}", "error")
        finally:
            self.is_refreshing = False
            self._update_status_bar()
    
    def _fetch_panels_sync(self) -> List[AIPanel]:
        """Synchronous wrapper for fetching and sorting panels."""
        panels = scan_ai_panels()
        if self.filter_ai:
            panels = filter_panels(panels, self.filter_ai)
        
        # Sort by last_activity (descending) - most recent first
        # Panels with last_activity=0 (no activity) go to the end
        panels.sort(key=lambda p: p.last_activity, reverse=True)
        
        return panels

    def action_search(self) -> None:
        """Show search box."""
        self.is_searching = True
        search_box = self.query_one("#search-container", SearchBox)
        search_box.show()
        self._update_status_bar()

    def action_cancel_search(self) -> None:
        """Cancel search and restore full list."""
        if self.is_searching:
            self.is_searching = False
            self.search_query = ""
            search_box = self.query_one("#search-container", SearchBox)
            search_box.hide()
            self._apply_filter()

    def action_attach_popup(self) -> None:
        """Attach to selected panel using popup window with link-window approach.
        
        This creates a temporary session, links the target window to it,
        and opens a popup attached to the temporary session.
        This ensures the popup stays open until user exits.
        """
        if not self.filtered_panels or self.selected_index >= len(self.filtered_panels):
            self.notify("No panel selected", severity="error")
            return
        
        panel = self.filtered_panels[self.selected_index]
        
        # Generate unique temporary session name
        import uuid
        temp_session = f"tak-popup-{uuid.uuid4().hex[:8]}"
        
        try:
            # Step 1: Create a temporary detached session
            subprocess.run(
                ["tmux", "new-session", "-d", "-s", temp_session, "-c", panel.working_dir],
                check=True,
                capture_output=True,
            )
            
            # Step 2: Link the target window to our temporary session
            # Format: session:window.pane -> we need session:window
            window_target = f"{panel.session}:{panel.window}"
            link_result = subprocess.run(
                ["tmux", "link-window", "-s", window_target, "-t", temp_session, "-k"],
                check=False,
                capture_output=True,
            )
            
            # Step 3: Select the linked window (it should be window 0 now)
            subprocess.run(
                ["tmux", "select-window", "-t", f"{temp_session}:0"],
                check=False,
                capture_output=True,
            )
            
            # Step 4: Focus to the specific pane and zoom it
            # This makes the popup show only the target pane (full screen)
            subprocess.run(
                ["tmux", "select-pane", "-t", panel.pane_id],
                check=False,
                capture_output=True,
            )
            subprocess.run(
                ["tmux", "resize-pane", "-Z", "-t", panel.pane_id],
                check=False,
                capture_output=True,
            )
            
            # Step 5: Build popup command
            cmd = ["tmux", "popup", "-E", "-w", "90%", "-h", "85%"]
            
            # Add title if supported (tmux 3.3+)
            supports_title = False
            try:
                version_result = subprocess.run(
                    ["tmux", "-V"],
                    capture_output=True,
                    text=True,
                    check=False,
                    timeout=1,
                )
                version_str = version_result.stdout.strip()
                version_match = re.search(r'(\d+)\.(\d+)', version_str)
                if version_match:
                    major = int(version_match.group(1))
                    minor = int(version_match.group(2))
                    supports_title = major > 3 or (major == 3 and minor >= 3)
            except Exception:
                supports_title = False
            
            if supports_title:
                cmd.extend(["-T", f"{panel.ai_type.value} @ {panel.full_id}"])
            
            # Step 5: Add the attach command
            # When user exits (Ctrl+B D or Ctrl+D), the popup closes
            cmd.extend(["tmux", "attach-session", "-t", temp_session])
            
            # Step 6: Run popup (blocks until user exits)
            subprocess.run(cmd, check=False)
            
            # Step 7: Cleanup - kill the temporary session
            subprocess.run(
                ["tmux", "kill-session", "-t", temp_session],
                check=False,
                capture_output=True,
            )
            
            self.notify(f"Returned from {panel.full_id}")
            
        except subprocess.CalledProcessError as e:
            # Cleanup on error
            subprocess.run(
                ["tmux", "kill-session", "-t", temp_session],
                check=False,
                capture_output=True,
            )
            self.notify(f"Failed to open popup: {e}", severity="error")
        except Exception as e:
            # Cleanup on error
            subprocess.run(
                ["tmux", "kill-session", "-t", temp_session],
                check=False,
                capture_output=True,
            )
            self.notify(f"Failed to open popup: {e}", severity="error")

    def action_attach_session(self) -> None:
        """Attach to selected panel by switching tmux client.
        
        Kanban will be detached to a background session.
        User can return with 'tmux attach -t tak-kanban' or restart tak tui.
        """
        if not self.filtered_panels or self.selected_index >= len(self.filtered_panels):
            self.notify("No panel selected", severity="error")
            return
        
        panel = self.filtered_panels[self.selected_index]
        
        # Create a detached session for kanban if not exists
        kanban_session = "tak-kanban"
        try:
            # Check if session exists
            result = subprocess.run(
                ["tmux", "has-session", "-t", kanban_session],
                capture_output=True,
                check=False,
            )
            if result.returncode != 0:
                # Create detached session
                subprocess.run(
                    ["tmux", "new-session", "-d", "-s", kanban_session],
                    check=False,
                )
        except Exception:
            pass
        
        # Move current pane to kanban session (this preserves kanban)
        # Then switch to target
        try:
            # Get current pane info
            result = subprocess.run(
                ["tmux", "display-message", "-p", "#{session_name}:#{window_index}.#{pane_index}"],
                capture_output=True,
                text=True,
                check=False,
            )
            current_target = result.stdout.strip()
            
            # Join current pane to kanban session
            subprocess.run(
                ["tmux", "move-pane", "-s", current_target, "-t", f"{kanban_session}:"],
                check=False,
            )
            
            # Switch to target panel
            subprocess.run(
                ["tmux", "switch-client", "-t", panel.full_id],
                check=False,
            )
            
            self.exit(0)
        except Exception as e:
            self.notify(f"Failed to switch: {e}", severity="error")

    def _jump_to_index(self, index: int) -> None:
        """Jump to panel by index (0-based)."""
        if 0 <= index < len(self.filtered_panels):
            self.selected_index = index
            # Also update DataTable cursor
            panel_list = self.query_one("#panel-table", PanelList)
            panel_list.move_cursor(row=index)
            self._update_preview()
            self._update_status_bar()
    
    # Individual jump actions for keys 1-9
    def action_jump_to_1(self) -> None: self._jump_to_index(0)
    def action_jump_to_2(self) -> None: self._jump_to_index(1)
    def action_jump_to_3(self) -> None: self._jump_to_index(2)
    def action_jump_to_4(self) -> None: self._jump_to_index(3)
    def action_jump_to_5(self) -> None: self._jump_to_index(4)
    def action_jump_to_6(self) -> None: self._jump_to_index(5)
    def action_jump_to_7(self) -> None: self._jump_to_index(6)
    def action_jump_to_8(self) -> None: self._jump_to_index(7)
    def action_jump_to_9(self) -> None: self._jump_to_index(8)

    def action_move_down(self) -> None:
        """Move selection down."""
        if self.selected_index < len(self.filtered_panels) - 1:
            self.selected_index += 1
            # Update DataTable cursor directly
            panel_list = self.query_one("#panel-table", PanelList)
            panel_list.move_cursor(row=self.selected_index)
            self._update_preview()
            self._update_status_bar()

    def action_move_up(self) -> None:
        """Move selection up."""
        if self.selected_index > 0:
            self.selected_index -= 1
            # Update DataTable cursor directly
            panel_list = self.query_one("#panel-table", PanelList)
            panel_list.move_cursor(row=self.selected_index)
            self._update_preview()
            self._update_status_bar()

    def action_move_top(self) -> None:
        """Move selection to top."""
        self.selected_index = 0
        self._update_panel_list()
        self._update_preview()
        self._update_status_bar()

    def action_move_bottom(self) -> None:
        """Move selection to bottom."""
        self.selected_index = max(0, len(self.filtered_panels) - 1)
        self._update_panel_list()
        self._update_preview()
        self._update_status_bar()

    def action_next_page(self) -> None:
        """Move to next page."""
        page_size = 10  # Approximate visible rows
        self.selected_index = min(
            len(self.filtered_panels) - 1,
            self.selected_index + page_size
        )
        self._update_panel_list()
        self._update_preview()
        self._update_status_bar()

    def action_prev_page(self) -> None:
        """Move to previous page."""
        page_size = 10
        self.selected_index = max(0, self.selected_index - page_size)
        self._update_panel_list()
        self._update_preview()
        self._update_status_bar()

    # Event handlers
    def on_search_box_changed(self, event: SearchBox.Changed) -> None:
        """Handle search query change."""
        self.search_query = event.value
        self.selected_index = 0
        self._apply_filter()

    def on_search_box_submitted(self) -> None:
        """Handle search submission."""
        self.is_searching = False
        search_box = self.query_one("#search-container", SearchBox)
        search_box.hide()
        self._update_status_bar()

    def on_panel_list_selected(self, event: PanelList.Selected) -> None:
        """Handle panel selection from list."""
        self.selected_index = event.index
        self._update_preview()
        self._update_status_bar()

    def on_data_table_row_highlighted(self, event) -> None:
        """Handle DataTable row highlight (cursor movement)."""
        # Only respond to events from our panel table
        if event.control.id == "panel-table":
            self.selected_index = event.cursor_row
            self._update_preview()
            self._update_status_bar()
