#!/usr/bin/env python3
"""
快速验证 DeepSeek relay 配置是否正确注入
"""
import json
import sys
from pathlib import Path

def verify_deepseek_config():
    settings_path = Path.home() / ".claude" / "settings.json"

    if not settings_path.exists():
        print("⚠️  ~/.claude/settings.json 不存在")
        return False

    with open(settings_path) as f:
        config = json.load(f)

    # 检查环境变量
    env = config.get("env", {})
    base_url = env.get("ANTHROPIC_BASE_URL", "")
    auth_token = env.get("ANTHROPIC_AUTH_TOKEN", "")

    print("📝 环境变量检查:")
    print(f"  ANTHROPIC_BASE_URL: {base_url[:50]}..." if len(base_url) > 50 else f"  ANTHROPIC_BASE_URL: {base_url}")
    print(f"  ANTHROPIC_AUTH_TOKEN: {'已设置 ✅' if auth_token else '未设置 ❌'}")

    # 检查 SessionStart hook
    hooks = config.get("hooks", {})
    session_start_hooks = hooks.get("SessionStart", [])

    deepseek_identity_found = False
    for hook_group in session_start_hooks:
        for hook in hook_group.get("hooks", []):
            output = hook.get("hookSpecificOutput", {})
            context = output.get("additionalContext", "")
            if "DeepSeek" in context:
                deepseek_identity_found = True
                print(f"\n🎭 身份注入检查:")
                print(f"  ✅ 找到 DeepSeek 身份: {context}")
                break

    if not deepseek_identity_found:
        print(f"\n🎭 身份注入检查:")
        print(f"  ❌ 未找到 DeepSeek 身份注入")
        return False

    return True

if __name__ == "__main__":
    print("🔍 验证 DeepSeek(cc) 配置\n")
    success = verify_deepseek_config()
    print(f"\n{'✅ 配置正确' if success else '❌ 配置有误'}")
    sys.exit(0 if success else 1)
