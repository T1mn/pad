/**
 * Renderer-side model catalog contract.
 *
 * The Rust bridge owns Pi's ModelRuntime and credentials.  The renderer only
 * receives this public projection:
 *
 *   { models: [{ provider, id, name, ... }], selected_provider,
 *     selected_model, source, stale }
 *
 * During rolling upgrades a bridge may wrap the projection in `catalog`, use
 * `available_models`, or return provider groups.  Keep those compatibility
 * rules in one pure function so the UI never reads Pi's private JSON files.
 */

export const MODEL_CATALOG_ACTION = "model_catalog" as const;

export type ModelCatalogSource = "live" | "cache" | "fallback" | "unknown";

export interface ModelCatalogModel {
  provider: string;
  id: string;
  name: string;
  description?: string;
  reasoning?: boolean;
  input: string[];
}

export interface ModelCatalogProvider {
  id: string;
  name: string;
  authenticated: boolean;
  models: ModelCatalogModel[];
}

export interface ModelCatalog {
  models: ModelCatalogModel[];
  providers: ModelCatalogProvider[];
  selectedProvider: string | null;
  selectedModel: string | null;
  source: ModelCatalogSource;
  stale: boolean;
  checkedAt: number | null;
  error?: string;
}

/**
 * Parse a renderer-safe model catalog without trusting arbitrary fields from
 * the sidecar.  Unknown/malformed entries are ignored; an empty catalog is a
 * valid result for a profile that is not authenticated.
 */
export function parseModelCatalog(value: unknown, defaults: Partial<Pick<ModelCatalog, "selectedProvider" | "selectedModel">> = {}): ModelCatalog {
  const root = record(value) ?? {};
  const payload = record(root.catalog) ?? root;
  const selectedProvider = cleanText(payload.selected_provider ?? payload.selectedProvider)
    ?? defaults.selectedProvider
    ?? null;
  const selectedModel = cleanText(payload.selected_model ?? payload.selectedModel)
    ?? defaults.selectedModel
    ?? null;

  const providers = parseProviders(payload.providers, selectedProvider);
  const directModels = parseModelCollection(payload.models ?? payload.available_models, selectedProvider);
  const models = deduplicateModels(
    directModels.length > 0
      ? directModels
      : providers.flatMap((provider) => provider.authenticated ? provider.models : []),
  );
  const checkedAt = finiteTimestamp(payload.checked_at ?? payload.checkedAt ?? payload.refreshed_at ?? payload.refreshedAt);
  const source = normalizeSource(payload.source ?? payload.origin ?? payload.cache_status);
  const stale = payload.stale === true || payload.is_stale === true || source === "cache";
  const error = cleanText(payload.error ?? payload.error_message);

  return {
    models,
    providers,
    selectedProvider,
    selectedModel,
    source,
    stale,
    checkedAt,
    ...(error ? { error } : {}),
  };
}

/**
 * Prefer a usable live response. If refresh failed or returned no models,
 * preserve the last usable cache and mark it stale. If neither exists, the
 * caller can still render the built-in `auto` option.
 */
export function modelCatalogWithFallback(current: unknown, cached: unknown, defaults: Partial<Pick<ModelCatalog, "selectedProvider" | "selectedModel">> = {}): ModelCatalog {
  const live = parseModelCatalog(current, defaults);
  if (live.models.length > 0) return live;

  const cachedCatalog = parseModelCatalog(cached, {
    selectedProvider: live.selectedProvider ?? defaults.selectedProvider,
    selectedModel: live.selectedModel ?? defaults.selectedModel,
  });
  if (cachedCatalog.models.length > 0) {
    return {
      ...cachedCatalog,
      source: "cache",
      stale: true,
      ...(live.error ? { error: live.error } : {}),
    };
  }

  return {
    ...live,
    source: "fallback",
    stale: false,
  };
}

/** The visible picker always keeps Pi's automatic selection as its first item. */
export function availableModelIds(catalog: ModelCatalog): string[] {
  return ["auto", ...catalog.models.map((model) => model.id).filter((id, index, values) => values.indexOf(id) === index)];
}

function parseProviders(value: unknown, selectedProvider: string | null): ModelCatalogProvider[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry) => {
    const item = record(entry);
    if (!item) return [];
    const id = cleanText(item.id ?? item.provider ?? item.name) ?? selectedProvider ?? "unknown";
    const name = cleanText(item.name ?? item.label) ?? id;
    const authenticated = item.authenticated !== false;
    const models = parseModelCollection(item.models ?? item.available_models, id);
    return [{ id, name, authenticated, models }];
  });
}

function parseModelCollection(value: unknown, fallbackProvider: string | null): ModelCatalogModel[] {
  if (Array.isArray(value)) return value.flatMap((entry) => parseModel(entry, fallbackProvider));
  const values = record(value);
  if (!values) return [];

  // Older Pi stores may use an object keyed by model ID.
  return Object.entries(values).flatMap(([key, entry]) => {
    const parsed = parseModel(entry, fallbackProvider, key);
    return parsed.length > 0 ? parsed : parseModel(key, fallbackProvider);
  });
}

function parseModel(value: unknown, fallbackProvider: string | null, key?: string): ModelCatalogModel[] {
  if (typeof value === "string") {
    const id = cleanText(value);
    return id ? [{ provider: fallbackProvider ?? "unknown", id, name: id, input: [] }] : [];
  }
  const item = record(value);
  if (!item) return [];
  const id = cleanText(item.id ?? item.model ?? item.model_id ?? item.value ?? key);
  if (!id) return [];
  const provider = cleanText(item.provider) ?? fallbackProvider ?? "unknown";
  const name = cleanText(item.name ?? item.label ?? item.display_name) ?? id;
  const description = cleanText(item.description);
  const input = Array.isArray(item.input)
    ? item.input.filter((entry): entry is string => typeof entry === "string").map((entry) => entry.trim()).filter(Boolean)
    : [];
  const reasoning = typeof item.reasoning === "boolean" ? item.reasoning : undefined;
  return [{
    provider,
    id,
    name,
    ...(description ? { description } : {}),
    ...(reasoning === undefined ? {} : { reasoning }),
    input,
  }];
}

function deduplicateModels(models: ModelCatalogModel[]): ModelCatalogModel[] {
  const seen = new Set<string>();
  return models.filter((model) => {
    const key = `${model.provider}\u0000${model.id}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function normalizeSource(value: unknown): ModelCatalogSource {
  const source = cleanText(value)?.toLowerCase();
  if (source === "live" || source === "network" || source === "remote" || source === "pi_model_runtime") return "live";
  if (source === "cache" || source === "cached") return "cache";
  if (source === "fallback" || source === "default") return "fallback";
  return "unknown";
}

function finiteTimestamp(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : null;
}

function cleanText(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const text = value.replace(/[\u0000-\u001f\u007f]/g, "").trim();
  return text || null;
}

function record(value: unknown): Record<string, any> | null {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, any> : null;
}
