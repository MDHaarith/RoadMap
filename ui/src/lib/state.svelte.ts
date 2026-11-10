import { api, apiError, chatStream } from './api';
import { enc, href } from './format';
import { lessons, checked, passed } from './progress';
import type { Course, Lesson, LibraryCourse, Settings, ReviewData, Message, Session, RunState } from './types';

export const appState = $state({
  course: null as Course | null,
  current: null as Lesson | null,
  screen: 'loading', error: '', routeVersion: 0,
  library: [] as LibraryCourse[], config: {} as Settings,
  review: { count: 0, questions: [] } as ReviewData,
  toast: '', ideasOpen: false, ideas: null as string[] | null,
  busyKey: '', adapting: false,
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
export function normalize(c: Course) {
  c.state ||= {} as Course['state'];
  for (const k of ['checks', 'done', 'notes', 'quiz', 'code'] as const) c.state[k] ||= {};
  c.state.last ||= '';
  for (const [lid, ms] of Object.entries(c.chats || {})) appState.chats[c.id + '/' + lid] ||= ms;
  delete c.chats;
  return c;
}
export function save(c = appState.course!) {
  clearTimeout(saveTimer);
  saveFn = async () => {
    saveFn = undefined;
    try { await api('PUT', 'api/courses/' + enc(c.id) + '/state', $state.snapshot(c.state)); }
    catch (error) { toast("Couldn't save your progress: " + message(error)); }
  };
  saveTimer = setTimeout(saveFn, 400);
}
export async function flushSave() {
  clearTimeout(saveTimer);
  if (saveFn) await saveFn();
}
export async function loadLibrary() {
  const version = appState.routeVersion;
  const [list, cfg, review] = await Promise.all([
    api<LibraryCourse[]>('GET', 'api/courses'), api<Settings>('GET', 'api/settings'), api<ReviewData>('GET', 'api/review'),
  ]);
  if (version !== appState.routeVersion) return;
  appState.library = list; appState.config = cfg; appState.review = review; appState.screen = 'library';
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
    if (a === 'settings' || a === 'review' || a === 'mistakes' || a === 'search') appState.screen = a;
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
export async function adaptNow(loud: boolean, ask = '') {
  if (appState.adapting) { if (ask) toast('Your roadmap is being updated. Try again in a moment.'); return false; }
  const c = appState.course; if (!c) return false;
  appState.adapting = true;
  try {
    const r = await api<{ course: Course; change: { added: string[]; dropped: string[]; note: string } }>('POST', 'api/courses/' + enc(c.id) + (ask ? '/revise' : '/adapt'), ask ? { ask } : undefined);
    const changed = r.change.added.length || r.change.dropped.length;
    if (changed) {
      c.phases = r.course.phases; c.changes = r.course.changes; c.prompt = r.course.prompt;
      if (appState.course === c && appState.current) appState.current = lessons(c).find(l => l.id === appState.current!.id) || appState.current;
    }
    if (loud || changed) toast((changed ? 'Roadmap updated. ' : '') + (r.change.note || 'No changes needed.'));
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
  const msgs = appState.chats[key]; msgs.push({ role: 'user', content: text });
  const body = { course: c.id, lesson: l.id, messages: $state.snapshot(msgs), stream: true };
  appState.busyKey = key;
  msgs.push({ role: 'assistant', content: '' });
  const m = msgs[msgs.length - 1];
  try {
    const full = await chatStream(body, piece => { m.content += piece; });
    if (!m.content) m.content = full || '(No answer.)';
    return true;
  } catch (error) {
    if (m.content) m.content += '\n\n(The answer was cut off.)';
    else { msgs.pop(); msgs.pop(); toast(message(error)); }
    return false;
  } finally { appState.busyKey = ''; }
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
      else await api('PUT', 'api/courses/' + enc(c.id) + '/state', $state.snapshot(c.state));
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
