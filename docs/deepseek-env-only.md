# DeepSeek(cc) 纯环境变量方案

## ✅ 测试结果

DeepSeek API 端点 **已确认支持** 通过环境变量直接调用 Claude Code！

## 使用方法（超轻量）

### 方案 1：直接 export + claude
```bash
export ANTHROPIC_BASE_URL="https://api.deepseek.com/anthropic"
export ANTHROPIC_AUTH_TOKEN="<your-deepseek-key>"
export ANTHROPIC_MODEL="deepseek-v4-flash"

claude  # 直接使用 DeepSeek
```

### 方案 2：启动脚本（推荐）
```bash
# 使用封装好的脚本
./scripts/deepseek-cc.sh

# 或创建别名
alias deepseek-cc='ANTHROPIC_BASE_URL=https://api.deepseek.com/anthropic ANTHROPIC_AUTH_TOKEN=<your-deepseek-key> ANTHROPIC_MODEL=deepseek-v4-flash claude'
```

## 优势

- ✅ **零配置文件修改** - 完全不碰 `~/.claude/settings.json`
- ✅ **零备份需求** - 不需要备份恢复机制
- ✅ **即时切换** - 开新终端就是原生 Claude
- ✅ **更轻量** - 只需要环境变量

## 身份注入

⚠️ **注意**：纯 export 方式无法注入 "你是 DeepSeek" 身份（SessionStart hook 需要 settings.json）。

如果需要身份注入，仍然需要使用 pad relay 方式修改配置文件。

## 两种方案对比

| 特性 | 纯 export | pad relay |
|------|-----------|-----------|
| 修改配置文件 | ❌ 不修改 | ✅ 修改并备份 |
| 身份注入 | ❌ 无法注入 | ✅ SessionStart hook |
| 切换速度 | ⚡ 即时 | 需要 pad 切换 |
| 使用场景 | 临时测试、脚本调用 | 完整体验、需要身份 |

## 推荐

- **临时使用 / 脚本调用**：使用纯 export
- **完整 DeepSeek 体验**：使用 pad relay（支持身份注入）
