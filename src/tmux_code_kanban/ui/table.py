"""Table UI for displaying AI panels."""

from typing import List, Optional

from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.text import Text
from rich import box

from ..models import AIPanel, AIType, GitStatus


def get_ai_emoji(ai_type: AIType) -> str:
    """Get emoji for AI type."""
    return {
        AIType.CLAUDE: "🟣",
        AIType.CODEX: "🔵",
        AIType.KIMI: "🟢",
        AIType.UNKNOWN: "⚪",
    }.get(ai_type, "⚪")


def get_status_indicator(is_active: bool) -> str:
    """Get status indicator."""
    return "●" if is_active else "○"


def format_git_info(panel: AIPanel) -> Text:
    """Format git info for display."""
    if not panel.git_info or panel.git_info.status == GitStatus.NO_GIT:
        return Text("—", style="dim")
    
    git = panel.git_info
    parts = []
    
    if git.branch:
        branch_color = "red" if git.status == GitStatus.DIRTY else "green"
        parts.append((f"{git.branch}", branch_color))
    
    if git.short_commit:
        parts.append((f"@{git.short_commit}", "dim"))
    
    if git.changed_files > 0:
        parts.append((f" +{git.changed_files}", "yellow"))
    
    text = Text()
    for content, style in parts:
        text.append(content, style=style)
    return text


def format_working_dir(path: str, max_len: int = 35) -> Text:
    """Format working directory with truncation."""
    # Replace home with ~
    import os
    home = os.path.expanduser("~")
    if path.startswith(home):
        path = "~" + path[len(home):]
    
    # Truncate if too long
    if len(path) > max_len:
        path = "..." + path[-(max_len-3):]
    
    return Text(path, style="cyan")


def create_table(panels: List[AIPanel], title: str = "Tmux AI Panels") -> Table:
    """Create a rich table from panels."""
    table = Table(
        title=title,
        box=box.ROUNDED,
        show_header=True,
        header_style="bold magenta",
    )
    
    # Add columns
    table.add_column("AI", style="cyan", width=4)
    table.add_column("Status", width=6, justify="center")
    table.add_column("Session", style="blue")
    table.add_column("Window", style="blue")
    table.add_column("Working Dir", style="cyan", max_width=25)
    table.add_column("Git", max_width=20)
    table.add_column("Latest Activity", style="dim", max_width=45)
    
    # Add rows
    for panel in panels:
        ai_emoji = get_ai_emoji(panel.ai_type)
        status = get_status_indicator(panel.is_active)
        status_style = "green" if panel.is_active else "dim"
        
        git_text = format_git_info(panel)
        dir_text = format_working_dir(panel.working_dir, max_len=25)
        
        # Use conversation summary if available
        if panel.conversation_turns:
            content = panel.conversation_summary
        else:
            content = panel.last_content or "—"
        
        if len(content) > 45:
            content = content[:42] + "..."
        
        table.add_row(
            ai_emoji,
            Text(status, style=status_style),
            panel.session,
            panel.window,
            dir_text,
            git_text,
            content,
        )
    
    return table


def display_panels(panels: List[AIPanel], console: Optional[Console] = None) -> None:
    """Display panels in a table."""
    if console is None:
        console = Console()
    
    if not panels:
        console.print(Panel(
            "[yellow]No AI panels found.[/yellow]\n"
            "Run [cyan]claude[/cyan], [cyan]codex[/cyan], or [cyan]kimi[/cyan] in a tmux pane.",
            title="Tmux AI Kanban",
            border_style="red"
        ))
        return
    
    # Create summary
    summary = Text()
    summary.append(f"Found {len(panels)} AI panel(s): ", style="bold")
    
    counts = {}
    for p in panels:
        counts[p.ai_type] = counts.get(p.ai_type, 0) + 1
    
    parts = []
    for ai_type, count in counts.items():
        emoji = get_ai_emoji(ai_type)
        parts.append(f"{emoji} {ai_type.value}({count})")
    summary.append(" | ".join(parts))
    
    console.print(summary)
    console.print()
    
    # Create and print table
    table = create_table(panels)
    console.print(table)
    
    # Print help
    console.print()
    console.print("[dim]Use --tui flag for interactive mode. Press 'q' to quit.[/dim]")



