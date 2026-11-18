import { api } from './api';

/** Polls what the model is thinking while a request that sent `key` as "thinking" runs. Call `stop` when it ends. */
export function watchThinking(onText: (text: string) => void) {
  const key = crypto.randomUUID().replaceAll('-', '');
  const timer = setInterval(() => {
    api<{ text: string }>('GET', 'api/thinking/' + key).then(r => { if (r.text) onText(r.text); }, () => {});
  }, 1000);
  return { key, stop: () => clearInterval(timer) };
}
