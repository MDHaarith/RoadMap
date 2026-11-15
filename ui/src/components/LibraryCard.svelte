<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { href, enc } from '../lib/format';
  import { loadLibrary, toast, message } from '../lib/state.svelte';
  import type { LibraryCourse } from '../lib/types';
  import Progress from './Progress.svelte';
  import { confirmAsk } from './Confirm.svelte';
  let { course: c }: { course: LibraryCourse } = $props();
  let busy = $state(false);
  let alive = true;
  onMount(() => () => { alive = false; });
  let complete = $derived(c.total > 0 && c.done >= c.total);
  async function change(remove = false) {
    if (remove && !(await confirmAsk({ message: `Delete "${c.title}" and all its progress? This can't be undone.`, action: 'Delete', danger: true }))) return;
    busy = true;
    try { await api(remove ? 'DELETE' : 'PUT', 'api/courses/' + enc(c.id) + (remove ? '' : '/paused'), remove ? undefined : { paused: !c.paused }); if (alive) await loadLibrary(); }
    catch (error) { toast(message(error)); } finally { busy = false; }
  }
</script>
<section class="lib-card ruled-row"><div class="row between"><span class="small">{c.project ? 'Project roadmap' : 'Roadmap'}</span>{#if complete}<span class="tag ok">Finished</span>{:else if c.paused}<span class="tag">Paused</span>{/if}</div><h3><a href={href(c.id)}>{c.title}</a></h3><p>{c.summary}</p><div class="card-progress"><Progress n={c.done} total={c.total} /><span>{c.done} / {c.total}</span></div><div class="card-actions"><a href={href(c.id)}>Open</a>{#if !complete}<button class="link" disabled={busy} onclick={() => change()}>{c.paused ? 'Resume' : 'Pause'}</button>{/if}<span class="grow"></span><button class="link danger" disabled={busy} onclick={() => change(true)}>Delete</button></div></section>
