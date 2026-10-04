// Authentication state machine for the native workbench host.
//
// Owns one auth attempt per profile: synchronous reservation (so two attempts
// or an attempt and a logout can never race past provider validation), SDK
// login driving, native prompt/notice events, and attempt identity. Every
// AuthState carries its attemptId, and every publish is guarded by identity +
// aborted checks so a superseded attempt can never mutate current UI state.
import crypto from 'node:crypto';
import { AuthError, authMethodAllowed, classifyAuthError, safeAuthMessage } from './auth-policy.mjs';

const MAX_ATTEMPT_ID_LENGTH = 200;

/** Accept a client attempt id only when it is a sane, short string. */
export function normalizeAttemptId(value) {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  if (trimmed.length === 0 || trimmed.length > MAX_ATTEMPT_ID_LENGTH) return undefined;
  return trimmed;
}

/** Protocol AuthState for one attempt; attemptId is always present. */
export function buildAuthState(op) {
  const state = {
    profileId: op.profileId,
    provider: op.providerId,
    method: op.method,
    phase: op.phase,
    attemptId: op.attemptId,
  };
  if (op.message !== undefined) state.message = op.message;
  if (op.promptId !== undefined) state.promptId = op.promptId;
  if (op.promptKind !== undefined) state.promptKind = op.promptKind;
  if (op.placeholder !== undefined) state.placeholder = op.placeholder;
  if (op.options !== undefined) state.options = op.options;
  if (op.url !== undefined) state.url = op.url;
  if (op.userCode !== undefined) state.userCode = op.userCode;
  return state;
}

export class AuthManager {
  constructor({
    requireProfile, isClosed, profileBusy, agentDirFor, sdkClient,
    emit, diagnostic, makeError, onCredentialsChanged, recordAuthDiagnostic,
  }) {
    this.requireProfile = requireProfile;
    this.isClosed = isClosed;
    this.profileBusy = profileBusy;
    this.agentDirFor = agentDirFor;
    this.sdkClient = sdkClient;
    this.emit = emit;
    this.diagnostic = diagnostic;
    this.makeError = makeError;
    this.onCredentialsChanged = onCredentialsChanged;
    this.recordAuthDiagnostic = recordAuthDiagnostic;
    this.ops = new Map();
    this.states = new Map();
    this.logoutGuards = new Set();
  }

  /** True while an attempt or a sign-out reservation owns this profile. */
  isAuthBusy(profileId) {
    return this.ops.has(profileId) || this.logoutGuards.has(profileId);
  }

  /** Emit the current state of an active attempt; ignores stale/aborted ops. */
  publish(op) {
    if (this.isClosed()) return;
    if (op.controller.signal.aborted) return;
    if (this.ops.get(op.profileId) !== op) return;
    const state = buildAuthState(op);
    this.states.set(op.profileId, state);
    this.emit({ type: 'event', event: 'auth', data: state });
  }

  record(op, detail) {
    // Best-effort and never awaited by authentication. Do not pass the op itself:
    // it contains profile/attempt identifiers and potentially authorization URLs.
    try {
      Promise.resolve(this.recordAuthDiagnostic?.({
        providerId: op.providerId, method: op.method,
        phase: detail.phase, category: detail.category, outcome: detail.outcome,
        httpStatus: detail.httpStatus, errno: detail.errno,
      })).catch(() => {});
    } catch { /* diagnostics cannot replace the auth result */ }
  }

  observe(op, phase) {
    op.observedPhase = phase;
    this.record(op, { phase, category: 'stage', outcome: 'observed' });
  }

  async begin(profileId, provider, method, clientAttemptId) {
    const profile = this.requireProfile(profileId);
    if (this.isClosed()) throw this.makeError('The host is shutting down');
    if (this.ops.has(profile.id) || this.logoutGuards.has(profile.id)) {
      throw this.makeError('Authentication is already in progress for this profile');
    }
    if (this.profileBusy(profile.id)) {
      throw this.makeError('Cannot start authentication while this profile has an active task');
    }
    if (typeof provider !== 'string' || provider.trim().length === 0) throw this.makeError('provider is required');
    if (typeof method !== 'string' || method.trim().length === 0) throw this.makeError('method is required');
    const providerId = provider.trim();
    const methodId = method.trim();
    const op = {
      profileId: profile.id,
      providerId,
      method: methodId,
      attemptId: normalizeAttemptId(clientAttemptId) ?? crypto.randomUUID(),
      phase: 'running',
      controller: new AbortController(),
      pending: new Map(),
    };
    // Reserve synchronously, before any await, so a second attempt or a cancel
    // can never race past provider validation.
    this.ops.set(profile.id, op);
    this.publish(op);
    this.record(op, { phase: 'setup', category: 'attempt', outcome: 'started' });
    let agentDir;
    try {
      agentDir = this.agentDirFor(profile.id);
      const status = await this.sdkClient().providerStatus(agentDir, providerId);
      if (this.ops.get(profile.id) !== op) return buildAuthState(op);
      if (!status || !authMethodAllowed(providerId, methodId) || !status.authTypes.includes(methodId)) {
        throw new AuthError('unsupported');
      }
    } catch (error) {
      if (this.ops.get(profile.id) !== op) return buildAuthState(op);
      op.failureDiagnostic = classifyAuthError(error, { providerId: op.providerId, method: op.method });
      const message = op.failureDiagnostic.message;
      this.finish(op, 'failed', message);
      throw this.makeError(message);
    }
    if (this.isClosed() || op.controller.signal.aborted) {
      if (this.ops.get(profile.id) === op) this.finish(op, 'cancelled');
      return buildAuthState(op);
    }
    void this.runLogin(op, agentDir).catch((error) => {
      this.diagnostic(safeAuthMessage(error));
    });
    return buildAuthState(op);
  }

