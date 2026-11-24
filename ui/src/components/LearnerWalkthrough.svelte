<script lang="ts">
  import type { Course, Walkthrough } from '../lib/types';
  import { api } from '../lib/api';
  import { enc, href } from '../lib/format';
  import { lessons } from '../lib/progress';
  import { message, saveFirst } from '../lib/state.svelte';

  let { c }: { c: Course } = $props();
  let additional = $state(''), editing = $state(false);
  let busy = $state(false), error = $state(''), status = $state('');
  let report = $derived(c.walkthrough);

  async function review() {
    busy = true; error = ''; status = 'Reading the lessons in order…';
    try {
      await saveFirst();
      c.walkthrough = await api<Walkthrough>('POST', 'api/courses/' + enc(c.id) + '/walkthrough',
        { additional: editing ? additional : report?.additional ?? '' });
      editing = false;
      status = 'Walkthrough complete.';
    } catch (e) { error = message(e); status = ''; }
    finally { busy = false; }
  }
</script>

<details class="note">
  <summary>Learner walkthrough{#if report?.findings.length} · {report.findings.length} {report.findings.length === 1 ? 'sticking point' : 'sticking points'}{/if}</summary>
  {#if report}
    <p class="small">AI walkthrough from {report.date}{report.outlines ? ` · ${report.outlines} outline-only lessons at the time` : ''}</p>
    <p>{report.perspective}</p>
    {#each report.findings as item}
      {@const current = lessons(c).find(l => l.id === item.lesson && l.title === item.title)}
      <h3>{#if current}<a href={href(c.id, item.lesson)}>{item.lesson} {item.title}</a>{:else}{item.lesson} {item.title}{/if}</h3>
      <blockquote>{item.quote}</blockquote>
      <p>{item.difficulty}</p>
      <p><strong>Suggested change</strong> {item.change}</p>
    {:else}
      <p>The reviewer found no specific sticking points in this pass.</p>
    {/each}
  {:else}
    <p>No walkthrough yet. The AI can follow the lessons using your goal, starting level and available resources.</p>
  {/if}
  <form class="form-col" onsubmit={e => { e.preventDefault(); void review(); }}>
    <label for="walkthrough-context">What else should the reviewer know about you? <span class="small">Optional</span></label>
    <textarea id="walkthrough-context" rows="2" maxlength="1200" disabled={busy}
      placeholder="For example, I share a computer and can practise for 20 minutes at a time."
      value={editing ? additional : report?.additional ?? ''} oninput={e => { additional = e.currentTarget.value; editing = true; }}></textarea>
    <button class="btn secondary" disabled={busy}>{report ? 'Run walkthrough again' : 'Run learner walkthrough'}</button>
    <span role="status">{status}</span>
    {#if error}<p role="alert" class="err">{error}</p>{/if}
  </form>
</details>
