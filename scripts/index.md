# scripts

- `build_installer.sh`：将 `../install/` 组装为 `../install.sh`。
- `claude_hook_bridge.py`：把 Claude hook 事件和 PAD 原生 pane 标识转发到本地 sockets。
- `ci/pi_rpc_smoke.sh`：macOS 上启动真实 Pi 0.84.4，验证 RPC 握手、PAD 私有 agent root 与 `--session-dir`；默认不需要模型凭据。
- `ci/pi_rpc_prompt_smoke.mjs`：可选的真实 provider prompt smoke；由 `PAD_PI_SMOKE_PROMPT=1` 启用，需要 Pi 已配置凭据并会消耗一次模型调用。
- `deepseek-cc.sh` / `test_deepseek_api.py` / `verify_deepseek_config.py`：DeepSeek(cc) 启动与诊断辅助脚本。
- `ci/`：CI / smoke test 脚本。
