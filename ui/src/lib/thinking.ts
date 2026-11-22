import { api } from './api';

export interface DraftPhase { title: string; lessons: string[] }
/** The roadmap as the model writes it: stages for long roadmaps, otherwise phases only. */
export interface Draft { title: string; summary: string; stages: { title: string; blurb: string; phases: DraftPhase[] }[]; phases: DraftPhase[] }

/** Polls what the model is thinking while a request that sent `key` as "thinking" runs. Call `stop` when it ends.
 *  `onDraft` gets the roadmap written so far, when the request is building one and the AI service streams its answer. */
export function watchThinking(onText: (text: string) => void, onDraft?: (draft: Draft) => void) {
  const key = crypto.randomUUID().replaceAll('-', '');
  const timer = setInterval(() => {
    api<{ text: string; draft?: Draft | null }>('GET', 'api/thinking/' + key).then(r => { if (r.text) onText(r.text); if (r.draft && onDraft) onDraft(r.draft); }, () => {});
  }, 1000);
  return { key, stop: () => clearInterval(timer) };
}
