## DeepSeek Relay 实现总结

### 已完成的工作

1. **创建 relay 模块** (`src/relay/deepseek.rs`)
   - 复用 claude settings path（共享 `~/.claude/settings.json`）
   - 独立备份路径 (`~/.pad/deepseek-settings.pre-pad.json`)
   - 注入 SessionStart hook with `additionalContext: "你是 DeepSeek。"`

2. **集成到主模块**
   - 在 `src/relay.rs` 添加 deepseek 模块
   - 在 `apply_relay_configs()` 添加 deepseek 分支
   - 在 `src/theme/config.rs` 默认 agents 列表添加 deepseek

3. **文档**
   - `src/relay/deepseek/index.md` - 技术说明
   - `docs/deepseek-setup.md` - 用户指南

### 核心机制

```rust
// 1. 写入环境变量到 ~/.claude/settings.json
env_obj.insert("ANTHROPIC_BASE_URL", deepseek_endpoint);
env_obj.insert("ANTHROPIC_AUTH_TOKEN", deepseek_key);

// 2. 注入身份 hook
hooks.SessionStart.push({
  "hooks": [{
    "type": "inline",
    "hookSpecificOutput": {
      "hookEventName": "SessionStart",
      "additionalContext": "你是 DeepSeek。"
    }
  }]
});
```

### 隔离保证

- ✅ 独立备份，不影响原生 claude 配置
- ✅ 复用 claude 命令，只改环境变量和 hook
- ✅ 切换回 claude agent 时自动恢复原配置

### 用户配置示例

```yaml
agents:
  - name: deepseek
    cmd: claude
    providers:
      - label: DeepSeek
        base_url: https://api.deepseek.com  # 需要 Anthropic 协议转换
        api_key: <your-api-key>
        disable_thinking: false
    active_provider: 0
    default_model: deepseek-chat
```

### 依赖要求

用户需要：
1. DeepSeek API key
2. 支持 Anthropic Messages API 协议的 endpoint（通过 one-api/new-api 等转换）

### 编译验证

```bash
✅ cargo build - 编译通过
✅ cargo test relay::tests - 38个测试全部通过
```
