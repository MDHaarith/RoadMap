<script lang="ts">
  import type { Course, Lesson, Portfolio } from '../lib/types';
  import { api } from '../lib/api';
  import { adaptNow, message } from '../lib/state.svelte';
  let { c, l }: {c: Course; l: Lesson} = $props();
  let record = $derived(c.state.portfolio?.[l.id]);
  let latest = $derived(record?.attempts[record.attempts.length - 1]);
  let work = $state(''), style = $state('Coach me'), unaided = $state(false), error = $state(''), busy = $state(false);
  async function submit(mode: 'challenge' | 'practice' | 'independent' | 'leave') {
    if (busy) return;
    busy = true; error = '';
    try {
      const result = await api<Portfolio>('POST','api/courses/'+c.id+'/milestone',{lesson:l.id,version:record?.version || 0,mode,work,style});
      (c.state.portfolio ||= {})[l.id] = result;
      unaided = false;
      if (mode === 'practice' || mode === 'independent') work = '';
    } catch(e) { error = message(e); } finally { busy = false; }
  }
</script>
<section class="card form-col" aria-label="Milestone studio">
  <h2>Milestone studio</h2><p>{l.checkpoint?.task || l.goal}</p>
  {#if l.checkpoint?.preparation}<p><strong>Before you begin</strong> {l.checkpoint.preparation}</p>{/if}
  {#if l.checkpoint}<h3>Success criteria</h3><p style="white-space:pre-line">{l.checkpoint.criteria}</p>{:else if l.done?.length}<h3>Success criteria</h3><ul>{#each l.done as criterion}<li>{criterion}</li>{/each}</ul>{/if}
  {#if latest}<h3>{latest.mode === 'independent' ? 'Independent attempt' : 'Guided practice'}</h3>
    <p><strong>Evidence</strong> {latest.evidence}</p><p><strong>Improve next</strong> {latest.improvement}</p><p><strong>Next step</strong> {latest.next}</p>
    <details><summary>Work and feedback history ({record?.attempts.length})</summary>{#each record?.attempts || [] as a}<article><h4>{a.mode === 'practice' ? 'Guided' : 'Independent'} · {a.task}</h4><p style="white-space:pre-wrap">{a.work}</p><p>{a.evidence} {a.improvement} {a.next}</p></article>{/each}</details>
    <button type="button" class="btn secondary sm" onclick={() => adaptNow(true)}>Preview roadmap recommendation</button>
  {/if}
  {#if record?.challenge}<h3>Independent challenge</h3><p>{record.challenge}</p><button class="link" disabled={busy} onclick={() => submit('leave')}>Return to guided practice</button>{/if}
  {#if latest && !work && !record?.challenge}<button class="link" onclick={() => work = latest?.work || ''}>Revise my last submission</button>{/if}
  <label for="studio-work">{record?.challenge ? 'Your independent response' : 'Your work or revision'}</label>
  <textarea id="studio-work" rows="5" maxlength="8000" bind:value={work} placeholder="Paste your work, results, or observations here."></textarea>
  {#if !record?.challenge}<label for="studio-style">Feedback style</label><select id="studio-style" bind:value={style}><option>Coach me</option><option>Give me a hint</option><option>Show a worked example</option></select>{/if}
  {#if record?.challenge}<label><input type="checkbox" bind:checked={unaided} /> I completed this attempt without coaching.</label>{/if}
  <div class="row"><button class="btn" disabled={busy || !work.trim() || (!!record?.challenge && !unaided)} onclick={() => submit(record?.challenge ? 'independent' : 'practice')}>{record?.challenge ? 'Assess my attempt' : 'Review my work'}</button>
    {#if !record?.challenge}<button class="btn secondary" disabled={busy || !!work.trim()} onclick={() => submit('challenge')}>Try a fresh task independently</button>{/if}</div>
  {#if busy}<p role="status">Preparing your {work.trim() ? 'feedback' : 'challenge'}…</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>
