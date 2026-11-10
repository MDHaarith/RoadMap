import type { Course, Lesson } from './types';

export const lessons = (c: Course) => c.phases.flatMap(p => p.lessons);
export const checked = (c: Course, l: Lesson) => (l.done || []).map((_, i) => !!c.state.checks[l.id + '#' + i]);
export const passed = (c: Course, l: Lesson) => {
  const q = c.state.quiz[l.id];
  return !!q && q.total >= 3 && q.score / q.total >= .8;
};
export const status = (c: Course, l: Lesson) => c.state.done[l.id] ? 'done' : checked(c, l).some(Boolean) ? 'part' : '';
export const stageLessons = (c: Course, s: number) => c.phases.filter(p => p.stage === s).flatMap(p => p.lessons);
export const examPool = (c: Course, s: number) => stageLessons(c, s).flatMap(l => (l.practice?.quiz || []).map((q, i) => ({ ...q, lesson: l.id, i })));
export const examPassed = (c: Course, s: number) => { const x = c.state.exams?.[s]; return !!x && x.score / x.total >= .7; };
export const examReady = (c: Course, s: number) => stageLessons(c, s).every(l => c.state.done[l.id]) && examPool(c, s).length >= 5;
export function locked(c: Course, l: Lesson) {
  if (!c.stages || c.state.done[l.id] || checked(c, l).some(Boolean)) return false;
  const s = c.phases.find(p => p.lessons.includes(l))?.stage;
  if (s === undefined || s <= 0 || examPassed(c, s - 1)) return false;
  return !(stageLessons(c, s - 1).every(x => c.state.done[x.id]) && examPool(c, s - 1).length < 5);
}
export function nextLesson(c: Course) {
  const ls = lessons(c), last = ls.find(l => l.id === c.state.last);
  return last && !c.state.done[last.id] ? last : ls.find(l => !c.state.done[l.id]) || ls[0];
}
export function stageOpen(c: Course, n: number) {
  const next = lessons(c).find(l => !c.state.done[l.id]);
  return c.state.stageOpen?.[n] ?? (next ? stageLessons(c, n).includes(next) : n === (c.stages?.length || 0) - 1);
}
