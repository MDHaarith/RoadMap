import { api, apiError, chatStream } from './api';
import { enc, href } from './format';
import { loadModels } from './models.svelte';
import { watchThinking } from './thinking';
import { openUrl } from '@tauri-apps/plugin-opener';
import { lessons, checked, passed } from './progress';
import type { Course, Lesson, LibraryCourse, Settings, ReviewData, Message, Session, RunState, Activity, PlanPreview } from './types';

export const appState = $state({
  course: null as Course | null,
  current: null as Lesson | null,
  screen: 'loading', error: '', routeVersion: 0,
  openVideo: undefined as string | undefined, library: [] as LibraryCourse[], config: {} as Settings,
  review: { count: 0, questions: [] } as ReviewData,
  // Today's steps toward the daily goal and the streak, for the home page.
  activity: null as Activity | null,
  toast: '', ideasOpen: false, ideas: null as string[] | null,
  busyKey: '', adapting: false, suggestion: null as { cid: string; preview: PlanPreview } | null,
  // True while the ChatGPT sign-in page is open in the browser; see signInChatGpt.
  signingIn: false,
  // What the tutor is doing while a reply is on its way, for example "Reading the video transcript". Empty when idle.
  tutorStatus: '',
  // The tutor's thinking effort this session; empty means the Settings default.
  tutorEffort: '',
  // A reply that failed, per lesson: the question it answered and why it failed. Shown in the log with Try again.
  tutorErrors: {} as Record<string, { text: string; error: string }>,
  // Why progress could not be saved; empty once every roadmap's progress is saved.
  saveError: '',
  chats: {} as Record<string, Message[]>,
  sessions: {} as Record<string, Session>,
  practiceViews: {} as Record<string, string>,
  practiceBusy: {} as Record<string, boolean>,
  practiceFresh: {} as Record<string, boolean>,
  practiceErrors: {} as Record<string, string>,
  runs: {} as Record<string, RunState>,
  runFails: {} as Record<string, number>,
  visited: {} as Record<string, string[]>,
  activeStep: 'watch', examStage: 0,
});
let toastTimer: ReturnType<typeof setTimeout>;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
let saveFn: (() => Promise<void>) | undefined;
// Saves go out one after another, so an older copy of the progress can never land after a newer one.
let saving: Promise<void> = Promise.resolve();
let ideasTimer: ReturnType<typeof setTimeout> | undefined;
let adaptTimer: ReturnType<typeof setTimeout> | undefined;
const expanding = new Map<string, Promise<void>>();
export const keyFor = (c: Course, l: Lesson) => c.id + '/' + l.id;
export const message = (error: unknown) => apiError(error).message;
export function toast(text: string) {
  appState.toast = text;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => appState.toast = '', 9000);
}
// Every map the screens write into exists up front: `(c.state.exams ||= {})[s] = x` on a $state proxy writes to a plain object the proxy never sees.
export function fillState(c: Course) {
  c.state ||= {} as Course['state'];
  for (const k of ['checks', 'done', 'notes', 'quiz', 'code', 'exams', 'stageOpen', 'have'] as const) c.state[k] ||= {};
  c.state.last ||= '';
  return c;
}
export function normalize(c: Course) {
  fillState(c);
  for (const [lid, ms] of Object.entries(c.chats || {})) appState.chats[c.id + '/' + lid] ||= ms;
  delete c.chats;
  return c;
}
// Roadmaps whose last save failed. Their progress stays in memory and goes out again with the next save or flush.
const unsaved = new Set<Course>();
function send(c: Course) {
  const state = $state.snapshot(c.state);
  delete state.portfolio;
  return saving = saving.then(async () => {
    try { await api('PUT', 'api/courses/' + enc(c.id) + '/state', state); unsaved.delete(c); }
    catch (error) { unsaved.add(c); appState.saveError = message(error); }
    if (!unsaved.size) appState.saveError = '';
  });
}
export function save(c = appState.course!) {
  clearTimeout(saveTimer);
  saveFn = () => { saveFn = undefined; saveCourse = undefined; return send(c); };
  saveCourse = c;
  saveTimer = setTimeout(saveFn, 400);
}
let saveCourse: Course | undefined;
/** Sends any waiting or failed progress now. Resolves to false while some progress is still not saved. */
export async function flushSave() {
  clearTimeout(saveTimer);
  const retry = [...unsaved].filter(c => c !== saveCourse);
  void saveFn?.();
  for (const c of retry) void send(c);
  await saving;
  return !unsaved.size;
}
/** Why AI calls would fail before one is made: no service set up, or not signed in to ChatGPT. Empty when fine or not known yet. */
// ChatGPT reasoning effort: value, label, hint. More effort is slower but plans more carefully.
export const EFFORTS = [['low', 'Low effort', 'Fastest answers'], ['medium', 'Medium effort', 'Balanced'], ['high', 'High effort', 'Slowest, most careful']];
export const defaultEffort = () => appState.config.effort || 'medium';
export function aiProblem(cfg: Settings = appState.config): '' | 'none' | 'signed_out' {
  if (cfg.base === undefined) return '';
  if (!cfg.base || !cfg.model) return 'none';
  return cfg.base === 'codex' && cfg.codex === 'signed_out' ? 'signed_out' : '';
}
export async function refreshConfig() {
  return appState.config = await api<Settings>('GET', 'api/settings');
}
/** Opens ChatGPT's sign-in page in the browser and waits until the app is signed in, the sign-in fails, or 10 minutes pass. */
export async function signInChatGpt() {
  const { url } = await api<{ url: string }>('POST', 'api/chatgpt/login');
  await openUrl(url);
  // Pressed again while waiting (the tab was closed): the page opens again and the first call keeps waiting.
  if (appState.signingIn) return;
  appState.signingIn = true;
  try {
    for (;;) {
      await new Promise(r => setTimeout(r, 1500));
      const s = await api<{ waiting: boolean; error: string }>('GET', 'api/chatgpt/status');
      if (!s.waiting) { if (s.error) throw new Error(s.error); break; }
    }
    await refreshConfig();
    void loadModels(true);
  } finally { appState.signingIn = false; }
}
/** For actions that replace the roadmap with the server's copy: they must not run over progress that is not saved. */
export async function saveFirst() {
  if (!await flushSave()) throw new Error('Your progress is not saved yet, so nothing was changed. Try again in a moment.');
}
export async function loadLibrary() {
  const version = appState.routeVersion;
  const [list, cfg, review, activity] = await Promise.all([
    api<LibraryCourse[]>('GET', 'api/courses'), api<Settings>('GET', 'api/settings'), api<ReviewData>('GET', 'api/review'), api<Activity>('GET', 'api/activity'),
  ]);
  if (version !== appState.routeVersion) return;
  appState.library = list; appState.config = cfg; appState.review = review; appState.activity = activity; appState.screen = 'library';
  // ChatGPT model names come from the server, so load them early: the tutor then shows "GPT-5.6 Terra", not the raw id.
  if (cfg.base === 'codex') void loadModels();
}
export async function route() {
  const version = ++appState.routeVersion;
  appState.screen = 'loading';
  appState.current = null;
  await flushSave();
  if (version !== appState.routeVersion) return;
  appState.error = '';
  try {
    const [, a, rawId, rawLid, rawX] = location.hash.split('/');
    if (a === 'c' && rawId) {
      const id = decodeURIComponent(rawId);
      if (appState.course?.id !== id) {
        const c = await api<Course>('GET', 'api/courses/' + enc(id));
        if (version !== appState.routeVersion) return;
        appState.course = normalize(c);
      }
      if (rawLid === 'exam') { appState.examStage = Number(rawX); appState.screen = 'exam'; }
      else if (rawLid) { appState.current = lessons(appState.course!).find(l => l.id === decodeURIComponent(rawLid)) || null; appState.screen = 'lesson'; }
      else appState.screen = 'overview';
      return;
    }
    appState.course = null;
    if (a === 'settings' || a === 'review' || a === 'mistakes' || a === 'search' || a === 'import' || a === 'progress') appState.screen = a;
    else await loadLibrary();
  } catch (error) {
    if (version !== appState.routeVersion) return;
    appState.course = null; appState.error = message(error); appState.screen = 'error';
  }
}
export async function openIdeas(keep = false) {
  if (appState.ideasOpen) { if (!keep) appState.ideasOpen = false; return; }
  appState.ideasOpen = true;
  if (!appState.ideas) try { appState.ideas = (await api<{ notes: string[] }>('GET', 'api/ideas')).notes; }
  catch (error) { toast(message(error)); appState.ideasOpen = false; }
}
export function saveIdeas() {
  clearTimeout(ideasTimer);
  ideasTimer = setTimeout(() => { void flushIdeas(); }, 400);
}
export async function flushIdeas() {
  clearTimeout(ideasTimer); ideasTimer = undefined;
  try { if (appState.ideas) await api('PUT', 'api/ideas', { notes: $state.snapshot(appState.ideas) }); }
  catch (error) { toast("Couldn't save your ideas: " + message(error)); }
}
export async function applyPlan(plan: object, ask = '') {
  const c = appState.course;
  if (!c) throw new Error('No roadmap open.');
  await saveFirst();
  const r = await api<{ course: Course; change: { note: string } }>('POST', 'api/courses/' + enc(c.id) + '/apply', { plan, ...(ask ? { ask } : {}) });
  const id = appState.current?.id;
  fillState(Object.assign(c, r.course));
  if (id) {
    appState.current = lessons(c).find(l => l.id === id) || null;
    if (!appState.current) location.hash = href(c.id);
  }
  toast(r.change.note || 'Roadmap updated.');
}
export async function adaptNow(loud: boolean) {
  if (appState.adapting) return false;
  const c = appState.course; if (!c) return false;
  appState.adapting = true;
  try {
    await saveFirst();
    const preview = await api<PlanPreview>('POST', 'api/courses/' + enc(c.id) + '/adapt');
    if (appState.course === c) {
      appState.suggestion = {cid:c.id, preview};
      if (loud) location.hash = href(c.id);
      if (!loud && preview.options.length) toast('A suggested adjustment is ready on your roadmap.');
    }
    return true;
  } catch (error) { if (loud) toast(message(error)); return false; }
  finally { appState.adapting = false; }
}
export function expandPhase(c: Course, lid: string) {
  const key = c.id + '/' + c.phases.findIndex(p => p.lessons.some(l => l.id === lid));
  if (!expanding.has(key)) expanding.set(key, api<{ phases: Course['phases'] }>('POST', 'api/courses/' + enc(c.id) + '/expand', { lesson: lid }).then(r => {
    c.phases = r.phases;
    if (appState.course === c && appState.current) appState.current = lessons(c).find(l => l.id === appState.current!.id) || appState.current;
  }).finally(() => { expanding.delete(key); }));
  return expanding.get(key)!;
}
export async function ask(text: string) {
  text = text.trim(); const c = appState.course, l = appState.current;
  if (!text || appState.busyKey || !c || !l) return false;
  const key = keyFor(c, l);
  appState.chats[key] ||= [];
  const msgs = appState.chats[key];
  // A failed question with no reply is not sent again as history: two questions in a row would confuse the model.
  if (appState.tutorErrors[key]) { if (msgs[msgs.length - 1]?.role === 'user') msgs.pop(); delete appState.tutorErrors[key]; }
  msgs.push({ role: 'user', content: text });
  // The model's latest thinking summary shows as the status line until the next tool step replaces it.
  const thinking = watchThinking(t => { if (appState.busyKey === key && !stopped) appState.tutorStatus = t; });
  const body = { course: c.id, lesson: l.id, messages: $state.snapshot(msgs), stream: true, thinking: thinking.key, effort: appState.tutorEffort || defaultEffort(), ...(appState.openVideo ? { video: appState.openVideo } : {}) };
  appState.busyKey = key; appState.tutorStatus = 'Thinking'; stopped = false;
  msgs.push({ role: 'assistant', content: '' });
  const m = msgs[msgs.length - 1];
  try {
    const full = await chatStream(body, piece => { m.content += piece; }, status => { if (status.done) { m.steps ||= []; m.steps.push(status.text); } else appState.tutorStatus = status.text; });
    if (!m.content) m.content = full || '(No answer.)';
    return true;
  } catch (error) {
    // A stopped reply keeps what had arrived; with nothing arrived the question goes back to the box.
    if (stopped) { if (m.content) return true; msgs.pop(); msgs.pop(); return false; }
    // A failed reply keeps the question (and any part of the answer) in the log, with the error under it.
    if (m.content) m.content += '\n\n(The answer was cut off.)'; else msgs.pop();
    appState.tutorErrors[key] = { text, error: message(error) };
    return true;
  } finally { thinking.stop(); appState.busyKey = ''; appState.tutorStatus = ''; }
}
/** Sends the question whose reply failed again, in place of the failed turn. */
export function retryAsk() {
  const c = appState.course, l = appState.current;
  if (!c || !l || appState.busyKey) return Promise.resolve(false);
  const key = keyFor(c, l), failed = appState.tutorErrors[key], msgs = appState.chats[key];
  if (!failed) return Promise.resolve(false);
  if (msgs && msgs[msgs.length - 1]?.role === 'assistant') msgs.pop();
  return ask(failed.text);
}
let stopped = false;
/** Stops the tutor reply that is on its way. */
export function stopAsk() {
  if (!appState.busyKey || stopped) return;
  stopped = true; appState.tutorStatus = 'Stopping';
  void api('POST', 'api/chat/stop').catch(() => {});
}
/** Empties this lesson's conversation, on screen and on disk. */
export async function clearChat(c: Course, l: Lesson) {
  if (appState.busyKey) return;
  try {
    await api('DELETE', 'api/courses/' + enc(c.id) + '/chat', { lesson: l.id });
    appState.chats[keyFor(c, l)] = []; delete appState.tutorErrors[keyFor(c, l)];
  } catch (error) { toast(message(error)); }
}
export async function loadPractice(fresh = false) {
  const c = appState.course!, l = appState.current!, key = keyFor(c, l);
  if (appState.practiceBusy[key]) return;
  appState.practiceBusy[key] = true; appState.practiceFresh[key] = fresh; delete appState.practiceErrors[key];
  try {
    l.practice = await api<NonNullable<Lesson['practice']>>('POST', 'api/courses/' + enc(c.id) + '/practice', { lesson: l.id, ...(fresh ? { fresh: true } : {}) });
    delete appState.sessions[key]; appState.practiceViews[key] = 'quiz';
    if (fresh) {
      delete c.state.quiz[l.id];
      if (appState.course === c) save(c);
      else { const state = $state.snapshot(c.state); delete state.portfolio; await api('PUT', 'api/courses/' + enc(c.id) + '/state', state); }
    }
  } catch (error) { appState.practiceErrors[key] = message(error); }
  finally { delete appState.practiceBusy[key]; }
}
export async function runCode(test: boolean) {
  const c = appState.course!, l = appState.current!, key = keyFor(c, l);
  if (appState.runs[key]?.running || appState.runs[key]?.installing) return;
  const code = c.state.code[l.id] ?? l.practice!.exercise!.starter;
  c.state.code[l.id] = code; save(c); appState.runs[key] = { running: true, test, code };
  try {
    const r = await api<RunState>('POST', 'api/run', { course: c.id, lesson: l.id, code, test });
    appState.runs[key] = { ...r, test, code };
    if (test && !r.missing) appState.runFails[key] = r.ok ? 0 : (appState.runFails[key] ?? c.state.tests?.[l.id]?.runs ?? 0) + 1;
  } catch (error) { appState.runs[key] = { error: true, output: message(error), test, code }; }
}
export async function installLanguage() {
  const c = appState.course!, l = appState.current!, key = keyFor(c, l), run = appState.runs[key];
  if (!run?.missing || run.installing) return;
  run.installing = true; delete run.installError;
  try {
    const installed = await api<RunState>('POST', 'api/install', { lang: run.missing });
    if (!installed.ok) { run.installError = installed.output || 'Installation did not finish.'; return; }
    appState.runs[key] = { running: true, test: run.test, code: run.code };
    try {
      const result = await api<RunState>('POST', 'api/run', { course: c.id, lesson: l.id, code: run.code, test: run.test });
      appState.runs[key] = { ...result, test: run.test, code: run.code };
    } catch (error) { appState.runs[key] = { error: true, output: message(error), test: run.test, code: run.code }; }
  } catch (error) { run.installError = message(error); }
  finally { run.installing = false; }
}
export function finishLesson() {
  const c = appState.course!, l = appState.current!;
  if (c.state.done[l.id]) delete c.state.done[l.id];
  else if (checked(c, l).every(Boolean) || passed(c, l)) c.state.done[l.id] = true;
  else return '';
  save(c);
  if (!c.state.done[l.id]) return '';
  const achievement = lessons(c).every(x => c.state.done[x.id]) ? 'Roadmap complete' : c.phases.find(p => p.lessons.includes(l))!.lessons.every(x => c.state.done[x.id]) ? 'Phase complete' : 'Lesson complete';
  if (achievement === 'Phase complete') adaptTimer = setTimeout(async () => {
    if (appState.course !== c) return;
    await adaptNow(false);
    const upcoming = lessons(c).find(x => !c.state.done[x.id]);
    if (upcoming?.stub) expandPhase(c, upcoming.id).catch(() => {});
  }, 1200);
  return achievement;
}
export async function dispose() {
  clearTimeout(toastTimer); clearTimeout(adaptTimer);
  await flushSave();
  if (ideasTimer) await flushIdeas();
}
export function goCourse(c: Course) { appState.course = normalize(c); location.hash = href(c.id); }
