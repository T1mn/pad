import { describe, expect, it } from "vitest";
import {
  MODEL_CATALOG_ACTION,
  availableModelIds,
  modelCatalogWithFallback,
  parseModelCatalog,
} from "./model-catalog";

describe("Pi model_catalog renderer contract", () => {
  it("已登录时读取具体模型、当前选择和显示名称", () => {
    const catalog = parseModelCatalog({
      models: [
        { provider: "openai-codex", id: "gpt-5.4", name: "GPT-5.4", reasoning: true, input: ["text", "image"] },
        { provider: "openai-codex", id: "gpt-5.5", name: "GPT-5.5", reasoning: true, input: ["text"] },
      ],
      selected_provider: "openai-codex",
      selected_model: "gpt-5.4",
      source: "live",
      checked_at: 1_788_068_570_947,
    });

    expect(catalog.models.map((model) => model.id)).toEqual(["gpt-5.4", "gpt-5.5"]);
    expect(catalog.models[0]).toMatchObject({ provider: "openai-codex", name: "GPT-5.4", reasoning: true });
    expect(catalog.selectedProvider).toBe("openai-codex");
    expect(catalog.selectedModel).toBe("gpt-5.4");
    expect(catalog.source).toBe("live");
    expect(catalog.checkedAt).toBe(1_788_068_570_947);
    expect(availableModelIds(catalog)).toEqual(["auto", "gpt-5.4", "gpt-5.5"]);
  });

  it("识别 Pi ModelRuntime 的 pi_model_runtime 来源", () => {
    const catalog = parseModelCatalog({
      source: "pi_model_runtime",
      available_models: [{ provider: "openai-codex", id: "gpt-5.4", name: "GPT-5.4" }],
    });

    expect(catalog.source).toBe("live");
    expect(catalog.models).toHaveLength(1);
  });

  it("未登录或返回空目录时只保留 auto，不伪造模型", () => {
    const catalog = parseModelCatalog({
      models: [],
      selected_provider: null,
      selected_model: null,
      source: "live",
    });

    expect(catalog.models).toEqual([]);
    expect(catalog.providers).toEqual([]);
    expect(availableModelIds(catalog)).toEqual(["auto"]);
  });

  it("兼容多个 provider 分组，仅显示已认证 provider 的模型", () => {
    const catalog = parseModelCatalog({
      providers: [
        {
          id: "openai-codex",
          name: "OpenAI Codex",
          authenticated: true,
          models: [{ id: "gpt-5.4", name: "GPT-5.4" }],
        },
        {
          id: "anthropic",
          name: "Anthropic",
          authenticated: true,
          models: [{ id: "claude-sonnet", name: "Claude Sonnet", input: ["text", "image"] }],
        },
        {
          id: "google",
          authenticated: false,
          models: [{ id: "gemini-pro", name: "Gemini Pro" }],
        },
      ],
      selected_provider: "anthropic",
    });

    expect(catalog.providers.map((provider) => provider.id)).toEqual(["openai-codex", "anthropic", "google"]);
    expect(catalog.models.map((model) => `${model.provider}/${model.id}`)).toEqual([
      "openai-codex/gpt-5.4",
      "anthropic/claude-sonnet",
    ]);
    expect(catalog.selectedProvider).toBe("anthropic");
  });

  it("刷新失败时回退到最后可用缓存并明确标记 stale", () => {
    const catalog = modelCatalogWithFallback(
      { source: "live", error: "provider unavailable", models: [] },
      {
        source: "cache",
        checked_at: 1_700_000_000,
        models: [{ provider: "openai-codex", id: "gpt-5.4", name: "GPT-5.4" }],
      },
      { selectedProvider: "openai-codex", selectedModel: "gpt-5.4" },
    );

    expect(catalog.source).toBe("cache");
    expect(catalog.stale).toBe(true);
    expect(catalog.error).toBe("provider unavailable");
    expect(catalog.models[0]?.id).toBe("gpt-5.4");
    expect(catalog.selectedModel).toBe("gpt-5.4");
  });

  it("支持 catalog 包装、available_models 和模型对象 map 的滚动升级格式", () => {
    const catalog = parseModelCatalog({
      catalog: {
        available_models: {
          "gpt-5.4": { provider: "openai-codex", display_name: "GPT-5.4" },
          "gpt-5.5": { provider: "openai-codex", label: "GPT-5.5" },
        },
        source: "cached",
        stale: true,
      },
    });

    expect(catalog.models).toEqual([
      expect.objectContaining({ id: "gpt-5.4", name: "GPT-5.4", provider: "openai-codex" }),
      expect.objectContaining({ id: "gpt-5.5", name: "GPT-5.5", provider: "openai-codex" }),
    ]);
    expect(catalog.source).toBe("cache");
    expect(catalog.stale).toBe(true);
  });

  it("固定 model_catalog action 名，避免 renderer 与 sidecar 请求字符串漂移", () => {
    expect(MODEL_CATALOG_ACTION).toBe("model_catalog");
  });
});
