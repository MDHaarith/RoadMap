export type Theme = 'light' | 'dark' | 'system';
export interface Usage { in?: number; out?: number; calls?: number; since?: number }
export interface VideoPrefs { style: string; length: string; lang: string; recent: string }
export interface PlanOption { label: string; note: string; added: { title: string; goal: string; after?: number; phase?: boolean; lessons?: string[] }[]; dropped: string[]; changed: { from: string; title: string; goal: string }[]; plan: object }
export interface PlanPreview { options: PlanOption[]; note: string }
/** codex: only when the AI service is Codex. ready = installed and signed in; signed_out = run `codex login` in a terminal; missing = not installed.
 *  damaged: how many roadmap files could not be read and are left out of the library (kept on disk untouched). */
export interface Settings { videos: VideoPrefs; base: string; model: string; small?: string; key_set?: boolean; theme: Theme; usage?: Usage; codex?: 'ready' | 'signed_out' | 'missing'; damaged: number }
/** steps: what the tutor looked up before this reply, in the order it did so. */
export interface Message { role: 'user' | 'assistant'; content: string; steps?: string[] }
/** exam: a stage exam question kept in its lesson's bank; the lesson quiz leaves it out. */
export interface Quiz { q: string; options: string[]; answer: number; why: string; exam?: boolean }
export interface Exercise { title: string; prompt: string; lang: string; starter: string; tests: string }
export interface Video { id: string; title: string; channel: string; duration?: number; why?: string; start?: number; covers?: string[]; searched?: boolean; chapters?: { t: number; title: string }[] }
export interface Chart { title: string; kind: 'line' | 'bar'; x: string; y: string; labels: string[]; series: { name: string; values: number[] }[]; note: string }
export interface Lesson { id: string; title: string; goal: string; query: string; concepts?: string[]; known?: string[]; tasks?: string[]; done?: string[]; lookups?: string[]; files?: string[]; warn?: string; charts?: Chart[]; stub?: boolean; added?: boolean; videos?: Video[]; novideos?: boolean; practice?: { quiz?: Quiz[]; from?: number; recall?: string[]; exercise?: Exercise } }
export interface Phase { title: string; blurb: string; stage?: number; lessons: Lesson[] }
export interface Score { score: number; total: number }
export interface CourseState { checks: Record<string, boolean>; done: Record<string, boolean>; notes: Record<string, string>; quiz: Record<string, Score>; code: Record<string, string>; last: string; exams?: Record<number, Score>; stageOpen?: Record<number, boolean>; have?: Record<number, boolean>; tests?: Record<string, { ok: boolean; runs: number }> }
export interface Course { prefs?: VideoPrefs; id: string; title: string; summary: string; prompt: string; phases: Phase[]; state: CourseState; stages?: { title: string; blurb?: string }[]; folder?: string; work?: string; changes?: { changed?: string[]; note: string; added: string[]; dropped: string[] }[]; needs?: { name: string; why?: string; kind: string; command?: string }[]; chats?: Record<string, Message[]> }
export interface LibraryCourse { id: string; title: string; summary: string; total: number; done: number; paused?: boolean; project?: boolean; next?: { id: string; title: string } }
export interface ReviewItem extends Quiz { course: string; lesson: string; i: string | number; roadmap: string; lesson_title: string; kind?: string; title?: string; prompt?: string; starter?: string }
export interface ReviewData { count: number; questions: ReviewItem[] }
/** picks: the option chosen for each question so far (-1 when time ran out), for the answer review at the end. */
export interface Session { index: number; score: number; selected: number | null; finished: boolean; total?: number; pending?: boolean; error?: string; text?: string; picks?: number[]; result?: { right: boolean; feedback?: string; points?: { text: string; hit: boolean }[]; output?: string; missing?: string } | null }
export interface RunState { running?: boolean; installing?: boolean; test?: boolean; code?: string; missing?: string; error?: boolean; ok?: boolean; output?: string; installError?: string }
export interface Mistake extends ReviewItem { fixed?: boolean; n: number; picked: number }