  async runLogin(op, agentDir) {
    if (this.isClosed() || op.controller.signal.aborted) {
      this.finish(op, 'cancelled');
      return;
    }
    const interaction = {
      signal: op.controller.signal,
      prompt: (prompt) => this.authPrompt(op, prompt),
      notify: (event) => this.authNotify(op, event),
    };
    let succeeded = false;
    let failure;
    let committed = false;
    try {
      await this.sdkClient().login({ agentDir, providerId: op.providerId, method: op.method, interaction });
      succeeded = true;
    } catch (error) {
      op.failureDiagnostic = classifyAuthError(error, { providerId: op.providerId, method: op.method });
      failure = op.failureDiagnostic.message;
      // An observed prompt/notice is not evidence of callback acceptance.
      if (op.failureDiagnostic.category === 'unknown' && op.observedPhase) {
        const labels = {
          authorization_notice_observed: 'authorization notice',
          manual_prompt_observed: 'manual callback prompt',
          token_exchange_observed: 'token-exchange progress notice',
        };
        failure += ` Last observed stage: ${labels[op.observedPhase]}; the failure cause is unknown.`;
      }
      committed = error instanceof AuthError && error.code === 'credential_sync';
    }
    if (this.ops.get(op.profileId) !== op) return;
    if (op.controller.signal.aborted) {
      this.finish(op, 'cancelled');
      return;
    }
    if (succeeded || committed) {
      // Keep the profile reserved until idle peers have dropped old credentials.
      // Cancellation during this await must not release that reservation either.
      op.cleaningCredentials = true;
      try {
        await this.onCredentialsChanged(op.profileId);
      } catch {
        // Fail closed: old peers may still be alive. Only host restart can safely
        // release this reservation; never expose a cleanup error's raw contents.
        this.record(op, { phase: 'cleanup', category: 'cleanup_failed', outcome: 'failed' });
        op.message = 'Credentials were saved, but local credential cleanup failed. Restart PAD before using this profile; do not repeat sign-in.';
        this.publish(op);
        this.diagnostic('Credential cleanup failed. Restart PAD before using this profile.');
        return;
      }
      op.cleaningCredentials = false;
    }
    if (this.ops.get(op.profileId) !== op) return;
    if (op.controller.signal.aborted) {
      this.finish(op, 'cancelled');
      return;
    }
    this.finish(op, succeeded ? 'succeeded' : 'failed', failure);
  }

  authPrompt(op, prompt) {
    if (op.controller.signal.aborted || this.ops.get(op.profileId) !== op) {
      return Promise.reject(new Error('Authentication cancelled'));
    }
    if (op.method === 'oauth' && ['openai', 'openai-codex'].includes(op.providerId) && prompt?.type === 'manual_code') {
      this.observe(op, 'manual_prompt_observed');
    }
    return new Promise((resolve, reject) => {
      const promptId = crypto.randomUUID();
      const signals = [op.controller.signal, prompt?.signal]
        .filter((signal) => signal && typeof signal.addEventListener === 'function');
      const entry = {
        cleanup: () => {
          for (const signal of signals) signal.removeEventListener('abort', onAbort);
        },
        resolve: (value) => settle(() => resolve(value)),
        reject: (error) => settle(() => reject(error)),
      };
      const settle = (finish) => {
        if (!op.pending.delete(promptId)) return;
        entry.cleanup();
        if (this.ops.get(op.profileId) === op) {
          op.promptId = undefined;
          op.promptKind = undefined;
          op.placeholder = undefined;
          op.options = undefined;
          this.publish(op);
        }
        finish();
      };
      const onAbort = () => entry.reject(new Error('Authentication cancelled'));
      op.pending.set(promptId, entry);
      op.promptId = promptId;
      op.promptKind = typeof prompt?.type === 'string' ? prompt.type : 'text';
      op.message = typeof prompt?.message === 'string' ? prompt.message : undefined;
      op.placeholder = typeof prompt?.placeholder === 'string' ? prompt.placeholder : undefined;
      op.options = prompt?.type === 'select' && Array.isArray(prompt.options)
        ? prompt.options.map((option) => ({
          id: String(option.id),
          label: typeof option.label === 'string' ? option.label : String(option.id),
        }))
        : undefined;
      // Authorization notices belong to the attempt, not just one input prompt.
      // ChatGPT emits auth_url immediately before its manual_code prompt.
      this.publish(op);
      for (const signal of signals) signal.addEventListener('abort', onAbort, { once: true });
      if (signals.some((signal) => signal.aborted)) entry.reject(new Error('Authentication cancelled'));
    });
  }

