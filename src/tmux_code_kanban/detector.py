"""Code panel detector - identify panes running code tools."""

import re
import time
from typing import List, Optional

from .models import CodeType, CodePanel, GitInfo, GitStatus
from .tmux_client import get_all_child_processes, list_panes, capture_pane
from .git_info import get_git_info


# Code tool process patterns
CODE_PATTERNS = {
    CodeType.CLAUDE: ["claude"],
    CodeType.CODEX: ["codex"],
    CodeType.KIMI: ["Kimi", "kimi", "Kimi Code"],
}


def detect_code_type(processes: List[str]) -> Optional[CodeType]:
    """Detect code type from process list.
    
    Args:
        processes: List of process command names

    Returns:
        Detected code type or None
    """
    process_str = " ".join(processes).lower()
    
    # Check each code type
    for code_type, patterns in CODE_PATTERNS.items():
        for pattern in patterns:
            if pattern.lower() in process_str:
                return code_type
    
    return None


def get_code_version(code_type: CodeType, processes: List[str]) -> Optional[str]:
    """Try to extract code tool version.
    
    For now, return simple identifier. Could be extended to parse --version.
    """
    # Could add version detection logic here
    # For example, parsing output of `claude --version`
    return None


def extract_content_summary(content: str, max_lines: int = 5) -> str:
    """Extract summary from pane content.
    
    Args:
        content: Full pane content
        max_lines: Maximum lines to include in summary

    Returns:
        Truncated content with last non-empty lines
    """
    lines = content.split("\n")
    
    # Remove empty lines from end
    while lines and not lines[-1].strip():
        lines.pop()
    
    # Get last n non-empty lines
    non_empty = [l for l in lines if l.strip()]
    summary_lines = non_empty[-max_lines:] if len(non_empty) > max_lines else non_empty
    
    # Join and truncate
    summary = " ".join(summary_lines)
    if len(summary) > 100:
        summary = summary[:97] + "..."
    
    return summary


def extract_conversation_turns(content: str, max_turns: int = 3) -> List[dict]:
    """Extract conversation turns from pane content.
    
    Identifies patterns like:
    - User input: lines starting with "$", ">", "❯"
    - AI response: lines starting with "●", "•", "💫"
    
    Returns:
        List of dicts with 'role' and 'content'
    """
    lines = content.split("\n")
    turns = []
    current_role = None
    current_content = []
    
    # Patterns for user/AI messages
    user_patterns = [r'^\$', r'^>', r'^❯', r'^[\$#]\s']
    ai_patterns = [r'^●', r'^•', r'^💫', r'^🤖', r'^🟣', r'^🔵', r'^🟢']
    
    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        
        # Check if this is a new turn
        is_user = any(re.search(p, stripped) for p in user_patterns)
        is_ai = any(re.search(p, stripped) for p in ai_patterns)
        
        if is_user:
            # Save previous turn
            if current_role and current_content:
                turns.append({
                    'role': current_role,
                    'content': '\n'.join(current_content)
                })
            current_role = 'user'
            current_content = [stripped.lstrip('$>❯ ').strip()]
        elif is_ai:
            if current_role and current_content:
                turns.append({
                    'role': current_role,
                    'content': '\n'.join(current_content)
                })
            current_role = 'assistant'
            current_content = [stripped.lstrip('●•💫').strip()]
        else:
            # Continue current turn
            if current_role:
                current_content.append(stripped)
    
    # Save last turn
    if current_role and current_content:
        turns.append({
            'role': current_role,
            'content': '\n'.join(current_content)
        })
    
    # Return last N turns
    return turns[-max_turns:] if len(turns) > max_turns else turns


def is_panel_active(content: str) -> bool:
    """Check if panel appears to be actively working.
    
    Heuristics:
    - Contains loading indicators
    - Recent activity markers
    """
    active_markers = [
        "thinking",
        "loading",
        "processing",
        "generating",
        "running",
        "executing",
        "⋯",
        "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",  # spinner chars
    ]
    
    content_lower = content.lower()
    return any(marker in content_lower for marker in active_markers)


def scan_code_panels() -> List[CodePanel]:
    """Scan all tmux panes and identify code panels.
    
    Returns:
        List of CodePanel objects
    """
    code_panels = []
    
    try:
        panes = list_panes()
    except Exception:
        return code_panels
    
    for pane_info in panes:
        pid = pane_info.get("pane_pid", 0)
        if not pid:
            continue
        
        # Get all child processes
        child_processes = get_all_child_processes(pid)
        current_cmd = pane_info.get("pane_current_command", "")
        all_processes = [current_cmd] + child_processes
        
        # Detect code type
        code_type = detect_code_type(all_processes)
        if not code_type:
            continue
        
        # Get pane content (capture more history)
        pane_id = pane_info["pane_id"]
        from .tmux_client import get_pane_history_size
        history_size = get_pane_history_size(pane_id)
        
        # Capture last 100 lines for better context
        content = capture_pane(pane_id, start_line=-100)
        
        # Parse conversation turns
        conversation_turns = extract_conversation_turns(content)
        
        # Get git info
        working_dir = pane_info.get("pane_current_path", "")
        git_info = None
        if working_dir:
            git_info = get_git_info(working_dir)
        
        # Check if panel is active and update last_activity
        is_active = is_panel_active(content)
        last_activity = time.time() if is_active else 0.0
        
        # Create panel object
        panel = CodePanel(
            session=pane_info["session_name"],
            window=pane_info["window_name"],
            pane=pane_info["pane_index"],
            pane_id=pane_id,
            code_type=code_type,
            code_version=get_code_version(code_type, all_processes),
            working_dir=working_dir,
            git_info=git_info,
            last_content=extract_content_summary(content),
            history_lines=history_size,
            conversation_turns=conversation_turns,
            is_active=is_active,
            last_activity=last_activity,
            process_count=len(all_processes),
        )
        
        code_panels.append(panel)
    
    return code_panels


def filter_panels(panels: List[CodePanel], code_type: Optional[CodeType] = None) -> List[CodePanel]:
    """Filter panels by criteria.
    
    Args:
        panels: List of panels to filter
        code_type: Optional code type to filter by

    Returns:
        Filtered list
    """
    if code_type is None:
        return panels
    return [p for p in panels if p.code_type == code_type]


def group_panels_by_session(panels: List[CodePanel]) -> dict:
    """Group panels by tmux session."""
    groups = {}
    for panel in panels:
        session = panel.session
        if session not in groups:
            groups[session] = []
        groups[session].append(panel)
    return groups
