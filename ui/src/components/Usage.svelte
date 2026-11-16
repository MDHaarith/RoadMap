<script lang="ts">
  import type { Usage } from '../lib/types';
  import { api } from '../lib/api';
  import { toast, message } from '../lib/state.svelte';
  import BusyLabel from './BusyLabel.svelte';
  let { usage: u = {} }: { usage?: Usage } = $props();
  let busy = $state(false);
  const n = (x = 0) => x.toLocaleString();
  // u.since counts whole days since 1970 (UTC); show it as 8 Nov 2025, never 11/8/2025.
  const day = (d: number) => { const t = new Date(d * 864e5); return `${t.getUTCDate()} ${t.toLocaleString('en', { month: 'short', timeZone: 'UTC' })} ${t.getUTCFullYear()}`; };
  async function reset() {
    busy = true;
    try { await api('PUT', 'api/usage'); u = { since: Math.floor(Date.now() / 864e5) }; }
    catch (error) { toast(message(error)); } finally { busy = false; }
  }
</script>
<div id="usage"><p class="usage-total"><b>{n((u.in || 0) + (u.out || 0))}</b> tokens in {n(u.calls)} request{u.calls === 1 ? '' : 's'}{u.since ? ' since ' + day(u.since) : ''}</p><p class="small">{n(u.in)} sent · {n(u.out)} received. A count is an estimate when the service does not report it.</p><button class="btn secondary sm" id="usage-reset" type="button" disabled={busy} onclick={reset}><BusyLabel {busy} idle="Reset counter" label="Resetting…" /></button></div>
