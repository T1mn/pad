"""Search box widget for filtering panels."""

from textual.widgets import Input, Static, Label
from textual.containers import Horizontal, Container
from textual.message import Message


class SearchBox(Container):
    """Widget for entering search queries."""

    DEFAULT_CSS = """
    SearchBox {
        width: 100%;
        height: auto;
        background: $surface-darken-2;
        border: solid $primary;
        padding: 0 1;
        display: none;
        layer: overlay;
        dock: bottom;
    }
    
    SearchBox.visible {
        display: block;
    }
    
    SearchBox > Horizontal {
        width: 100%;
        height: auto;
    }
    
    SearchBox Label {
        color: $text;
        text-style: bold;
        padding: 0 1;
        content-align: center middle;
        width: auto;
    }
    
    SearchBox Input {
        width: 1fr;
        border: none;
        background: $surface;
    }
    """

    class Changed(Message):
        """Message sent when search query changes."""
        
        def __init__(self, value: str) -> None:
            self.value = value
            super().__init__()

    class Submitted(Message):
        """Message sent when search is submitted."""
        pass

    def __init__(self, *args, **kwargs):
        """Initialize the search box."""
        super().__init__(*args, **kwargs)
        self._input: Input | None = None

    def compose(self):
        """Compose the search box."""
        with Horizontal():
            yield Label("Filter: ")
            yield Input(placeholder="session, window, directory, or branch...", id="search-input")

    def on_mount(self) -> None:
        """Store reference to input on mount."""
        self._input = self.query_one("#search-input", Input)

    def show(self) -> None:
        """Show the search box and focus it."""
        self.add_class("visible")
        if self._input:
            self._input.focus()
            self._input.value = ""

    def hide(self) -> None:
        """Hide the search box."""
        self.remove_class("visible")
        if self._input:
            self._input.value = ""

    def on_input_changed(self, event: Input.Changed) -> None:
        """Handle input changes."""
        self.post_message(self.Changed(event.value))

    def on_input_submitted(self, event: Input.Submitted) -> None:
        """Handle input submission."""
        self.post_message(self.Submitted())
