"""Rust-accelerated detector with fallback to Python."""

from typing import List, Optional

# Try to import Rust core
try:
    from tmux_kanban_core import scan_code_panels as _rust_scan_panels
    USE_RUST = True
except ImportError:
    USE_RUST = False

from .models import CodeType, CodePanel, GitInfo, GitStatus
from .detector import scan_code_panels as _python_scan_panels, filter_panels


def scan_code_panels() -> List[CodePanel]:
    """Scan code panels using Rust core if available, otherwise Python.
    
    Returns:
        List of CodePanel objects sorted by last_activity
    """
    if USE_RUST:
        try:
            return _rust_scan_panels_impl()
        except Exception as e:
            # Fallback to Python on error
            print(f"Rust scan failed ({e}), falling back to Python")
            return _python_scan_panels()
    else:
        return _python_scan_panels()


def _rust_scan_panels_impl() -> List[CodePanel]:
    """Convert Rust panels to Python CodePanel objects."""
    rust_panels = _rust_scan_panels()
    
    panels = []
    for rp in rust_panels:
        # Convert string code_type to enum
        code_type_str = rp.get("code_type", "unknown")
        try:
            code_type = CodeType(code_type_str)
        except ValueError:
            code_type = CodeType.UNKNOWN
        
        panel = CodePanel(
            session=rp.get("session", ""),
            window=rp.get("window", ""),
            pane=rp.get("pane", ""),
            pane_id=rp.get("pane_id", ""),
            code_type=code_type,
            working_dir=rp.get("working_dir", ""),
            is_active=rp.get("is_active", False),
            last_activity=rp.get("last_activity", 0.0),
        )
        panels.append(panel)
    
    return panels


def is_rust_available() -> bool:
    """Check if Rust core is available."""
    return USE_RUST
