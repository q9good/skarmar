import {goalMethodHierarchy,type GoalBranch} from './goal-method-hierarchy.ts';
import type {Goal,Method} from './foundation';
export type DiagramNode={key:string;kind:'goal'|'method';id:string;title:string;column:number;x:number;y:number;width:number;height:number;children:number;goal?:Goal;method?:Method};
export type DiagramEdge={from:string;to:string;kind:'parent'|'method'};
/** One node per saved object, including a method used on multiple branches.
 * Layout is local; labels never leave the device for a rendering service. */
export function goalMethodDiagram(goals:Goal[],methods:Method[],expanded:Set<string>,ranks:Map<string,number>,term='',orientation:'horizontal'|'vertical'='horizontal'){
 const tree=goalMethodHierarchy(goals,methods),nodes:DiagramNode[]=[],edges:DiagramEdge[]=[],byId=new Map(goals.map(g=>[g.id,g])),keep=new Set<string>();
 const query=term.trim().toLocaleLowerCase(),matches=(s:string)=>s.toLocaleLowerCase().includes(query);
 const descend=(r:GoalBranch)=>{keep.add(r.goal.id);r.children.forEach(descend);};
 if(query){const visit=(r:GoalBranch)=>{if(matches(r.goal.input.title))descend(r);r.children.forEach(visit);};tree.roots.forEach(visit);for(const m of methods.filter(m=>matches(m.input.title)))m.input.goal_ids.forEach(id=>keep.add(id));for(const id of [...keep]){let g=byId.get(id);const seen=new Set([id]);while(g?.input.parent_id&&!seen.has(g.input.parent_id)){seen.add(g.input.parent_id);keep.add(g.input.parent_id);g=byId.get(g.input.parent_id);}}}
 const visible=(r:GoalBranch)=>!query||keep.has(r.goal.id),open=(id:string)=>Boolean(query)||expanded.has(id),w=190,gap=42,pad=16,rowGap=30;
 const node=(kind:'goal'|'method',id:string,title:string,column:number,children=0):DiagramNode=>({key:kind+':'+id,kind,id,title,column,x:0,y:0,width:w,height:152+Math.ceil(Math.max(1,Array.from(title).length)/8)*26,children});
 const branchRank=(r:GoalBranch):number=>Math.min(ranks.get('goal:'+r.goal.id)??999,...r.children.map(branchRank),...r.methods.map(m=>ranks.get('method:'+m.id)??999));
 const sort=(rows:GoalBranch[])=>[...rows].filter(visible).sort((a,b)=>branchRank(a)-branchRank(b)||a.goal.input.title.localeCompare(b.goal.input.title,'zh-CN')||a.goal.id.localeCompare(b.goal.id));
 const heights=new Map<string,number>();
 const height=(r:GoalBranch):number=>{const children=open(r.goal.id)?sort(r.children):[],own=node('goal',r.goal.id,r.goal.input.title,0).height;const value=Math.max(own,children.reduce((sum,c)=>sum+height(c),0)+Math.max(0,children.length-1)*rowGap);heights.set(r.goal.id,value);return value;};
 let offset=52;
 const place=(r:GoalBranch,top:number)=>{const children=open(r.goal.id)?sort(r.children):[],n=node('goal',r.goal.id,r.goal.input.title,({long:0,medium:1,short:2})[r.goal.input.level],r.children.length+r.methods.length);n.goal=r.goal;n.y=top+((heights.get(r.goal.id)??n.height)-n.height)/2;nodes.push(n);let next=top;for(const c of children){place(c,next);edges.push({from:n.key,to:'goal:'+c.goal.id,kind:'parent'});next+=(heights.get(c.goal.id)??0)+rowGap;}};
 for(const r of sort(tree.roots)){height(r);place(r,offset);offset+=(heights.get(r.goal.id)??0)+46;}
 if(orientation==='vertical'){let y=52;for(const n of nodes){n.y=y;n.width=154;n.height=152+Math.ceil(Math.max(1,Array.from(n.title).length)/6)*26;y+=n.height+30;}}
 const visibleGoals=new Map(nodes.map(n=>[n.id,n]));
 const used=methods.filter(m=>m.input.goal_ids.some(id=>visibleGoals.has(id)&&open(id))||tree.unlinked.some(v=>v.id===m.id)&&(!query||matches(m.input.title)));
 let lastBottom=52;
 for(const m of [...used].sort((a,b)=>{
  const center=(m:Method)=>m.input.goal_ids.map(id=>visibleGoals.get(id)).filter(Boolean).reduce((sum,n)=>sum+n!.y,0)/Math.max(1,m.input.goal_ids.filter(id=>visibleGoals.has(id)).length);
  return center(a)-center(b)||(ranks.get('method:'+a.id)??999)-(ranks.get('method:'+b.id)??999)||a.id.localeCompare(b.id);
 })){
  const linked=m.input.goal_ids.map(id=>visibleGoals.get(id)).filter((n):n is DiagramNode=>Boolean(n&&open(n.id))),n=node('method',m.id,m.input.title,3);n.method=m;if(orientation==='vertical'){n.width=154;n.height=152+Math.ceil(Math.max(1,Array.from(n.title).length)/6)*26;}
  n.y=Math.max(lastBottom,linked.length?linked.reduce((s,g)=>s+g.y+g.height/2,0)/linked.length-n.height/2:52);lastBottom=n.y+n.height+rowGap;nodes.push(n);
  for(const g of linked)edges.push({from:g.key,to:n.key,kind:'method'});
 }
 const columns=[...new Set(nodes.map(n=>n.column))].sort((a,b)=>a-b);nodes.forEach(n=>n.x=orientation==='vertical'?(n.kind==='method'?206:16+n.column*8):pad+columns.indexOf(n.column)*(w+gap));
 return {nodes,edges,columns,orientation,width:orientation==='vertical'?376:Math.max(280,columns.length*(w+gap)-gap+pad*2),height:Math.max(180,...nodes.map(n=>n.y+n.height+24))};
}
