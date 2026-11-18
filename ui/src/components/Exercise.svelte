<script lang="ts">
  import Skeleton from './Skeleton.svelte';
  import type { Course, Lesson, Exercise } from '../lib/types';
  import { appState, keyFor, save, runCode, installLanguage } from '../lib/state.svelte';
  import { languageName } from '../lib/format';
  import { highlight } from '../lib/highlight';
  import Icon from './Icon.svelte';
  import Formatted from './Formatted.svelte';
  import Elapsed from './Elapsed.svelte';
  import BusyLabel from './BusyLabel.svelte';
  import { confirmAsk } from './Confirm.svelte';
  let { c, l, ex }: { c: Course; l: Lesson; ex: Exercise } = $props();
  let key = $derived(keyFor(c, l)), run = $derived(appState.runs[key]);
  let busy = $derived(run?.running || run?.installing);
  let code = $derived(c.state.code[l.id] ?? ex.starter);
  async function reset() {
    if (!(await confirmAsk({ message: 'Replace your code with the starter? Your edits will be lost.', action: 'Replace code', danger: true }))) return;
    c.state.code[l.id] = ex.starter; delete appState.runs[key]; save(c);
  }
  let hl = $state<HTMLPreElement>();
  function edit(value: string) { c.state.code[l.id] = value; save(c); }
</script>
<section class="exercise"><div class="kicker">Coding exercise · {languageName(ex.lang)}</div><h3>{ex.title}</h3><p><Formatted text={ex.prompt} /></p><div class="code-label"><label for="code">Your code</label><span>Tab = 4 spaces</span></div>
  <div class="code-editor"><pre class="code-hl" aria-hidden="true" bind:this={hl}>{@html highlight(code, ex.lang)}{"\n"}</pre><textarea id="code" onscroll={e => { if (hl) { hl.scrollTop = e.currentTarget.scrollTop; hl.scrollLeft = e.currentTarget.scrollLeft; } }} spellcheck="false" autocapitalize="off" autocomplete="off" wrap="off" value={code} oninput={e => edit(e.currentTarget.value)} onkeydown={e => { if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); const el = e.currentTarget; el.setRangeText('    ', el.selectionStart, el.selectionEnd, 'end'); edit(el.value); } }}></textarea></div>
  <div class="row run-actions"><button class="btn" id="run-code" disabled={busy} onclick={() => runCode(false)}><Icon name="Watch" /> Run code</button><button class="btn secondary" id="run-tests" disabled={busy} onclick={() => runCode(true)}><Icon name="Done when" /> Run tests</button><button class="link" id="reset-code" disabled={busy} onclick={reset}>Reset to starter</button></div>
  <div class="output" class:passed={!!run && !busy && !run.missing && !run.error && !!run.ok} class:failed={!!run && !busy && !run.missing && !run.error && !run.ok}><div class="output-header" role="status">{run?.installing ? 'Installing ' + languageName(run.missing || '') + '…' : run?.running ? run.test ? 'Running tests… ' : 'Running code… ' : run?.missing ? languageName(run.missing) + ' is needed' : run?.error ? 'The code could not run' : run ? run.test ? run.ok ? 'Tests passed' : 'Tests failed' : run.ok ? 'Run finished' : 'Run failed' : 'Output'}{#if busy}<Elapsed />{/if}</div>
    <pre id="output">{run?.installing ? 'Finish the system password dialog if it is open. Installation can take several minutes; you can keep reading while it runs.' : run?.running ? 'Compiling and running can take up to 60 seconds. You can keep reading while it runs.' : run && run.output === '' ? 'Your program printed nothing. Write the code that prints, then run it again.' : run?.output ?? 'Run your code to see the result here.'}</pre>
    {#if busy}<Skeleton rows={3} />{/if}
    {#if run?.missing}<div class="install-note"><p>{run.installing ? 'The system is installing a package. Keep the app open until it finishes.' : 'This installs a system package and will ask for your password.'}</p><button class="btn secondary" id="install-lang" disabled={run.installing} onclick={installLanguage}><BusyLabel busy={!!run.installing} idle={'Install ' + languageName(run.missing)} label="Installing…" /></button>{#if run.installError}<p class="err" role="alert">{run.installError}</p>{/if}</div>{/if}
  </div>
  <details><summary>View the tests</summary><pre>{ex.tests}</pre></details>
</section>
