#!/bin/bash
# DeepSeek(cc) 纯环境变量启动 - 不修改任何配置文件

if [ -z "${DEEPSEEK_API_KEY:-}" ]; then
  echo "DEEPSEEK_API_KEY is required" >&2
  exit 1
fi

export ANTHROPIC_BASE_URL="${DEEPSEEK_BASE_URL:-https://api.deepseek.com/anthropic}"
export ANTHROPIC_AUTH_TOKEN="$DEEPSEEK_API_KEY"
export ANTHROPIC_MODEL="${DEEPSEEK_MODEL:-deepseek-v4-flash}"

exec claude "$@"
