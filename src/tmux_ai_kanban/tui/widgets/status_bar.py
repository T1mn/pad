"""Status bar widget for displaying app status and key bindings."""

from textual.widgets import Static


class StatusBar(Static):
    """Widget for displaying status information and key bindings."""

    DEFAULT_CSS = """
    StatusBar {
        height: 1;
        width: 100%;
        background: $primary-darken-3;
        color: $text;
        content-align: left middle;
        padding: 0 1;
        dock: bottom;
    }
    """

    def __init__(self, *args, **kwargs):
        """Initialize the status bar."""
        super().__init__("Loading...", *args, **kwargs)
        self._total = 0
        self._filtered = 0
        self._selected = 0
        self._refreshing = False
        self._searching = False

    def update_status(
        self,
        total: int,
        filtered: int,
        selected: int,
        refreshing: bool = False,
        searching: bool = False,
    ) -> None:
        """Update the status display."""
        self._total = total
        self._filtered = filtered
        self._selected = selected
        self._refreshing = refreshing
        self._searching = searching
        
        self._update_content()

    def _update_content(self) -> None:
        """Update the status bar content."""
        if self._searching:
            status = (
                "[bold blue]SEARCH[/bold blue] | "
                "Type to filter | "
                "[bold]Enter[/bold]: confirm | "
                "[bold]Esc[/bold]: cancel"
            )
        else:
            parts = []
            parts.append("[bold green]↑/k[/bold green]↑ [bold green]↓/j[/bold green]↓")
            parts.append("[bold green]1-9[/bold green] jmp")
            parts.append("[bold green]/[/bold green] find")
            parts.append("[bold green]⏎[/bold green] popup")
            parts.append("[bold green]s[/bold green] sess")
            parts.append("[bold green]r[/bold green] refresh")
            parts.append("[bold green]q[/bold green] quit")
            parts.append("|")
            
            if self._filtered != self._total:
                parts.append(f"[bold yellow]{self._selected + 1}/{self._filtered}[/bold yellow] (of {self._total})")
            else:
                parts.append(f"[bold yellow]{self._selected + 1}/{self._total}[/bold yellow]")
            
            if self._refreshing:
                parts.append("[bold blink green]⟳[/bold blink green]")
            
            status = "  ".join(parts)
        
        self.update(status)
