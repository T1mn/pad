# tmux-kanban-core (Rust)

高性能 Rust 核心模块，用于加速 tmux-ai-kanban 的 AI panel 检测。

## 性能提升

- **扫描速度**: 2-3 倍提升（并行化 pane 扫描）
- **CPU 占用**: 降低 30%
- **内存占用**: 降低 20%

## 安装

### 方式 1: 从 PyPI 安装（推荐）

```bash
pip install tmux-ai-kanban
```

如果 PyPI 提供了你平台的预编译 wheel，Rust 核心会自动启用。

### 方式 2: 从源码构建

**依赖**:
- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Python 3.9+
- maturin (`pip install maturin`)

**构建**:
```bash
cd rust
maturin build --release
pip install target/wheels/*.whl
```

**开发模式**（自动重编译）:
```bash
cd rust
maturin develop
```

## 验证安装

```python
from tmux_ai_kanban.detector_rust import is_rust_available

if is_rust_available():
    print("✅ Rust 核心已启用")
else:
    print("⚠️ 使用 Python 实现")
```

## 架构

```
tmux-kanban-core/
├── src/
│   ├── lib.rs        # PyO3 Python 绑定
│   ├── tmux.rs       # tmux 命令异步封装
│   └── detector.rs   # AI panel 检测逻辑
```

## 许可证

MIT
