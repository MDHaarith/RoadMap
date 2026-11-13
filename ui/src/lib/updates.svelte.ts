import { updateCommand, updateState } from './api';
export const updates = $state({ version: '', state: '', available: '', dismissed: false });
export async function refreshUpdate() {
  const status = await updateCommand<{ version: string; state: string; available: string }>('update_status');
  status.state = status.state === 'error' && updates.state === 'install-error' ? 'install-error' : updateState(status.state, updates.state === 'error');
  Object.assign(updates, status);
}
export async function checkUpdate(manual = false) {
  try { await updateCommand('update_check'); await refreshUpdate(); }
  catch { if (manual) updates.state = 'error'; }
}
export async function installUpdate() {
  try { await updateCommand('update_install'); }
  catch { updates.state = 'install-error'; }
}
export function startUpdates() {
  let stopped = false;
  let poll: ReturnType<typeof setInterval> | undefined;
  let periodic: ReturnType<typeof setInterval> | undefined;
  const timer = setTimeout(() => {
    void refreshUpdate().then(() => {
      if (stopped) return;
      void checkUpdate();
      poll = setInterval(() => { void refreshUpdate().catch(() => {}); }, 2000);
      periodic = setInterval(() => { void checkUpdate(); }, 6 * 60 * 60 * 1000);
    }).catch(() => {});
  }, 4000);
  return () => { stopped = true; clearTimeout(timer); clearInterval(poll); clearInterval(periodic); };
}
