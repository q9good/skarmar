import type {Goal} from './foundation';

/** The saved catalogue is shared with Goals; never create choices from examples. */
export function visibleGoals(goals:Goal[],showArchived=false,selected:string[]=[]){
 const keep=new Set(selected);
 return goals.filter(g=>showArchived||!g.input.archived||keep.has(g.id));
}

/** Ancestors are implied associations, not additional performed targets or notes. */
export function actualGoalRelations(goals:Goal[],selected:string[]){
 const byId=new Map(goals.map(g=>[g.id,g])),rank={long:0,medium:1,short:2},explicit=new Set(selected),inherited=new Map<string,string[]>();
 for(const id of explicit){
  let child=byId.get(id);const seen=new Set([id]);
  while(child?.input.parent_id){
   const parent=byId.get(child.input.parent_id);
   if(!parent||seen.has(parent.id)||rank[parent.input.level]>=rank[child.input.level])break;
   seen.add(parent.id);const sources=inherited.get(parent.id)??[];
   if(!sources.includes(id))sources.push(id);inherited.set(parent.id,sources);child=parent;
  }
 }
 return {explicit,inherited,associated:new Set([...explicit,...inherited.keys()])};
}
