<script lang="ts">
  import type { Course, Lesson, Exercise } from '../lib/types';
  import { appState, keyFor, save, runCode, installLanguage } from '../lib/state.svelte';
  import { languageName } from '../lib/format';
  import Icon from './Icon.svelte';
  import Formatted from './Formatted.svelte';
  let { c, l, ex }: { c: Course; l: Lesson; ex: Exercise } = $props();
  let key = $derived(keyFor(c, l)), run = $derived(appState.runs[key]);
  let busy = $derived(run?.running || run?.installing);
  let code = $derived(c.state.code[l.id] ?? ex.starter);
  function reset() {
    if (!confirm('Replace your code with the starter? Your edits will be lost.')) return;
    c.state.code[l.id] = ex.starter; delete appState.runs[key]; save(c);
  }
  function edit(value: string) { c.state.code[l.id] = value; save(c); }
</script>
<section class="exercise"><div class="kicker"><Icon name="Practise" /> Make it work · {languageName(ex.lang)}</div><h3>{ex.title}</h3><p><Formatted text={ex.prompt} /></p><div class="code-label"><label for="code">Your code</label><span>Tab = 4 spaces</span></div>
  <textarea id="code" spellcheck="false" autocapitalize="off" autocomplete="off" wrap="off" value={code} oninput={e => edit(e.currentTarget.value)} onkeydown={e => { if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); const el = e.currentTarget; el.setRangeText('    ', el.selectionStart, el.selectionEnd, 'end'); edit(el.value); } }}></textarea>
  <div class="row run-actions"><button class="btn" id="run-code" disabled={busy} onclick={() => runCode(false)}><Icon name="Watch" /> Run</button><button class="btn secondary" id="run-tests" disabled={busy} onclick={() => runCode(true)}><Icon name="Done when" /> Run tests</button><button class="link" id="reset-code" disabled={busy} onclick={reset}>Reset to starter</button></div>
  <div class="output" class:passed={!!run && !busy && !run.missing && !run.error && !!run.ok} class:failed={!!run && !busy && !run.missing && !run.error && !run.ok}><div class="output-header" role="status">{#if busy}<svg class="running-route" viewBox="0 0 52 20" aria-hidden="true"><path d="M0 15h12c14 0 4-10 18-10h22" /></svg>{/if}{run?.installing ? 'Installing ' + languageName(run.missing || '') + '…' : run?.running ? run.test ? 'Running tests…' : 'Running code…' : run?.missing ? languageName(run.missing) + ' is needed' : run?.error ? 'Run couldn’t finish' : run ? run.test ? run.ok ? 'Tests passed' : 'Tests failed' : run.ok ? 'Run finished' : 'Run failed' : 'Output'}</div>
    <pre id="output">{run?.installing ? 'Finish the system password dialog if it is open. Installation can take several minutes; you can keep reading while it runs.' : run?.running ? 'Compiling and running can take up to 60 seconds. You can keep reading while it runs.' : run?.output ?? 'Run your code to see the result here.'}</pre>
    {#if run?.missing}<div class="install-note"><p>{run.installing ? 'The system is installing a package. Keep the app open until it finishes.' : 'This installs a system package and will ask for your password.'}</p><button class="btn secondary" id="install-lang" disabled={run.installing} onclick={installLanguage}>{run.installing ? 'Installing…' : 'Install ' + languageName(run.missing)}</button>{#if run.installError}<p class="err" role="alert">{run.installError}</p>{/if}</div>{/if}
  </div>
  <details><summary>View the tests</summary><pre>{ex.tests}</pre></details>
</section>
