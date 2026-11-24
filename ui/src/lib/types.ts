export type Theme = 'light' | 'dark' | 'system';
export type Tally = { in?: number; cached?: number; out?: number; calls?: number };
export interface Usage extends Tally { since?: number; models?: Record<string, Tally & { service?: string }>; days?: Record<string, Record<string, Tally>>; roadmaps?: Record<string, { title?: string; create?: Record<string, Tally>; tutor?: Record<string, Tally>; use?: Record<string, Tally> }> }
export interface VideoPrefs { style: string; length: string; lang: string; recent: string }
export interface PlanOption { label: string; note: string; added: { title: string; goal: string; after?: number; phase?: boolean; lessons?: string[] }[]; dropped: string[]; changed: { from: string; title: string; goal: string }[]; plan: object }
export interface PlanPreview { options: PlanOption[]; note: string }
/** codex: only when the AI service is ChatGPT (base "codex"). ready = this app is signed in; signed_out = press Sign in with ChatGPT.
 *  damaged: how many roadmap files could not be read and are left out of the library (kept on disk untouched). */
export interface Settings { videos: VideoPrefs; base: string; model: string; small?: string; key_set?: boolean; theme: Theme; usage?: Usage; prices?: Record<string, number[]>; codex?: 'ready' | 'signed_out'; chatgpt_email?: string; opencode?: boolean; claude?: boolean; claude_auth?: 'ready' | 'signed_out'; claude_email?: string; ollama?: boolean; providers?: Record<string, { model: string; small: string; key_set: boolean }>; effort?: string; damaged: number; goal?: number; os?: string; unsandboxed?: boolean }
/** steps: what the tutor looked up before this reply, in the order it did so. */
export interface Message { role: 'user' | 'assistant'; content: string; steps?: string[] }
/** exam: a stage exam question kept in its lesson's bank; the lesson quiz leaves it out. */
export interface Quiz { q: string; options: string[]; answer: number; why: string; exam?: boolean }
export interface Exercise { title: string; prompt: string; lang: string; starter: string; tests: string }
export interface Video { id: string; title: string; name?: string; channel: string; duration?: number; why?: string; start?: number; action?: string; reference?: boolean; segments?: { start: number; end: number; title: string }[]; covers?: string[]; searched?: boolean; chapters?: { t: number; title: string }[] }
export interface Chart { title: string; kind: 'line' | 'bar'; x: string; y: string; labels: string[]; series: { name: string; values: number[] }[]; note: string }
export interface Lesson { checkpoint?: {task:string;criteria:string;preparation:string}; id: string; title: string; goal: string; query: string; concepts?: string[]; known?: string[]; tasks?: string[]; done?: string[]; lookups?: string[]; files?: string[]; warn?: string; charts?: Chart[]; stub?: boolean; added?: boolean; videos?: Video[]; novideos?: boolean; practice?: { quiz?: Quiz[]; from?: number; recall?: string[]; exercise?: Exercise }; cards?: Card[] }
export interface Phase { title: string; blurb: string; stage?: number; lessons: Lesson[] }
export interface Score { score: number; total: number }
export interface Blueprint { challenge: string; criteria: string; milestones: string; diagnostic: string }
export interface Attempt { mode: 'practice' | 'independent'; task: string; work: string; evidence: string; improvement: string; next: string }
export interface Portfolio { attempts: Attempt[]; challenge?: string; version: number }
export interface CourseState { portfolio?: Record<string, Portfolio>; checks: Record<string, boolean>; done: Record<string, boolean>; notes: Record<string, string>; quiz: Record<string, Score>; code: Record<string, string>; last: string; exams?: Record<number, Score>; stageOpen?: Record<number, boolean>; have?: Record<number, boolean>; tests?: Record<string, { ok: boolean; runs: number }> }
export interface Walkthrough {
  date: string; outlines: number; additional: string; perspective: string;
  findings: { lesson: string; title: string; quote: string; difficulty: string; change: string }[];
}
export interface Course { walkthrough?: Walkthrough; blueprint?: Blueprint; prefs?: VideoPrefs; id: string; title: string; summary: string; prompt: string; phases: Phase[]; state: CourseState; stages?: { title: string; blurb?: string }[]; folder?: string; work?: string; changes?: { changed?: string[]; note: string; added: string[]; dropped: string[] }[]; needs?: { name: string; why?: string; kind: string; command?: string; stage?: number }[]; chats?: Record<string, Message[]>; offline?: boolean; saved_videos?: string[] }
export interface LibraryCourse { id: string; title: string; summary: string; total: number; done: number; paused?: boolean; project?: boolean; next?: { id: string; title: string } }
export interface ReviewItem extends Quiz { course: string; lesson: string; i: string | number; roadmap: string; lesson_title: string; kind?: string; title?: string; prompt?: string; starter?: string; back?: string }
export interface ReviewData { count: number; questions: ReviewItem[] }
/** picks: the option chosen for each question so far (-1 when time ran out), for the answer review at the end. */
export interface Session { index: number; score: number; selected: number | null; finished: boolean; total?: number; pending?: boolean; error?: string; text?: string; picks?: number[]; result?: { right: boolean; feedback?: string; points?: { text: string; hit: boolean }[]; output?: string; missing?: string } | null }
export interface RunState { running?: boolean; installing?: boolean; test?: boolean; code?: string; missing?: string; error?: boolean; ok?: boolean; output?: string; installError?: string }
export interface Mistake extends ReviewItem { fixed?: boolean; n: number; picked: number }
/** A review card made from the learner's notes; it comes back in the daily review. */
export interface Card { front: string; back: string }
/** days: the last 35 days, oldest first; `day` counts days since 1970 on the learner's calendar, like `today`. */
export interface Activity { today: number; goal: number; done: number; streak: number; best: number; first?: number | null; total?: number; days: { day: number; n: number }[] }
export interface Stats { activity: Activity; weeks: { week: number; right: number; asked: number }[]; year: { day: number; s: number; n: number }[]; roadmaps: { id: string; title: string; done: number; total: number; paused: boolean; secs: number }[]; weak: { course: string; roadmap: string; lesson: string; title: string; score: number; total: number }[]; lessons_done: number; mistakes_open: number }
export interface OfflineStatus { on: boolean; saved: number; total: number; bytes: number; running: boolean; failed: number; error: string }
export interface WorkCheck { verdict: 'done' | 'partly' | 'not_yet'; summary: string; next: string; items: { task: string; met: boolean; note: string }[] }
/** One search match: a lesson, a note, a tutor chat message or an idea (ideas have no course or lesson). */
export interface SearchHit { kind: string; snippet: string; course: string; lesson: string; title: string; roadmap: string }
