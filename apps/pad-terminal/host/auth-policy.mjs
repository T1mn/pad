// PAD-native subscription boundary only; the unmodified Pi CLI is unaffected.
// https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use
// https://geminicli.com/docs/resources/tos-privacy/
const legacySubscriptions = new Set(['google-gemini-cli', 'google-antigravity']);
export function authMethodAllowed(providerId, method) {
  return !legacySubscriptions.has(providerId) && !(providerId === 'anthropic' && method === 'oauth');
}
export function callableAuthTypes(provider) {
  return ['api_key', 'oauth'].filter((method) => {
    const handler = provider?.auth?.[method === 'api_key' ? 'apiKey' : 'oauth'];
    return typeof handler?.login === 'function' && authMethodAllowed(provider.id, method);
  });
}
export function credentialAllowed(providerId, type) {
  return authMethodAllowed(providerId, type);
}

export class AuthError extends Error {
  constructor(code) {
    super(code);
    this.name = 'AuthError';
    this.code = code;
  }
}
const messages = {
  identity_missing: 'Installation identity is unavailable. Restart PAD and try again.',
  identity_persistence: 'Installation identity could not be safely read or saved. Check the PAD data directory permissions and identity file; it was not replaced.',
  credential_sync: 'Credentials were saved, but local model refresh failed. Restart PAD to refresh; do not repeat sign-in.',
  unsupported: 'This authentication method is not available in PAD. Use a supported API key or configured provider flow.',
  sdk_load: 'Installed Pi SDK could not be loaded. Check the Pi installation.',
  runtime_create: 'Profile model runtime could not be created. Check the profile configuration.',
};
// Exact constants from the pinned installed SDK, not substring classification.
const pinnedMessages = new Map([
  ['Sign in with ChatGPT requires a device ID (UUID) for this installation', messages.identity_missing],
  ['Port 1455 is in use, probably by an unfinished login in another pi session or by the Codex CLI. Cancel that login and try again.', 'The sign-in callback port is busy. Cancel the unfinished Pi or Codex login and try again.'],
]);
const genericMessage = 'Authentication failed. Please try again.';
const networkCodes = new Set(['ENOTFOUND', 'EAI_AGAIN', 'ECONNREFUSED', 'ECONNRESET', 'ETIMEDOUT', 'UND_ERR_CONNECT_TIMEOUT', 'CERT_HAS_EXPIRED', 'UNABLE_TO_VERIFY_LEAF_SIGNATURE', 'DEPTH_ZERO_SELF_SIGNED_CERT']);
const callbackMessages = new Set([
  'Missing authorization code', 'Missing OAuth state', 'OAuth state mismatch',
  'OpenAI OAuth registration callback did not contain an issued client ID',
  'Paste the full callback URL from the browser',
  'The pasted callback URL must start with http://127.0.0.1:1455/auth/callback',
]);
const tokenMessages = new Set([
  'OpenAI OAuth token response must be an object',
  ...['access_token', 'refresh_token', 'scope', 'expires_in'].map((field) => `OpenAI OAuth token response has invalid ${field}`),
  'OpenAI OAuth token response did not contain an ID token',
  'OpenAI OAuth grant did not include chatgpt.tokens.use.direct',
]);

export const AUTH_TOKEN_BODY_MAX_BYTES = 16 * 1024;
const oauthTokenMeanings = new Map([
  ['invalid_request', 'The OAuth server reported an invalid token request.'],
  ['invalid_client', 'The OAuth server rejected client authentication.'],
  ['invalid_grant', 'The OAuth server rejected the authorization grant; this does not establish expiry or reuse.'],
  ['unauthorized_client', 'The OAuth server reported that this client is not authorized for this grant type.'],
  ['unsupported_grant_type', 'The OAuth server reported an unsupported grant type.'],
  ['invalid_scope', 'The OAuth server reported an invalid scope.'],
]);
function tokenBodyClassification(body) {
  // Reject before parsing: never parse a truncated JSON document. UTF-8 bytes,
  // not just JS character count, enforce the limit. Nothing from body escapes.
  if (body.length > AUTH_TOKEN_BODY_MAX_BYTES || Buffer.byteLength(body, 'utf8') > AUTH_TOKEN_BODY_MAX_BYTES) {
    return { category: 'token_body_unrecognized', meaning: 'The response body was not safely recognized.' };
  }
  if (/^\s*(?:<!doctype\s+html(?:\s[^>]*)?>|<html(?:\s|>))/i.test(body)) {
    return { category: 'token_html_response', meaning: 'The token endpoint returned an HTML document, not a recognized OAuth error. This does not identify its source.' };
  }
  try {
    const data = JSON.parse(body);
    if (data !== null && typeof data === 'object' && !Array.isArray(data)) {
      if (typeof data.error === 'string' && oauthTokenMeanings.has(data.error)) {
        return { category: `token_oauth_${data.error}`, meaning: oauthTokenMeanings.get(data.error) };
      }
      return { category: 'token_json_unrecognized', meaning: 'The response was a JSON object without a recognized standard OAuth error.' };
    }
  } catch { /* malformed JSON and statusText are deliberately indistinguishable */ }
  return { category: 'token_body_unrecognized', meaning: 'The response body was not safely recognized.' };
}

