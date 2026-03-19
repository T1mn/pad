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
        overflow-y: auto;
        overflow-x: hidden;
        content-align-vertical: bottom;
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
        """Scroll to the bottom of the preview to show latest content."""
        import asyncio
        
        async def do_scroll():
            """Perform scroll with retries."""
            # Wait for content to be rendered
            await asyncio.sleep(0.05)
            
            # Try to scroll to bottom using multiple methods
            for _ in range(3):
                try:
                    # Get the scrollable region
                    if hasattr(self, 'scrollable_content_region'):
                        region = self.scrollable_content_region
                        if region:
                            # Scroll to show the bottom of the content
                            target_y = max(0, region.height - self.size.height)
                            self.scroll_to(y=target_y, animate=False)
                            return
                except Exception:
                    pass
                
                try:
                    # Alternative: use max_scroll_y
                    if hasattr(self, 'max_scroll_y'):
                        self.scroll_y = self.max_scroll_y
                        return
                except Exception:
                    pass
                
                try:
                    # Last resort: action_scroll_end
                    self.action_scroll_end()
                    return
                except Exception:
                    pass
                
                await asyncio.sleep(0.05)
        
        # Run scroll task
        asyncio.create_task(do_scroll())

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
