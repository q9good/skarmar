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
};
export type Session = SessionInput & {
  id: string; version: number; status: 'draft' | 'completed';
  target_snapshots: (Target & { goal_snapshot: Goal })[];
};
export type LessonInput = {
  business_date: string; course: string; teacher: string; content: string;
  goals: string; problems: string; home_cooperation: string;
};
export type Lesson = LessonInput & { id: string; version: number };
export type FollowUp = {
  id: string; version: number; source_session_id: string | null; content: string;
  kind: 'general' | 'task' | null; status: string;
  next_review_date: string | null; conclusion: string;
  goal_ids: string[]; pathway_ids: string[];
};
export type FollowUpUpdate = {
  expected_version: number; status: string; conclusion: string; next_review_date: string | null;
  content?: string; goal_ids?: string[]; pathway_ids?: string[];
};
export type Pathway = {
  id: string; version: number; title: string; description: string; steps: string;
  status: 'active' | 'archived'; goal_ids: string[];
};
export type AppState = { goals: Goal[]; sessions: Session[]; difficulties: FollowUp[]; experiences: FollowUp[]; pathways: Pathway[]; lessons: Lesson[] };
export type SaveRequest = {
  operation_id: string; expected_version: number | null;
  action: 'save_draft' | 'complete'; session: SessionInput;
};
export type SaveResult = { session: Session; difficulty_id: string | null; experience_id: string | null };

const base = (process.env.EXPO_PUBLIC_API_URL ?? '').replace(/\/$/, '');
export class ApiError extends Error {
  constructor(message: string, public status: number) { super(message); }
}
let unauthorized: (() => void) | undefined;
let sessionEpoch = 0;
export function onUnauthorized(handler?: () => void) { unauthorized = handler; }
export function advanceAuthSession() { sessionEpoch += 1; }
async function fetchResponse(path: string, method = 'GET', body?: unknown, upload?: FormData): Promise<Response> {
  const epoch = sessionEpoch;
  let response: Response;
  try { response = await fetch(`${base}/api${path}`, {
    method, credentials: 'include', cache: 'no-store',
    headers: { 'X-Skarma-Request': '1', ...(body === undefined ? {} : { 'Content-Type': 'application/json' }) },
    body: upload ?? (body === undefined ? undefined : JSON.stringify(body)),
  }); } catch { throw new ApiError('暂时无法连接，请保留草稿后重试', 503); }
  if (response.status === 401 && epoch === sessionEpoch && path !== '/auth/login' && path !== '/auth/me') unauthorized?.();
  if (!response.ok) {
    let data: { error?: string };
    try { data = await response.json(); } catch { throw new ApiError('服务暂时无法响应，请稍后重试', response.status); }
    throw new ApiError(data.error ?? '操作未完成，请保留草稿后重试', response.status);
  }
  return response;
}
export async function request<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const response = await fetchResponse(path, method, body);
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

export async function exportWorkbook(): Promise<Blob> {
  const epoch = sessionEpoch;
  const response = await fetchResponse('/export.xlsx');
  if (response.headers.get('content-type')?.split(';')[0] !== 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet') {
    throw new ApiError('下载未完成，请稍后重试', 502);
  }
  let file: Blob;
  try { file = await response.blob(); } catch { throw new ApiError('下载连接中断，请重新导出', 503); }
  if (epoch !== sessionEpoch) throw new ApiError('账号已变更，请重新导出', 409);
  return file;
}
