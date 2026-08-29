import { describe, expect, it, vi } from 'vitest';
import { redactRemotePowerSignal, RemotePowerSaveCoordinator } from './remote-power-blocker';

describe('RemotePowerSaveCoordinator', () => {
  it('uses prevent-app-suspension only for an online remote device and stops cleanly', () => {
    const blocker = { start: vi.fn(() => 41), stop: vi.fn() };
    const coordinator = new RemotePowerSaveCoordinator(blocker);

    coordinator.observe({ remote: { enabled: true, active_connections: 0, devices: [] } });
    expect(blocker.start).not.toHaveBeenCalled();

    coordinator.observe({ remote: { enabled: true, active_connections: 1, devices: [] } });
    expect(blocker.start).toHaveBeenCalledOnce();
    expect(blocker.start).toHaveBeenCalledWith('prevent-app-suspension');

    coordinator.observe({ remote: { enabled: true, active_connections: 2, devices: [{ online: true }] } });
    expect(blocker.start).toHaveBeenCalledOnce();

    coordinator.observe({ remote: { enabled: true, active_connections: 0, devices: [{ online: false }] } });
    expect(blocker.stop).toHaveBeenCalledWith(41);

    coordinator.observe({
      type: 'backend_event',
      payload: { event: { kind: 'remote_changed', payload: { remote: { enabled: true, active_connections: 0, devices: [{ online: true }] } } } },
    });
    expect(blocker.start).toHaveBeenCalledTimes(2);
    coordinator.dispose();
    expect(blocker.stop).toHaveBeenLastCalledWith(41);
  });

  it('ignores unrelated or malformed events and never requests system sleep prevention', () => {
    const blocker = { start: vi.fn(() => 7), stop: vi.fn() };
    const coordinator = new RemotePowerSaveCoordinator(blocker);
    coordinator.observe({ remote: { enabled: 'yes', active_connections: 1 } });
    coordinator.observe({ event: { kind: 'task_changed', payload: { active_connections: 2 } } });
    coordinator.dispose();
    expect(blocker.start).not.toHaveBeenCalled();
    expect(blocker.stop).not.toHaveBeenCalled();
  });

  it('keeps the Mac active across Profile switches without exposing the host-global signal', () => {
    const blocker = { start: vi.fn(() => 19), stop: vi.fn() };
    const coordinator = new RemotePowerSaveCoordinator(blocker);
    const event = {
      type: 'backend_event',
      payload: {
        event: {
          kind: 'remote_changed',
          payload: {
            has_online_remote: true,
            remote: { enabled: true, active_connections: 0, devices: [] },
          },
        },
      },
    };

    coordinator.observe(event);
    expect(blocker.start).toHaveBeenCalledOnce();

    // A subsequent Profile-scoped status must not release another Profile's live connection.
    coordinator.observe({ remote: { enabled: true, active_connections: 0, devices: [] } });
    expect(blocker.stop).not.toHaveBeenCalled();

    const rendererEvent = redactRemotePowerSignal(event);
    expect(JSON.stringify(rendererEvent)).not.toContain('has_online_remote');
    expect(JSON.stringify(rendererEvent)).toContain('remote_changed');

    coordinator.observe({
      type: 'backend_event',
      payload: {
        event: {
          kind: 'remote_changed',
          payload: {
            has_online_remote: false,
            remote: { enabled: true, active_connections: 0, devices: [] },
          },
        },
      },
    });
    expect(blocker.stop).toHaveBeenCalledWith(19);
  });
});
