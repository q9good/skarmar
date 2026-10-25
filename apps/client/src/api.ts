export type Level = 'long' | 'medium' | 'short';
export type Role = 'primary' | 'secondary' | 'review';
export type Outcome = 'continue' | 'archive' | 'adjust';
export type GoalProgress = 'cultivating' | 'internalized' | 'not_started' | 'in_progress' | 'achieved' | 'completed' | 'overdue';
export type Goal = {
  id: string; title: string; level: Level; parent_id: string | null;
  area: string; criteria: string; status: 'active' | 'archived'; version: number;
  progress_status: GoalProgress | null; start_date: string | null; due_date: string | null;
  last_review_date: string | null; next_review_date: string | null; completion_date: string | null;
  review_notes: string;
};
export type GoalUpdate = Omit<Goal, 'id' | 'level' | 'version'> & { expected_version: number; change_note: string };
export type GoalRevision = { goal: Goal; recorded_at: string; change_note: string; source_session_id: string | null };
export const isReviewable = (goal: Goal) => goal.level !== 'long' &&
  (goal.status === 'archived' || goal.progress_status === 'achieved' || goal.progress_status === 'completed');
export const isTrainable = (goal: Goal) => goal.level !== 'long' && goal.status === 'active' && !isReviewable(goal);
export type Target = {
  goal_id: string; role: Role; plan: string; progress: string;
  outcome: Outcome | null; next_step: string;
};
export type SessionInput = {
  business_date: string; time_slot: string; activities: string[]; organization: string;
  plan: string; observation: string; targets: Target[]; has_difficulty: boolean;
  difficulty: string; has_experience: boolean; experience: string; experience_kind: string;
  difficulty_goal_ids?: string[]; difficulty_pathway_ids?: string[];
  experience_goal_ids?: string[]; experience_pathway_ids?: string[];
};
export type Session = SessionInput & {
  id: string; version: number; status: 'draft' | 'completed';
  target_snapshots: (Target & { goal_snapshot: Goal })[];
};
export type LessonInput = {
  business_date: string; course: string; teacher: string; content: string;
  goals: string; problems: string; home_cooperation: string;
  links?: LessonLinks;
};
export type LessonTarget = Pick<Target, 'goal_id' | 'role' | 'plan' | 'progress'>;
export type LessonLinks = { targets: LessonTarget[]; pathway_ids: string[]; track_problem: boolean; problem_goal_ids: string[]; problem_pathway_ids: string[] };
export const emptyLessonLinks = (): LessonLinks => ({ targets: [], pathway_ids: [], track_problem: false, problem_goal_ids: [], problem_pathway_ids: [] });
export type Lesson = LessonInput & { id: string; version: number; target_snapshots?: { goal_id: string; goal_snapshot: Goal }[] };
export type FollowUp = {
  id: string; version: number; source_session_id: string | null; content: string;
  source_lesson_id?: string | null;
  kind: 'general' | 'task' | null; status: string;
  next_review_date: string | null; conclusion: string;
  goal_ids: string[]; pathway_ids: string[];
  source_deleted?: boolean;
};
export type FollowUpUpdate = {
  expected_version: number; status: string; conclusion: string; next_review_date: string | null;
  content?: string; goal_ids?: string[]; pathway_ids?: string[];
};
export type Pathway = {
  id: string; version: number; title: string; description: string; steps: string;
  status: 'active' | 'archived'; goal_ids: string[];
};
export type LessonOption = { kind: 'course' | 'teacher'; value: string; active: boolean; version: number; renamed_to?: string | null };
export type NewLessonOption = Pick<LessonOption, 'kind' | 'value'>;
export type AppState = { goals: Goal[]; sessions: Session[]; difficulties: FollowUp[]; experiences: FollowUp[]; pathways: Pathway[]; lessons: Lesson[]; lesson_options?: LessonOption[]; deleted?: { kind: string; id: string }[] };
export type SessionRevisionInput = { note: string; difficulty_version: number | null; experience_version: number | null };
export type SessionRevision = { session: Session; recorded_at: string; note: string };
export type SaveRequest = {
  operation_id: string; expected_version: number | null;
  action: 'save_draft' | 'complete' | 'revise'; session: SessionInput;
  revision?: SessionRevisionInput;
};
export type SaveResult = { session: Session; difficulty_id: string | null; experience_id: string | null };

