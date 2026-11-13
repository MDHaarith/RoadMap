import { api } from './api';
export const models = $state({ items: [] as { id: string; name: string }[], error: '', loading: false });
let request: Promise<void> | undefined;
export const modelName = (id: string) => id === 'default' ? 'Codex default' : models.items.find(m => m.id === id)?.name || id;
export function loadModels() {
  return request ||= (async () => {
    models.loading = true;
    try { models.items = (await api<{ models: { id: string; name: string }[] }>('GET', 'api/models')).models; }
    catch (e) { models.error = e instanceof Error ? e.message : String(e); }
    finally { models.loading = false; }
  })();
}