// Only app-owned text leaves this classifier. The openai prefix is verified
// against installed SDK 1.0.2; retain the existing provider-specific Codex guard.
// Suffixes can contain secrets and are never returned or persisted.
export function classifyAuthError(error, { providerId, method } = {}) {
  const result = (phase, category, message, extra = {}) => ({ phase, category, message, ...extra });
  if (error instanceof AuthError) {
    return result(error.code === 'credential_sync' ? 'credential_sync' : 'setup',
      Object.hasOwn(messages, error.code) ? error.code : 'unknown', messages[error.code] ?? genericMessage);
  }
  if (!(error instanceof Error)) return result('unknown', 'unknown', genericMessage);
  const openai = method === 'oauth' && ['openai', 'openai-codex'].includes(providerId);
  if (openai) {
    const text = error.message;
    if (pinnedMessages.has(text)) return result(text.startsWith('Port ') ? 'callback_server' : 'setup',
      text.startsWith('Port ') ? 'callback_port_busy' : 'identity_missing', pinnedMessages.get(text));
    if (callbackMessages.has(text) || (providerId === 'openai-codex' && text === 'State mismatch')) {
      return result('callback_validation', 'callback_invalid', 'The sign-in callback could not be validated. Start a new sign-in and use its full final redirect URL.');
    }
    if (providerId === 'openai-codex' && ['OAuth callback server did not bind to TCP', 'OAuth callback server closed'].includes(text)) {
      return result('callback_server', 'callback_server_failed', 'The sign-in callback listener failed. Start a new sign-in after checking local listener availability.');
    }
    if (/^ChatGPT authorization failed: [\s\S]*$/.test(text) ||
        (providerId === 'openai-codex' && /^OpenAI authorization failed: [\s\S]*$/.test(text))) {
      return result('authorization', 'authorization_rejected', 'ChatGPT authorization was rejected. Start a new sign-in; no authorization details were retained.');
    }
    const http = text.match(providerId === 'openai'
      ? /^OpenAI OAuth token request failed \(([1-5][0-9]{2})\): /
      : /^OpenAI Codex token exchange failed \(([1-5][0-9]{2})\): /);
    if (http) {
      const { category, meaning } = tokenBodyClassification(text.slice(http[0].length));
      return result('token_exchange', category,
        `The sign-in token exchange was rejected (HTTP ${http[1]}). ${meaning} Start a new sign-in; if it repeats, check the provider or network configuration. Response details were not retained.`,
        { httpStatus: Number(http[1]) });
    }
    if (tokenMessages.has(text) || (providerId === 'openai-codex' &&
      (/^OpenAI Codex token exchange response missing fields: [\s\S]*$/.test(text) || text === 'Failed to extract accountId from token'))) {
      return result('token_exchange', 'token_response_invalid', 'The sign-in token response failed SDK validation. Start a new sign-in; token details were not retained.');
    }
  }
  // Read only a single allowlisted code, never a cause message/stack or chain.
  const errno = networkCodes.has(error.code) ? error.code
    : networkCodes.has(error.cause?.code) ? error.cause.code : undefined;
  if (errno) return result('unknown', 'transport_error',
    `Authentication encountered a transport error (${errno}). Check connectivity or TLS configuration before trying again.`, { errno });
  if (openai && ['EADDRINUSE', 'EACCES', 'EADDRNOTAVAIL'].includes(error.code)) {
    return result('unknown', 'local_socket_error', `Authentication encountered a local socket error (${error.code}). Check the local callback listener configuration.`, { errno: error.code });
  }
  return result('unknown', 'unknown', genericMessage);
}
export function safeAuthMessage(error, context) {
  // Preserve existing callers' exact constant handling without inferring a flow.
  if (!context && error instanceof Error && pinnedMessages.has(error.message)) return pinnedMessages.get(error.message);
  return classifyAuthError(error, context).message;
}
