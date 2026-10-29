import type {Goal,Method} from './foundation';
export type GoalBranch={goal:Goal;children:GoalBranch[];methods:Method[]};
const rank={long:0,medium:1,short:2};
/** Keep actual parent links and many-to-many method links; never infer bindings. */
export function goalMethodHierarchy(goals:Goal[],methods:Method[]){
  const nodes=new Map(goals.map(goal=>[goal.id,{goal,children:[],methods:[]} as GoalBranch]));
  const roots:GoalBranch[]=[];
  for(const node of nodes.values()){
    const parent=nodes.get(node.goal.input.parent_id??'');
    if(parent&&rank[parent.goal.input.level]<rank[node.goal.input.level])parent.children.push(node);
    else roots.push(node);
    node.methods=methods.filter(m=>m.input.goal_ids.includes(node.goal.id));
  }
  const order=(a:GoalBranch,b:GoalBranch)=>rank[a.goal.input.level]-rank[b.goal.input.level]||a.goal.input.title.localeCompare(b.goal.input.title,'zh-CN')||a.goal.id.localeCompare(b.goal.id);
  const sort=(rows:GoalBranch[])=>{rows.sort(order);rows.forEach(row=>sort(row.children));};sort(roots);
  return {roots,unlinked:methods.filter(m=>!m.input.goal_ids.some(id=>nodes.has(id)))};
}
