"""Main entry point for tmux-code-kanban."""

from typing import Optional

import typer
from rich.console import Console
from rich import print as rprint

from . import __version__
from .models import CodeType
from .tmux_client import is_tmux_running

# Import TUI conditionally
try:
    from .tui import KanbanApp
    TUI_AVAILABLE = True
except ImportError:
    TUI_AVAILABLE = False

app = typer.Typer(
    name="code-kanban",
    help="Tmux Code Panel Kanban - Manage your code panels in tmux",
    add_completion=False,
)
console = Console()


def version_callback(value: bool):
    """Print version and exit."""
    if value:
        rprint(f"tmux-code-kanban version {__version__}")
        raise typer.Exit()


@app.callback(invoke_without_command=True)
def main(
    ctx: typer.Context,
    version: Optional[bool] = typer.Option(
        None, "--version", "-v",
        callback=version_callback,
        is_eager=True,
        help="Show version and exit",
    ),
    filter_code: Optional[str] = typer.Option(
        None, "--filter", "-f",
        help="Filter by code type (claude, codex, kimi)"
    ),
    refresh_interval: int = typer.Option(
        60, "--refresh-interval", "-r",
        help="Panel list refresh interval in seconds"
    ),
):
    """Tmux Code Kanban - Manage your code panels in tmux.
    
    Launch interactive TUI when run without subcommands.
    """
    # If subcommand is invoked, don't run TUI
    if ctx.invoked_subcommand is not None:
        return
    
    if not TUI_AVAILABLE:
        rprint("[red]Error:[/red] TUI dependencies not installed.")
        rprint("Install with: pip install tmux-code-kanban")
        raise typer.Exit(1)
    
    if not is_tmux_running():
        rprint("[red]Error:[/red] Tmux server is not running.")
        raise typer.Exit(1)
    
    # Parse filter
    code_type = None
    if filter_code:
        try:
            code_type = CodeType(filter_code.lower())
        except ValueError:
            rprint(f"[red]Error:[/red] Unknown code type '{filter_code}'. Choose from: claude, codex, kimi")
            raise typer.Exit(1)
    
    # Run TUI
    tui_app = KanbanApp(filter_code=code_type, refresh_interval=refresh_interval)
    tui_app.run()


@app.command(name="tk")
def tk_command(
    filter_code: Optional[str] = typer.Option(None, "--filter", "-f"),
    refresh_interval: int = typer.Option(60, "--refresh-interval", "-r"),
):
    """Launch TUI (alias)."""
    if not TUI_AVAILABLE:
        rprint("[red]Error:[/red] TUI dependencies not installed.")
        raise typer.Exit(1)
    
    if not is_tmux_running():
        rprint("[red]Error:[/red] Tmux server is not running.")
        raise typer.Exit(1)
    
    # Parse filter
    code_type = None
    if filter_code:
        try:
            code_type = CodeType(filter_code.lower())
        except ValueError:
            rprint(f"[red]Error:[/red] Unknown code type '{filter_code}'")
            raise typer.Exit(1)
    
    # Run TUI
    tui_app = KanbanApp(filter_code=code_type, refresh_interval=refresh_interval)
    tui_app.run()


if __name__ == "__main__":
    app()
