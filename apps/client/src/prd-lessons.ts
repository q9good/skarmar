import type { Catalog, Goal, Method } from './foundation';
export type Observation = { helpful_notes?:string;course_started?:boolean; business_date: string; course_id: string; teacher_id: string; notes: string; teacher_notes: string; progress_notes: string; interest_notes: string; difficulty_notes: string; cause_guess: string; request_review: boolean; review_message: string };
export type CatalogSnapshot = { id: string; version: number; kind: 'course'|'teacher'; name: string };
export type Target = { goal_id: string; note: string };
export type Organization = { feedback_ids?:string[]; seen_observation_version: number; targets: { target: Target; goal: Goal; ancestors:Goal[] }[]; methods: Method[]; focus_goals: Goal[]; processed_request_generation: number; state: string; claimed_by: string|null; claimed_name: string|null; outcome: string };
export type Lesson = { id: string; version: number; observation_version: number; organization_version: number; created_at: string; creator_id: string; creator_name: string; status: 'recorded'|'withdrawn'|'deleted'; marked_error: boolean; observation: Observation; course: CatalogSnapshot; teacher: CatalogSnapshot; organization: Organization; request_generation: number; requested_by: string|null; request_name: string|null; request_message: string; acknowledgements: string[] };
export type WithdrawalCapability={allowed:boolean;reason_code:'allowed'|'not_recorded'|'not_creator'|'outside_creation_day'|'referenced';reason:string;lesson_version:number;references:string[]};
export type LessonRevision = { lesson:Lesson; actor_id:string; actor_name:string; recorded_at:string; event:string };
export type NewKnowledge = { id: string; kind: 'experience'|'difficulty'; content: string; goal_ids: string[]; method_ids: string[]; stage: 'idea'|'try'|'helpful'|null; next_review_date: string|null };
export type NewAction = { suggested_date?:string|null; id: string; content: string; goal_ids: string[]; method_ids: string[]; assignee_id: string|null };
export type Knowledge = { details:import('./prd-followups').Details;derived_from:import('./prd-followups').Derivation[];source_event_id:string|null;source_review_id:string|null;deleted:boolean;audit:import('./prd-followups').Audit|null; input: NewKnowledge; version: number; source_lesson_id: string|null; source_session_id: string|null; source_session_version: number|null; source_observation_version: number; source_observation: Observation|null; source_summary: import('./prd-sessions').Summary|null; author_id: string; author_name: string; created_at: string };
export type NextAction = { audit:import('./prd-followups').Audit|null;source_deleted:boolean; source_event_id:string|null;source_review_id:string|null;input: NewAction; version: number; source_lesson_id: string|null; source_session_id: string|null; source_session_version: number|null; source_observation_version: number; author_id: string; author_name: string; assignee_name: string|null; status: string; plan_id:string|null; created_at: string };
export type OrganizationInput = { feedback?:import('./prd-followups').PracticeFeedback[];feedback_knowledge_versions?:Record<string,number>; seen_observation_version: number; targets: Target[]; method_ids: string[]; focus_goal_ids: string[]; processing: 'keep'|'claim'|'take_over'|'complete'|'reopen'; request_generation: number; outcome: string; new_knowledge: NewKnowledge[]; new_actions: NewAction[] };
export type LessonAction = { kind: 'observation'; value: Observation } | { kind: 'organization'; value: OrganizationInput } | { kind: 'withdraw'|'delete'|'acknowledge' } | { kind: 'mark_error'; value: { marked: boolean } };
export type LessonWrite = { id: string; operation_id: string; expected_version: number|null; action: LessonAction };
export type LessonDraft = { request: LessonWrite; phase: 'basic'|'observation'|'organization'|'administration'; source?: Observation; pending?: LessonWrite; backup?: LessonWrite };
// Validate persisted input before rendering it; never replace an unreadable queue.
export function decodeLessonDrafts(raw: string|null): Record<string,LessonDraft> {
  if (raw === null) return {};
  const object = (v:unknown):v is Record<string,unknown> => Boolean(v) && typeof v === 'object' && !Array.isArray(v);
  const strings = (v:unknown):v is string[] => Array.isArray(v) && v.every(s=>typeof s==='string');
  const integer = (v:unknown) => Number.isSafeInteger(v) && Number(v)>0;
  const observation = (v:unknown) => object(v) && (v.helpful_notes===undefined||typeof v.helpful_notes==='string') && (v.course_started===undefined||typeof v.course_started==='boolean') && ['business_date','course_id','teacher_id','notes','teacher_notes','progress_notes','interest_notes','difficulty_notes','cause_guess','review_message'].every(k=>typeof v[k]==='string') && typeof v.request_review==='boolean';
  const bindings = (v:Record<string,unknown>) => typeof v.id==='string' && typeof v.content==='string' && strings(v.goal_ids) && strings(v.method_ids);
  const write = (v:unknown):v is LessonWrite => {
    if (!object(v) || typeof v.id!=='string' || !v.id || typeof v.operation_id!=='string' || !v.operation_id || !(v.expected_version===null || integer(v.expected_version)) || !object(v.action)) return false;
    const a=v.action, p=a.value;
    if (a.kind==='observation') return observation(p);
    if (a.kind==='organization') return object(p) && (p.feedback===undefined||Array.isArray(p.feedback)&&p.feedback.every(f=>object(f)&&['id','knowledge_id','result','observation','attempt','adult_response','conditions','reason','basis','support'].every(k=>typeof f[k]==='string')&&typeof f.comparable==='boolean'&&(f.expected_version===null||integer(f.expected_version))))&&(p.feedback_knowledge_versions===undefined||object(p.feedback_knowledge_versions)&&Object.values(p.feedback_knowledge_versions).every(integer)) && integer(p.seen_observation_version) && Array.isArray(p.targets) && p.targets.every(t=>object(t)&&typeof t.goal_id==='string'&&typeof t.note==='string') && strings(p.method_ids) && strings(p.focus_goal_ids) && ['keep','claim','take_over','complete','reopen'].includes(String(p.processing)) && Number.isSafeInteger(p.request_generation) && Number(p.request_generation)>=0 && typeof p.outcome==='string' && Array.isArray(p.new_knowledge) && p.new_knowledge.every(k=>object(k)&&bindings(k)&&['experience','difficulty'].includes(String(k.kind))&&(k.stage===null||['idea','try','helpful'].includes(String(k.stage)))&&(k.next_review_date===null||typeof k.next_review_date==='string')) && Array.isArray(p.new_actions) && p.new_actions.every(a=>object(a)&&bindings(a)&&(a.assignee_id===null||typeof a.assignee_id==='string'));
    if (a.kind==='mark_error') return object(p) && typeof p.marked==='boolean';
    return ['withdraw','delete','acknowledge'].includes(String(a.kind));
  };
  const value:unknown=JSON.parse(raw);
  if (!object(value) || !Object.entries(value).every(([key,d])=> {
    if (!object(d) || !write(d.request) || !(d.source===undefined||observation(d.source)) || !(d.backup===undefined||(write(d.backup)&&d.backup.id===d.request.id)) || !(d.pending===undefined||(write(d.pending)&&JSON.stringify(d.pending)===JSON.stringify(d.request)))) return false;
    const kind=d.request.action.kind;
    if(kind==='organization'&&!observation(d.source))return false;
    const zone=kind==='organization'?'organization':kind==='observation'?'observation':'administration';
    return key===`${d.request.id}:${zone}` && (zone==='observation'?['basic','observation'].includes(String(d.phase)):d.phase===zone);
  })) throw new Error('无法读取本机课记草稿，请保留浏览器数据后重试。');
  return value as Record<string,LessonDraft>;
}
export const emptyObservation = (today: string): Observation => ({ business_date: today,course_started:false, course_id: '', teacher_id: '', notes: '', teacher_notes: '', progress_notes: '', interest_notes: '', difficulty_notes: '', cause_guess: '', request_review: false, review_message: '' });
export const organizationOf = (l: Lesson): OrganizationInput => ({ seen_observation_version: l.observation_version, targets: l.organization.targets.map(t => t.target), method_ids: l.organization.methods.map(m => m.id), focus_goal_ids: l.organization.focus_goals.map(g => g.id), processing: 'keep', request_generation: l.request_generation, outcome: l.organization.outcome, new_knowledge: [], new_actions: [] });
export const needsReview = (l: Lesson) => l.status === 'recorded' && l.request_generation > 0 && (l.organization.processed_request_generation < l.request_generation || l.organization.state !== 'done');
export const countable = (l: Lesson) => l.status === 'recorded';
export const observationText = (v: Observation) => [v.notes,v.teacher_notes,v.helpful_notes,v.progress_notes,v.interest_notes,v.difficulty_notes,v.cause_guess].filter(Boolean).join('\n');
export function lessonSearchSources(lesson: Lesson, knowledge: Knowledge[], query: string) {
  const needle=query.trim();
  if(!needle)return [];
  const sources:string[]=[];
  const add=(source:string)=>{if(!sources.includes(source))sources.push(source);};
  if(lesson.course.name.includes(needle))add('课程');
  if(lesson.teacher.name.includes(needle))add('老师');
  if(lesson.observation.teacher_notes.includes(needle))add('老师建议');
  if([lesson.observation.notes,lesson.observation.helpful_notes??'',lesson.observation.progress_notes,lesson.observation.interest_notes,lesson.observation.difficulty_notes,lesson.observation.cause_guess].some(value=>value.includes(needle)))add('课堂观察');
  for(const item of knowledge)if(!item.deleted&&item.source_lesson_id===lesson.id&&item.input.content.includes(needle))add(item.input.kind==='difficulty'?'关联困难':'关联经验');
  return sources;
}
export function monthRange(month: string) { const [y,m] = month.split('-').map(Number); return { start: `${month}-01`, end: `${month}-${new Date(Date.UTC(y,m,0)).getUTCDate()}` }; }
export function shiftMonth(month: string, offset: number) { const [y,m] = month.split('-').map(Number); return new Date(Date.UTC(y,m-1+offset,1)).toISOString().slice(0,7); }
export function teacherTotals(lessons: Lesson[], start: string, end: string, today='9999-12-31') {
  return lessonTotals(lessons,start,end,today,'teacher');
}
export function lessonTotals(lessons: Lesson[], start: string, end: string, today: string, by:'teacher'|'course') {
  const map = new Map<string,{ id: string; name: string; count: number; marked: number; records: Lesson[] }>();
  for (const l of lessons) if (countable(l) && l.observation.business_date >= start && l.observation.business_date <= end && l.observation.business_date <= today) {
    const item=l[by];
    const group = map.get(item.id) ?? { id: item.id, name: item.name, count: 0, marked: 0, records: [] };
    group.count++; group.marked += Number(l.marked_error); group.records.push(l); map.set(item.id, group);
  }
  return [...map.values()].sort((a,b) => a.name.localeCompare(b.name,'zh-CN'));
}
function catalogRanks(lessons: Lesson[], catalog: Catalog[], kind: 'course'|'teacher', today: string, relatedId: string) {
  const end = Date.parse(today+'T00:00:00Z'), start = end - 55*86400000;
  const weekday = new Date(end).getUTCDay();
  const relatedKind = kind === 'course' ? 'teacher' : 'course';
  const history = lessons.filter(l => countable(l) && Date.parse(l.observation.business_date+'T00:00:00Z') <= end);
  return catalog.filter(c => c.input.kind === kind && c.input.active).map(c => {
    const all = history.filter(l => l[kind].id === c.id);
    const related = relatedId ? all.filter(l => l[relatedKind].id === relatedId) : [];
    const relatedRecent = related.map(l => l.observation.business_date).sort().at(-1) ?? '';
    const records = all.filter(l => Date.parse(l.observation.business_date+'T00:00:00Z') >= start);
    const recent = records.map(l => l.observation.business_date).sort().at(-1) ?? '';
    const same = kind === 'course' ? records.filter(l => new Date(l.observation.business_date+'T00:00:00Z').getUTCDay() === weekday).map(l => l.observation.business_date).sort().at(-1) ?? '' : '';
    return { c, count: records.length, recent, same, relatedCount: related.length, relatedRecent };
  }).sort((a,b) => b.relatedCount-a.relatedCount || b.relatedRecent.localeCompare(a.relatedRecent) || b.same.localeCompare(a.same) || b.count-a.count || b.recent.localeCompare(a.recent) || a.c.id.localeCompare(b.c.id));
}
/** All active options remain available. Pairing suggests order, never a selection. */
export function rankedCatalogChoices(lessons: Lesson[], catalog: Catalog[], kind: 'course'|'teacher', today: string, relatedId = '') {
  return catalogRanks(lessons,catalog,kind,today,relatedId).map(r => r.c);
}
export function commonChoices(lessons: Lesson[], catalog: Catalog[], kind: 'course'|'teacher', today: string, relatedId = '') {
  return catalogRanks(lessons,catalog,kind,today,relatedId).filter(r => r.count > 0 || r.relatedCount > 0).slice(0,4).map(r => r.c);
}