  authNotify(op, event) {
    if (op.controller.signal.aborted || this.ops.get(op.profileId) !== op) return;
    if (!event || typeof event !== 'object') return;
    switch (event.type) {
      case 'info':
        op.message = typeof event.message === 'string' ? event.message : op.message;
        break;
      case 'auth_url':
        this.observe(op, 'authorization_notice_observed');
        op.url = typeof event.url === 'string' ? event.url : undefined;
        if (typeof event.instructions === 'string') op.message = event.instructions;
        break;
      case 'device_code':
        op.userCode = typeof event.userCode === 'string' ? event.userCode : undefined;
        op.url = typeof event.verificationUri === 'string' ? event.verificationUri : undefined;
        break;
      case 'progress':
        if (op.providerId === 'openai' && op.method === 'oauth' && event.message === 'Exchanging authorization code for tokens...') {
          this.observe(op, 'token_exchange_observed');
        }
        op.message = typeof event.message === 'string' ? event.message : op.message;
        break;
      default:
        return;
    }
    this.publish(op);
  }

  respond(profileId, promptId, value) {
    const profile = this.requireProfile(profileId);
    const op = this.ops.get(profile.id);
    if (!op) throw this.makeError('No authentication is in progress for this profile');
    if (typeof promptId !== 'string' || !op.pending.has(promptId)) {
      throw this.makeError('That authentication prompt is no longer active');
    }
    // Secret values stay in memory and are passed straight back to the SDK.
    op.pending.get(promptId).resolve(String(value ?? ''));
    return {};
  }

  cancel(profileId, clientAttemptId) {
    const profile = this.requireProfile(profileId);
    const op = this.ops.get(profile.id);
    const attemptId = normalizeAttemptId(clientAttemptId);
    if (!op) {
      if (attemptId !== undefined) return {};
      throw this.makeError('No authentication is in progress for this profile');
    }
    // A stale cancel must not tear down a newer attempt for the same profile.
    if (attemptId !== undefined && attemptId !== op.attemptId) return {};
    op.controller.abort();
    if (!op.cleaningCredentials) this.finish(op, 'cancelled');
    return {};
  }

  /** Reserve a profile's auth slot synchronously for sign-out. */
  reserveLogout(profileId) {
    if (this.ops.has(profileId) || this.logoutGuards.has(profileId)) {
      throw this.makeError('Cannot sign out while authentication is in progress');
    }
    if (this.profileBusy(profileId)) throw this.makeError('Cannot sign out while this profile has an active task');
    this.logoutGuards.add(profileId);
  }

  releaseLogout(profileId) {
    this.logoutGuards.delete(profileId);
  }

  finish(op, phase, message) {
    if (this.ops.get(op.profileId) !== op) return;
    if (phase === 'failed') {
      this.record(op, { ...(op.failureDiagnostic ?? { phase: 'unknown', category: 'unknown' }), outcome: 'failed' });
    } else if (phase === 'succeeded') {
      this.record(op, { phase: 'sdk_login_cleanup', category: 'sdk_login_cleanup_success', outcome: 'sdk_login_cleanup_success' });
    } else if (phase === 'cancelled') {
      this.record(op, { phase: 'unknown', category: 'cancelled', outcome: 'cancelled' });
    }
    // Remove the op first so settling prompts cannot publish a live state.
    this.ops.delete(op.profileId);
    for (const entry of [...op.pending.values()]) {
      try {
        entry.reject(new Error('Authentication cancelled'));
      } catch {
        // Ignore a prompt that settled concurrently.
      }
    }
    op.pending.clear();
    op.phase = phase;
    op.promptId = undefined;
    op.promptKind = undefined;
    op.placeholder = undefined;
    op.options = undefined;
    op.url = undefined;
    op.userCode = undefined;
    op.message = phase === 'failed' ? (message ?? 'Authentication failed') : undefined;
    const state = buildAuthState(op);
    this.states.set(op.profileId, state);
    this.emit({ type: 'event', event: 'auth', data: state });
  }

  /** Abort every in-flight attempt without publishing (host shutdown). */
  close() {
    const ops = [...this.ops.values()];
    this.ops.clear();
    for (const op of ops) {
      try {
        op.controller.abort();
      } catch {
        // Ignore abort races during teardown.
      }
      for (const entry of [...op.pending.values()]) {
        try {
          entry.reject(new Error('Authentication cancelled'));
        } catch {
          // Ignore a prompt that settled concurrently.
        }
      }
      op.pending.clear();
    }
  }
}
