<script lang="ts">
  // Tells the learner why AI features will not work yet, before a call fails: no service, or Codex signed out or not installed.
  import { aiProblem, refreshConfig, message } from '../lib/state.svelte';
  import BusyLabel from './BusyLabel.svelte';
  let { why, compact = false }: { why: string; compact?: boolean } = $props();
  let problem = $derived(aiProblem()), checking = $state(false), error = $state('');
  async function check() {
    checking = true; error = '';
    try { await refreshConfig(); } catch (e) { error = message(e); } finally { checking = false; }
  }
</script>
{#if problem}
  <div class="note setup" class:compact role="status">
    <div>
      <strong>{problem === 'none' ? 'No AI service connected' : problem === 'signed_out' ? 'Codex is signed out' : 'Codex is not installed'}</strong>
      {#if problem === 'none'}<p>{why}</p>
      {:else if problem === 'signed_out'}<p>Open a terminal, run <code>codex login</code> and sign in with ChatGPT. Then check again.</p>
      {:else}<p>The Codex command was not found on this computer. Install it, run <code>codex login</code> in a terminal to sign in with ChatGPT, then check again.</p>{/if}
      {#if error}<p class="err">{error}</p>{/if}
    </div>
    {#if problem === 'none'}<a class="btn secondary" class:sm={compact} href="#/settings">Open Settings</a>
    {:else}<button type="button" class="btn secondary" class:sm={compact} disabled={checking} onclick={check}><BusyLabel busy={checking} idle="Check again" label="Checking…" /></button>{/if}
  </div>
{/if}
