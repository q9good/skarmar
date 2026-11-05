import type {Goal} from './foundation';
import type {Coverage,Counts} from './prd-sessions';
export const overviewStateNames={active:'关注／推进中',maintaining:'维持',achieved:'已达成',paused:'暂停',not_started:'未开始'};
export type OverviewState=keyof typeof overviewStateNames;
export function goalStatusGroups(goals:Goal[]){
 return (Object.keys(overviewStateNames) as OverviewState[]).map(state=>({id:state,label:overviewStateNames[state],goals:goals.filter(g=>(g.input.state??'active')===state)}));
}
export type GoalOverviewRow={goal:Goal;counts:Counts|null};
/** Unknown report rows stay unknown; no plan/draft counts are inferred locally. */
export function goalOverviewRows(goals:Goal[],report:Coverage|null):GoalOverviewRow[]{
 return goals.map(goal=>{const row=report?.goals.find(r=>r.id===goal.id);return {goal,counts:row?(row.linked??(goal.input.level==='long'?row.focus:row.counts)):null};});
}
