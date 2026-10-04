import crypto from 'node:crypto';
import fs from 'node:fs';
import { nowIso } from './workbench-store.mjs';

// Pi alone has a compatible native continuation format. Preserve its complete
// tree/compactions in a private snapshot; never let Pi open/migrate the source.
export async function importLocalPi(host, id, profileId) {
  const profile = host.requireProfile(profileId);
  const { record, text } = await host.localSessions.piCopy(id);
  if (host.closed) throw new Error('宿主正在退出');
  const workspace = host.addWorkspace(record.cwd);
  const task = {
    id: `task-${crypto.randomUUID()}`, workspaceId: workspace.id, profileId: profile.id,
    title: `Pi 副本 · ${record.title}`.slice(0, 120), status: 'idle', updatedAt: nowIso(),
  };
  const target = host.taskSessionPath(task);
  if (!target) throw new Error('无法创建隔离的 Pi 会话目录');
  // The model remains unselected: only the user may choose one in this profile.
  task.sessionFile = target;
  fs.writeFileSync(target, text, { flag: 'wx', mode: 0o600 });
  try {
    host.store.mutate((draft) => { draft.tasks.push(task); });
  } catch (error) {
    fs.rmSync(target, { force: true });
    throw error;
  }
  host.scheduleSnapshot();
  return { task, snapshot: host.snapshot() };
}
