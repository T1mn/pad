"""Data models for tmux AI kanban."""

from dataclasses import dataclass, field
from enum import Enum
from typing import List, Optional


class AIType(str, Enum):
    """AI tool types."""
    CLAUDE = "claude"
    CODEX = "codex"
    KIMI = "kimi"
    UNKNOWN = "unknown"


class GitStatus(str, Enum):
    """Git repository status."""
    CLEAN = "clean"
    DIRTY = "dirty"
    NO_GIT = "no_git"


@dataclass
class GitInfo:
    """Git repository information."""
    branch: Optional[str] = None
    commit: Optional[str] = None
    commit_msg: Optional[str] = None
    status: GitStatus = GitStatus.NO_GIT
    changed_files: int = 0
    
    @property
    def short_commit(self) -> Optional[str]:
        """Get short commit hash."""
        return self.commit[:8] if self.commit else None


@dataclass
class AIPanel:
    """AI panel information."""
    # Tmux identifiers
    session: str
    window: str
    pane: str
    pane_id: str
    
    # AI info
    ai_type: AIType
    ai_version: Optional[str] = None
    
    # Working directory
    working_dir: str = ""
    
    # Git info
    git_info: Optional[GitInfo] = None
    
    # Content
    last_content: str = ""  # Summary of latest content
    history_lines: int = 0  # Total history available
    
    # Conversation turns (if parsed)
    conversation_turns: List[dict] = field(default_factory=list)
    
    # Activity status
    is_active: bool = False
    last_activity: float = 0.0  # Timestamp of last activity
    
    # Process info
    process_count: int = 0  # Number of child processes
    
    @property
    def full_id(self) -> str:
        """Get full pane identifier."""
        return f"{self.session}:{self.window}.{self.pane}"
    
    @property
    def tmux_target(self) -> str:
        """Get tmux target for commands."""
        return self.pane_id
    
    @property
    def conversation_summary(self) -> str:
        """Get summary of recent conversation."""
        if not self.conversation_turns:
            return self.last_content
        
        parts = []
        for turn in self.conversation_turns[-2:]:  # Last 2 turns
            role = "U" if turn.get('role') == 'user' else "A"
            content = turn.get('content', '')[:50]
            if len(turn.get('content', '')) > 50:
                content += "..."
            parts.append(f"{role}: {content}")
        return " | ".join(parts)
    
    def __str__(self) -> str:
        return f"{self.ai_type.value}@{self.full_id}"
