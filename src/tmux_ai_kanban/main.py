"""Main entry point for tmux-ai-kanban."""

from typing import Optional, List

import typer
from rich.console import Console
from rich import print as rprint

from . import __version__
from .models import AIType
from .detector_rust import scan_ai_panels, filter_panels
from .tmux_client import is_tmux_running, switch_client
from .ui.table import display_panels

# Import TUI conditionally
try:
    from .tui import KanbanApp
    TUI_AVAILABLE = True
except ImportError:
    TUI_AVAILABLE = False

app = typer.Typer(
    name="ai-kanban",
    help="Tmux AI Panel Kanban - Manage claude, codex, kimi panels",
    add_completion=False,
)
console = Console()


def version_callback(value: bool):
    """Print version and exit."""
    if value:
        rprint(f"tmux-ai-kanban version {__version__}")
        raise typer.Exit()


@app.callback()
def main(
    version: Optional[bool] = typer.Option(
        None, "--version", "-v",
        callback=version_callback,
        is_eager=True,
        help="Show version and exit",
    ),
):
    """Tmux AI Kanban - Manage your AI coding assistants in tmux."""
    pass


@app.command()
def list(
    filter_ai: Optional[str] = typer.Option(
        None, "--filter", "-f",
        help="Filter by AI type (claude, codex, kimi)"
    ),
    watch: bool = typer.Option(
        False, "--watch", "-w",
        help="Watch mode (auto-refresh every 2 seconds)"
    ),
):
    """List all AI panels in a table."""
    if not is_tmux_running():
        rprint("[red]Error:[/red] Tmux server is not running.")
        raise typer.Exit(1)
    
    # Parse filter
    ai_type = None
    if filter_ai:
        try:
            ai_type = AIType(filter_ai.lower())
        except ValueError:
            rprint(f"[red]Error:[/red] Unknown AI type '{filter_ai}'. Choose from: claude, codex, kimi")
            raise typer.Exit(1)
    
    if watch:
        import time
        try:
            while True:
                console.clear()
                panels = scan_ai_panels()
                panels = filter_panels(panels, ai_type)
                display_panels(panels, console)
                rprint("\n[dim]Press Ctrl+C to exit watch mode[/dim]")
                time.sleep(2)
        except KeyboardInterrupt:
            console.clear()
            raise typer.Exit()
    else:
        panels = scan_ai_panels()
        panels = filter_panels(panels, ai_type)
        display_panels(panels, console)


@app.command()
def jump(
    session: str = typer.Argument(..., help="Session name"),
    window: str = typer.Argument(..., help="Window name"),
    pane: str = typer.Argument(..., help="Pane index"),
):
    """Jump to a specific tmux pane."""
    target = f"{session}:{window}.{pane}"
    if switch_client(target):
        rprint(f"[green]Switched to[/green] {target}")
    else:
        rprint(f"[red]Failed to switch to[/red] {target}")
        raise typer.Exit(1)


@app.command()
def summary():
    """Show summary of AI panels."""
    if not is_tmux_running():
        rprint("[red]Error:[/red] Tmux server is not running.")
        raise typer.Exit(1)
    
    panels = scan_ai_panels()
    
    if not panels:
        rprint("[yellow]No AI panels found.[/yellow]")
        raise typer.Exit()
    
    # Count by AI type
    counts = {}
    active_count = 0
    sessions = set()
    
    for p in panels:
        counts[p.ai_type] = counts.get(p.ai_type, 0) + 1
        if p.is_active:
            active_count += 1
        sessions.add(p.session)
    
    # Print summary
    rprint("[bold magenta]Tmux AI Kanban Summary[/bold magenta]")
    rprint(f"Total panels: {len(panels)}")
    rprint(f"Active: {active_count}")
    rprint(f"Sessions: {len(sessions)}")
    rprint()
    
    for ai_type, count in sorted(counts.items(), key=lambda x: x[1], reverse=True):
        emoji = {"claude": "🟣", "codex": "🔵", "kimi": "🟢", "unknown": "⚪"}.get(ai_type.value, "⚪")
        rprint(f"  {emoji} [cyan]{ai_type.value}:[/cyan] {count}")
    
    rprint()
    rprint("[dim]Run 'ai-kanban list' for detailed view[/dim]")


# Alias commands
@app.command(name="ls")
def ls_command(
    filter_ai: Optional[str] = typer.Option(None, "--filter", "-f"),
):
    """Alias for 'list' command."""
    list(filter_ai=filter_ai, watch=False)


@app.command(name="s")
def s_command():
    """Alias for 'summary' command."""
    summary()


