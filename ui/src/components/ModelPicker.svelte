<script lang="ts">
  // The app-wide AI model menu, shared by the tutor composer and the new-roadmap form.
  import { api } from '../lib/api';
  import { models, loadModels, modelName } from '../lib/models.svelte';
  import { appState, refreshConfig } from '../lib/state.svelte';
  import Select from './Select.svelte';
  let { id, ariaLabel = 'AI model' }: { id?: string; ariaLabel?: string } = $props();
  // Menu values are "provider\nmodel": the same model id can exist at two providers.
  let modelOptions = $derived.by(() => {
    const current = appState.config.base + '\n' + appState.config.model;
    const options: { value: string; label: string; hint?: string; group?: string }[] = models.groups.flatMap(g => g.models.map(m => ({ value: g.base + '\n' + m.id, label: m.name, hint: m.note || undefined, group: g.name })));
    if (!options.some(o => o.value === current)) options.unshift({ value: current, label: modelName(appState.config.model) || 'Choose model' });
    return options;
  });
  let modelProblems = $derived([models.error, ...models.groups.filter(g => g.error && g.base !== appState.config.base).map(g => `${g.name}: ${g.error}`)].filter(Boolean).join('\n'));
  async function chooseModel(value: string) {
    const cut = value.indexOf('\n'), base = value.slice(0, cut), model = value.slice(cut + 1);
    try {
      await api('PUT', 'api/model', { model, base });
      if (base === appState.config.base) appState.config.model = model;
      else { await refreshConfig(); void loadModels(true); }
    }
    catch (e) { models.error = e instanceof Error ? e.message : String(e); }
  }
</script>
<Select {id} {ariaLabel} loading={models.loading && !models.error} title={modelProblems || (models.loading ? 'Loading models…' : '')} onopen={() => void loadModels()} value={appState.config.base + '\n' + appState.config.model} onchange={value => void chooseModel(value)} heading="AI model for the whole app" options={modelOptions} />
