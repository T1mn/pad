import { describe, expect, it } from 'vitest';
import { isDesktopAction, sanitizeDesktopParams } from './request-policy';

describe('remote request policy', () => {
  it('allows only the explicit Pi fast-mode flag on prompt requests', () => {
    expect(sanitizeDesktopParams('prompt', {
      task_id: 'task-1',
      prompt: 'hello',
      fast_mode: true,
    })).toEqual({ task_id: 'task-1', prompt: 'hello', fast_mode: true });
    expect(() => sanitizeDesktopParams('prompt', {
      task_id: 'task-1',
      prompt: 'hello',
      service_tier: 'fast',
    })).toThrow('Unsupported prompt parameters');
  });

  it('recognizes every remote action and admits only its exact public fields', () => {
    expect(isDesktopAction('remote_status')).toBe(true);
    expect(isDesktopAction('remote_set_enabled')).toBe(true);
    expect(isDesktopAction('remote_pair_begin')).toBe(true);
    expect(isDesktopAction('remote_pair_cancel')).toBe(true);
    expect(isDesktopAction('remote_device_revoke')).toBe(true);

    expect(sanitizeDesktopParams('remote_status', {})).toEqual({});
    expect(sanitizeDesktopParams('remote_pair_begin', {})).toEqual({});
    expect(sanitizeDesktopParams('remote_set_enabled', { enabled: true })).toEqual({ enabled: true });
    expect(sanitizeDesktopParams('remote_pair_cancel', { pairing_id: 'pair-1' })).toEqual({ pairing_id: 'pair-1' });
    expect(sanitizeDesktopParams('remote_device_revoke', { device_id: 'device-1' })).toEqual({ device_id: 'device-1' });
  });

  it('rejects credentials, network details, paths, and unrelated fields', () => {
    const cases = [
      ['remote_status', { token: 'secret' }],
      ['remote_set_enabled', { enabled: true, port: 443 }],
      ['remote_pair_begin', { qr_payload: 'secret' }],
      ['remote_pair_cancel', { pairing_id: 'pair-1', path: '/tmp/private' }],
      ['remote_device_revoke', { device_id: 'device-1', raw_error: 'private' }],
    ] as const;
    for (const [action, params] of cases) {
      expect(() => sanitizeDesktopParams(action, params)).toThrow(`Unsupported ${action} parameters`);
    }
  });
});