@app.command()
def show(
    pane_id: str = typer.Argument(..., help="Pane ID (e.g., %155)"),
    lines: int = typer.Option(50, "--lines", "-n", help="Number of lines to show"),
):
    """Show detailed content of a specific pane."""
    if not is_tmux_running():
        rprint("[red]Error:[/red] Tmux server is not running.")
        raise typer.Exit(1)
    
    from .tmux_client import capture_pane, get_pane_history_size
    
    history_size = get_pane_history_size(pane_id)
    content = capture_pane(pane_id, start_line=-lines)
    
    rprint(f"[bold magenta]Pane {pane_id} Content[/bold magenta]")
    rprint(f"[dim]History size: {history_size} lines | Showing last {lines} lines[/dim]")
    rprint()
    
    # Print with syntax highlighting for common patterns
    for line in content.split("\n"):
        # Highlight prompts
        if line.strip().startswith(("$", "#", "❯", ">")):
            rprint(f"[green]{line}[/green]")
        # Highlight AI responses
        elif line.strip().startswith(("●", "•", "💫")):
            rprint(f"[cyan]{line}[/cyan]")
        # Highlight timing info
        elif "Brewed" in line or "Worked" in line or "✻" in line:
            rprint(f"[yellow]{line}[/yellow]")
        else:
            rprint(line)


@app.command()
def conversations(
    filter_ai: Optional[str] = typer.Option(None, "--filter", "-f"),
):
    """Show conversation turns for all AI panels."""
    if not is_tmux_running():
        rprint("[red]Error:[/red] Tmux server is not running.")
        raise typer.Exit(1)
    
    from .detector import scan_ai_panels, filter_panels
    from .models import AIType
    
    # Parse filter
    ai_type = None
    if filter_ai:
        try:
            ai_type = AIType(filter_ai.lower())
        except ValueError:
            rprint(f"[red]Error:[/red] Unknown AI type '{filter_ai}'")
            raise typer.Exit(1)
    
    panels = scan_ai_panels()
    panels = filter_panels(panels, ai_type)
    
    if not panels:
        rprint("[yellow]No AI panels found.[/yellow]")
        raise typer.Exit()
    
    for panel in panels:
        emoji = {"claude": "🟣", "codex": "🔵", "kimi": "🟢"}.get(panel.ai_type.value, "⚪")
        rprint(f"\n[bold]{emoji} {panel.ai_type.value}[/bold] at [cyan]{panel.full_id}[/cyan]")
        rprint(f"[dim]{panel.working_dir}[/dim]")
        
        if panel.conversation_turns:
            for turn in panel.conversation_turns:
                role = turn.get('role', 'unknown')
                content = turn.get('content', '')
                
                if role == 'user':
                    rprint(f"  [green]User:[/green] {content[:80]}{'...' if len(content) > 80 else ''}")
                else:
                    rprint(f"  [cyan]AI:[/cyan] {content[:80]}{'...' if len(content) > 80 else ''}")
        else:
            rprint("  [dim]No conversation detected[/dim]")
        rprint()


@app.command()
def tui(
    filter_ai: Optional[str] = typer.Option(
        None, "--filter", "-f",
        help="Filter by AI type (claude, codex, kimi)"
    ),
    refresh_interval: int = typer.Option(
        60, "--refresh-interval", "-r",
        help="Panel list refresh interval in seconds"
    ),
):
    """Launch interactive TUI interface."""
    if not TUI_AVAILABLE:
        rprint("[red]Error:[/red] TUI dependencies not installed.")
        rprint("Install with: pip install tmux-ai-kanban[tui]")
        raise typer.Exit(1)
    
    if not is_tmux_running():
        rprint("[red]Error:[/red] Tmux server is not running.")
        raise typer.Exit(1)
    
    # Parse filter
    ai_type = None
    if filter_ai:
        try:
            ai_type = AIType(filter_ai.lower())
        except ValueError:
            rprint(f"[red]Error:[/red] Unknown AI type '{filter_ai}'. Choose from: claude, codex, kimi")
            raise typer.Exit(1)
    
    # Run TUI
    app = KanbanApp(filter_ai=ai_type, refresh_interval=refresh_interval)
    app.run()


@app.command(name="tk")
def tk_command(
    filter_ai: Optional[str] = typer.Option(None, "--filter", "-f"),
    refresh_interval: int = typer.Option(60, "--refresh-interval", "-r"),
):
    """Alias for 'tui' command."""
    tui(filter_ai=filter_ai, refresh_interval=refresh_interval)


if __name__ == "__main__":
    app()
