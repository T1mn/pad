"""Preview panel widget for displaying tmux pane content."""

from textual.widgets import Static
from textual.scroll_view import ScrollView

from ...models import CodePanel


class PreviewPanel(Static):
    """Widget for displaying the content preview of a tmux pane."""

    DEFAULT_CSS = """
    PreviewPanel {
        height: 100%;
        width: 100%;
        background: $surface;
        color: $text;
        padding: 0 1;
        overflow: auto scroll;
    }
    """

    def __init__(self, *args, **kwargs):
        """Initialize the preview panel."""
        super().__init__("", *args, **kwargs)
        self._current_content = ""
        self._current_panel: CodePanel | None = None

    def update_content(self, content: str, panel: CodePanel) -> None:
        """Update the preview content.
        
        Args:
            content: Raw tmux pane content
            panel: The code panel being previewed
        """
        self._current_content = content
        self._current_panel = panel
        
        # Process and display content
        display_text = self._process_content(content, panel)
        self.update(display_text)
        
        # Scroll to bottom to show latest content
        self._scroll_to_bottom()

    def clear(self) -> None:
        """Clear the preview."""
        self._current_content = ""
        self._current_panel = None
        self.update("[dim]Select a panel to preview its content[/dim]")
    
    def _scroll_to_bottom(self) -> None:
        """Scroll to the bottom of the preview."""
        # Use call_after_refresh to ensure content is rendered
        def do_scroll():
            # Get the actual scrollable height and scroll to bottom
            try:
                # Method 1: Use max_scroll_y
                if hasattr(self, 'max_scroll_y') and self.max_scroll_y is not None:
                    self.scroll_y = self.max_scroll_y
                    return
            except Exception:
                pass
            
            # Method 2: Use scroll_end action
            try:
                self.action_scroll_end()
                return
            except Exception:
                pass
            
            # Method 3: Try scroll_to with large value
            try:
                self.scroll_to(y=999999, animate=False)
            except Exception:
                pass
        
        # Schedule scroll after render with multiple attempts
        import asyncio
        async def delayed_scroll():
            # Try multiple times with increasing delays
            for delay in [0.05, 0.1, 0.2]:
                await asyncio.sleep(delay)
                self.call_after_refresh(do_scroll)
        
        asyncio.create_task(delayed_scroll())

    def _process_content(self, content: str, panel: CodePanel) -> str:
        """Process raw tmux content for display.
        
        Preserves formatting but handles edge cases.
        
        Args:
            content: Raw tmux content
            panel: Code panel info
            
        Returns:
            Processed content for display
        """
        if not content:
            return "[dim]No content available[/dim]"
        
        lines = content.split("\n")
        
        # Remove trailing empty lines
        while lines and not lines[-1].strip():
            lines.pop()
        
        # Process lines for display
        processed = []
        for line in lines[-50:]:  # Show last 50 lines
            processed.append(self._format_line(line))
        
        return "\n".join(processed)

    def _format_line(self, line: str) -> str:
        """Format a single line with appropriate styling.
        
        Args:
            line: Raw line content
            
        Returns:
            Formatted line with rich markup
        """
        stripped = line.strip()
        
        # Handle empty lines
        if not stripped:
            return ""
        
        # User prompts (command lines)
        user_markers = ["$", "#", "❯", ">", "%"]
        for marker in user_markers:
            if stripped.startswith(marker):
                # User input line
                content = stripped[len(marker):].strip()
                return f"[bold green]{marker}[/bold green] [green]{self._escape(content)}[/green]"
        
        # AI response markers
        ai_markers = ["●", "•", "💫", "🤖", "🟣", "🔵", "🟢", "⚡"]
        for marker in ai_markers:
            if stripped.startswith(marker):
                content = stripped[len(marker):].strip()
                return f"[bold cyan]{marker}[/bold cyan] [cyan]{self._escape(content)}[/cyan]"
        
        # Timestamps and metadata
        if any(kw in stripped for kw in ["Brewed", "Worked", "Duration", "✻", "◇"]):
            return f"[yellow]{self._escape(line)}[/yellow]"
        
        # Error messages
        if any(kw in stripped.lower() for kw in ["error", "failed", "exception", "traceback"]):
            return f"[red]{self._escape(line)}[/red]"
        
        # Success messages
        if any(kw in stripped.lower() for kw in ["success", "done", "completed", "✓", "✔"]):
            return f"[green]{self._escape(line)}[/green]"
        
        # Loading/spinner indicators
        spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        if any(c in stripped for c in spinner_chars) or "thinking" in stripped.lower():
            return f"[bold yellow]{self._escape(line)}[/bold yellow]"
        
        # Default: plain text with escaping
        return self._escape(line)

    def _escape(self, text: str) -> str:
        """Escape rich markup characters in text.
        
        Args:
            text: Raw text
            
        Returns:
            Escaped text safe for rich display
        """
        # Replace [ with [[ to escape rich markup
        return text.replace("[", "[[")
