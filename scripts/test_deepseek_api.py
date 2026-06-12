#!/usr/bin/env python3
"""
测试 DeepSeek API 连接（Anthropic 协议）
"""
import json
import os
import subprocess
import sys

API_KEY = os.environ.get("DEEPSEEK_API_KEY", "")
BASE_URL = "https://api.deepseek.com/anthropic/v1/messages"

def test_connection():
    print("🔍 测试 DeepSeek Anthropic API 连接...\n")

    if not API_KEY:
        print("❌ 请设置环境变量：export DEEPSEEK_API_KEY=<your-api-key>")
        return False

    cmd = [
        "curl", "-s", BASE_URL,
        "-H", "Content-Type: application/json",
        "-H", f"x-api-key: {API_KEY}",
        "-H", "anthropic-version: 2023-06-01",
        "-d", json.dumps({
            "model": "deepseek-v4-flash",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": "你好，你是谁？"}
            ]
        })
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        response = json.loads(result.stdout)

        if "error" in response:
            print(f"❌ API 错误: {response['error']}")
            return False

        if "content" in response and response["content"]:
            text = next((c["text"] for c in response["content"] if c.get("type") == "text"), None)
            thinking = next((c["thinking"] for c in response["content"] if c.get("type") == "thinking"), None)

            print("✅ API 连接成功！\n")
            print(f"模型: {response['model']}")
            print(f"回复: {text}")
            if thinking:
                print(f"\n思考过程: {thinking[:100]}...")
            print(f"\nTokens: {response['usage']['input_tokens']} in / {response['usage']['output_tokens']} out")
            return True

        print(f"❌ 响应格式异常: {response}")
        return False

    except subprocess.TimeoutExpired:
        print("❌ 请求超时")
        return False
    except json.JSONDecodeError as e:
        print(f"❌ JSON 解析失败: {e}")
        print(result.stdout)
        return False
    except Exception as e:
        print(f"❌ 未知错误: {e}")
        return False

if __name__ == "__main__":
    success = test_connection()
    sys.exit(0 if success else 1)
