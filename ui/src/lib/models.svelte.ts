import { api } from './api';
type Model = { id: string; name: string; note?: string };
type Group = { base: string; name: string; models: Model[]; error?: string };
// `items` is the current provider's list (Settings); `groups` is every provider set up, for the model menu.
export const models = $state({ items: [] as Model[], groups: [] as Group[], error: '', loading: false });
let request: Promise<void> | undefined;
export const modelName = (id: string) => id === 'default' ? 'ChatGPT default' : models.items.find(m => m.id === id)?.name || id;
export function loadModels(fresh = false) {
  if (fresh) request = undefined;
  return request ||= (async () => {
    models.loading = true; models.error = '';
    try {
      const r = await api<{ models: Model[]; groups?: Group[]; error?: string }>('GET', 'api/models');
      models.items = r.models; models.groups = r.groups || []; models.error = r.error || '';
    }
    catch (e) { models.error = e instanceof Error ? e.message : String(e); }
    finally { models.loading = false; }
  })();
}
