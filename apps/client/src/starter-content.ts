import pack from '../../../data/seeds/family-starter-v1.json';
import catalog from '../../../data/seeds/initial-catalog.json';
import {newGoal,newMethod,type GoalInput,type MethodInput} from './foundation';

export const starterGoals=pack.goals;
export const starterMethods=pack.methods;
export const starterCatalog=catalog.catalog.map(c=>({...c,key:'catalog.'+c.key}));
export const starterKeys=[...starterGoals,...starterMethods,...starterCatalog].map(r=>r.key);
export const defaultStarterKeys=[...starterGoals,...starterCatalog].map(r=>r.key);
export function goalReference(key:string):GoalInput {
  const r=starterGoals.find(r=>r.key===key);if(!r)throw new Error('未找到目标参考');
  return {...newGoal(),...r.input,level:'long',template:'text',state:'active'};
}
export function methodReference(key:string,goalIds:string[]=[]):MethodInput {
  const r=starterMethods.find(r=>r.key===key);if(!r)throw new Error('未找到方法参考');
  // Only preserve associations explicitly chosen in this form, never infer by title.
  return {...newMethod(),...r.input,goal_ids:[...goalIds],state:'active'};
}
export const blankGoalReference=(v:GoalInput)=>![v.title,v.meaning,v.criteria,v.support].some(x=>x?.trim());
export const blankMethodReference=(v:MethodInput)=>![v.title,v.description,v.steps,v.observe,v.scope].some(x=>x?.trim());
