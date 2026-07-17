# DeepSeek Relay

DeepSeek(cc) 支持通过套壳 Claude Code 实现，复用 Claude Code 的所有功能。

## 代码结构

- `../deepseek.rs`：更新 DeepSeek settings，并生成 launcher 内容。
- `launcher.rs`：以 owner-only 权限通过同目录临时文件原子替换 launcher。

## 工作原理

1. **配置写入**：将 endpoint、key 与 model 写入 `~/.pad/deepseek-config/settings.json`
2. **独立启动**：`~/.pad/deepseek-cc` 导出独立配置目录和 relay 环境变量后执行 Claude Code
3. **协议转换**：需要 DeepSeek API → Anthropic 协议的转换层（如 one-api、new-api）

## 配置要求

在 pad 主题配置中添加 DeepSeek provider：

```yaml
agents:
  - name: deepseek
    cmd: claude  # 复用 claude 命令
    providers:
      - label: DeepSeek
        base_url: https://your-api-gateway.com  # 需要支持 Anthropic 协议
        api_key: <your-api-key>
        disable_thinking: false
        models: []
    active_provider: 0
    default_model: "deepseek-chat"  # 或其他模型
```

## 注意事项

- `base_url` 必须是支持 Anthropic Messages API 协议的 endpoint
- 推荐使用 one-api 或 new-api 等网关做协议转换
- DeepSeek 原生 API 不兼容 Anthropic 协议，必须通过转换层
- DeepSeek 使用独立配置目录和 launcher，不改写原生 Claude 配置