const base = (process.env.EXPO_PUBLIC_API_URL ?? '').replace(/\/$/, '');
export class ApiError extends Error {
  constructor(message: string, public status: number) { super(message); }
}
let unauthorized: ((notice?:string) => void) | undefined;
let sessionEpoch = 0;
export function onUnauthorized(handler?: (notice?:string) => void) { unauthorized = handler; }
type RequestIdentity={actor:string;space:string};
let requestIdentity:RequestIdentity|null=null;
export const AUTH_SCOPE_CHANGED='登录账号与当前填写不一致，请重新登录原账号后继续。';
export function bindRequestIdentity(value:RequestIdentity|null){requestIdentity=value;}
export function advanceAuthSession() { sessionEpoch += 1; requestIdentity=null; }
/** Capture before awaiting local storage. An old async form must never resume
 * using the identity of a later login, even though request() binds at send time. */
export function authSessionGuard():()=>void {
  const epoch=sessionEpoch;
  return ()=>{if(epoch!==sessionEpoch)throw new ApiError('登录状态已更新，原填写仍保留，请重新确认后提交。',409);};
}
export type RequestOptions = { signal?:AbortSignal; space?:string };
async function fetchResponse(path: string, method = 'GET', body?: unknown, upload?: FormData, options?:RequestOptions): Promise<Response> {
  const epoch = sessionEpoch;
  const identity=!["/auth/login","/auth/register","/auth/me"].includes(path)?requestIdentity:null;
  let response: Response;
  try { response = await fetch(`${base}/api${path}`, {
    method, credentials: 'include', cache: 'no-store', signal:options?.signal,
    headers: { 'X-Skarma-Request': '1', ...(identity?{'X-Skarma-Actor':identity.actor,'X-Skarma-Space':identity.space}:{}), ...(path==='/data-transfer'?{'X-Skarma-Transfer':'skarmar.transfer.v1'}:{}), ...(options?.space?{'X-Skarma-Space':options.space}:{}), ...(path.startsWith('/prd/') ? { 'X-Skarma-Contract': 'skarmar.prd.v1' } : {}), ...(body === undefined ? {} : { 'Content-Type': 'application/json' }) },
    body: upload ?? (body === undefined ? undefined : JSON.stringify(body)),
  }); } catch { throw new ApiError('暂时无法连接，请保留草稿后重试', 503); }
  if (response.status === 401 && epoch === sessionEpoch && path !== '/auth/login' && path !== '/auth/me') unauthorized?.();
  if (!response.ok) {
    let data: { error?: string };
    try { data = await response.json(); } catch { throw new ApiError('服务暂时无法响应，请稍后重试', response.status); }
    if(response.status===409&&data.error===AUTH_SCOPE_CHANGED&&epoch===sessionEpoch)unauthorized?.(AUTH_SCOPE_CHANGED);
    throw new ApiError(data.error ?? '操作未完成，请保留草稿后重试', response.status);
  }
  return response;
}
export async function request<T>(path: string, method = 'GET', body?: unknown, options?:RequestOptions): Promise<T> {
  const response = await fetchResponse(path, method, body, undefined, options);
  let data: unknown;
  try { data = await response.json(); } catch { throw new ApiError('服务暂时无法响应，请稍后重试', 502); }
  return data as T;
}

export async function uploadFile<T>(path: string, file: File): Promise<T> {
  const form = new FormData();
  form.append('file', file, file.name);
  const response = await fetchResponse(path, 'POST', undefined, form);
  try { return await response.json() as T; }
  catch { throw new ApiError('预览结果暂时无法读取，请重新选择文件', 502); }
}

export async function exportWorkbook(model:'legacy'|'prd_v1'='legacy'): Promise<Blob> {
  const epoch = sessionEpoch;
  const response = await fetchResponse(model==='prd_v1'?'/prd/export.xlsx':'/export.xlsx');
  if (response.headers.get('content-type')?.split(';')[0] !== 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet') {
    throw new ApiError('下载未完成，请稍后重试', 502);
  }
  let file: Blob;
  try { file = await response.blob(); } catch { throw new ApiError('下载连接中断，请重新导出', 503); }
  if (epoch !== sessionEpoch) throw new ApiError('账号已变更，请重新导出', 409);
  return file;
}
