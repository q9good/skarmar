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
export type FollowUp = {
  id: string; version: number; source_session_id: string; content: string;
  kind: 'general' | 'task' | null; status: string;
  next_review_date: string | null; conclusion: string;
};
export type AppState = { goals: Goal[]; sessions: Session[]; difficulties: FollowUp[]; experiences: FollowUp[] };
export type SaveRequest = {
  operation_id: string; expected_version: number | null;
  action: 'save_draft' | 'complete'; session: SessionInput;
};
export type SaveResult = { session: Session; difficulty_id: string | null; experience_id: string | null };

const base = (process.env.EXPO_PUBLIC_API_URL ?? '').replace(/\/$/, '');
export class ApiError extends Error {
  constructor(message: string, public status: number) { super(message); }
}
export async function request<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const response = await fetch(`${base}/api${path}`, {
    method, headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const data = await response.json();
  if (!response.ok) throw new ApiError(data.error ?? '操作未完成，请保留草稿后重试', response.status);
  return data as T;
}
