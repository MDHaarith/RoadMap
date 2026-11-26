<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { message } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import Icon from '../components/Icon.svelte';
  import Progress from '../components/Progress.svelte';
  import Waiting from '../components/Waiting.svelte';
  import LoadError from '../components/LoadError.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  type File = { path: string; name: string; title: string; summary: string; lessons: number; done: number; template: boolean; modified: number };
  let files = $state<File[]>([]), loading = $state(true), error = $state(''), busy = $state(''), importError = $state('');
  let alive = true;
  const day = (s: number) => new Date(s * 1000).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
  function load() { loading = true; error = ''; api<{ files: File[] }>('GET', 'api/import/files').then(r => { if (alive) files = r.files; }).catch(e => { if (alive) error = message(e); }).finally(() => { if (alive) loading = false; }); }
  onMount(() => { load(); return () => { alive = false; }; });
  // path: a file from the list; none opens the system file picker.
  async function bring(path = '') {
    busy = path || 'pick'; importError = '';
    try { const r = await api<{ id?: string }>('POST', 'api/import', path ? { path } : {}); if (r.id && alive) location.hash = href(r.id); }
    catch (e) { if (alive) importError = message(e); } finally { if (alive) busy = ''; }
  }
</script>
<div class="wrap narrow import screen-enter"><a class="back-map" href="#/"><Icon name="back" /> Back</a>
  <div class="page-head"><h1>Import a roadmap</h1>{#if files.length}<div class="row"><button type="button" class="btn secondary sm" id="import-pick" disabled={!!busy} onclick={() => void bring()}><BusyLabel busy={busy === 'pick'} idle="Choose another file" label="Opening…" /></button></div>{/if}</div>
  {#if importError}<p class="err" role="alert">{importError}</p>{/if}
  {#if loading}<Waiting skeleton="rows" inline title="Looking for roadmap files" />
  {:else if error}<LoadError what="Roadmap files could not be listed." {error} retry={load} />
  {:else if !files.length}<div class="empty-state"><Icon name="empty-route" /><h2>No roadmap files found.</h2><p>Files saved with Export or Share as template end in <code>.roadmap.json</code>. None are in Downloads, Documents, Desktop or your home folder.</p><button type="button" class="btn" id="import-pick" disabled={!!busy} onclick={() => void bring()}><BusyLabel busy={busy === 'pick'} idle="Choose a file" label="Opening…" /></button></div>
  {:else}<p class="muted lead">Files saved with Export or Share as template, newest first, from Downloads, Documents, Desktop and your home folder. Importing adds a copy; the file stays where it is.</p>
    <div class="import-list ruled stagger">{#each files as f (f.path)}<section class="import-row">
      <span class="import-stop" class:template={f.template} aria-hidden="true"></span>
      <div class="grow"><div class="import-kind small">{f.template ? 'Template · no progress' : 'Export with progress'}</div><h3>{f.title || f.name}</h3>{#if f.summary}<p>{f.summary}</p>{/if}
        <div class="import-meta"><Progress n={f.template ? 0 : f.done} total={f.lessons} /><span class="count">{f.template ? f.lessons + ' lessons' : `${f.done} of ${f.lessons} lessons done`}</span><span class="small" title={f.path}>{f.name} · {day(f.modified)}</span></div></div>
      <button type="button" class="btn secondary sm" disabled={!!busy} onclick={() => void bring(f.path)} aria-label={`Import ${f.title || f.name}`}><BusyLabel busy={busy === f.path} idle="Import" label="Importing…" /></button>
    </section>{/each}</div>{/if}
</div>
