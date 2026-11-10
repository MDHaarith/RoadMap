<script lang="ts">
  import { onMount } from 'svelte';
  import { api, apiError } from '../lib/api';
  import type { Course, Lesson } from '../lib/types';
  let { c, l }: { c: Course; l: Lesson } = $props();
  let selected = $state(0), loading = $state(false), text = $state(''), error = $state('');
  let request = 0;
  onMount(() => { void loadFile(0); return () => { request++; }; });
  async function loadFile(i: number) {
    const path = l.files?.[i]; if (path === undefined) return;
    const id = ++request; selected = i; loading = true; error = '';
    try { const r = await api<{ text: string }>('POST', 'api/file', { course: c.id, path }); if (id === request) text = r.text; }
    catch (err) { if (id === request) { const e = apiError(err); error = e.status === 404 ? 'This file is gone or isn’t readable. Choose another file, or check the project folder.' : e.message; } }
    finally { if (id === request) loading = false; }
  }
</script>
<p class="muted">Read these files from your project. Use your notes to trace what happens before you change anything.</p>
<div class="file-list" role="group" aria-label="Lesson files">{#each l.files || [] as f, i}<button aria-pressed={selected === i} onclick={() => loadFile(i)}>{f}</button>{/each}</div>
<div id="file-content">{#if loading}<p class="muted" role="status">Reading file…</p>{:else if error}<p class="err" role="alert">{error}</p>{:else}<p class="file-heading">{l.files?.[selected]}</p><!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="code-viewer" tabindex="0" role="region" aria-label={`${l.files?.[selected]} — read-only code, scroll to read`}>{#if text}{#each text.split('\n') as line}<span class="code-line">{line || ' '}</span>{/each}{/if}</div>{/if}</div>
