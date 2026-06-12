# DeepSeek(cc) 配置指南

## ✅ 完全隔离方案

DeepSeek(cc) 使用独立的配置目录 `~/.pad/deepseek-config/`，**完全不影响原生 Claude Code**。

## 快速配置

在 `~/.pad/config.toml` 添加：

```toml
[[agents]]
name = "deepseek(cc)"
cmd = "~/.pad/deepseek-cc"
active_provider = 0

[[agents.providers]]
label = "DeepSeek"
base_url = "https://api.deepseek.com"
api_key = "<your-deepseek-key>"
disable_thinking = false
```

## 工作原理

1. **独立配置目录**：通过 `CLAUDE_CONFIG_DIR=~/.pad/deepseek-config` 隔离
2. **自动生成脚本**：pad 生成 `~/.pad/deepseek-cc` 启动脚本
3. **零污染**：完全不修改 `~/.claude/settings.json`

## 使用方式

在 pad TUI 中选择 **deepseek(cc)**，然后 c 选择目录，自动启动 DeepSeek。

## 配置位置

- 原生 Claude：`~/.claude/settings.json`
- DeepSeek(cc)：`~/.pad/deepseek-config/settings.json`
- 启动脚本：`~/.pad/deepseek-cc`

## 验证

```bash
# 查看生成的脚本
cat ~/.pad/deepseek-cc

# 查看独立配置
cat ~/.pad/deepseek-config/settings.json
```
