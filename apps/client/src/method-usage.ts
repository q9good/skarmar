import type {Snapshot} from './foundation';
import type {Session} from './prd-sessions';
import type {Lesson} from './prd-lessons';
import {historyEvidence} from './prd-history.ts';
export function methodUsage(data:Snapshot,id:string,today:string){
 const versions=new Map<number,{version:number;title:string;family:number;lesson:number;last:string}>();let family=0,lesson=0,last:string|null=null;const seen=new Set<string>();
 for(const e of historyEvidence(data,today,'combined')){
  const m=(e.kind==='family'?(e.record as Session).actual_methods:(e.record as Lesson).organization.methods).find(m=>m.id===id);if(!m||seen.has(e.kind+e.id))continue;seen.add(e.kind+e.id);
  if(e.kind==='family')family++;else lesson++;if(!last||e.date>last)last=e.date;
  const v=versions.get(m.version)??{version:m.version,title:m.input.title,family:0,lesson:0,last:e.date};v[e.kind==='family'?'family':'lesson']++;if(e.date>v.last)v.last=e.date;versions.set(m.version,v);
 }
 return {family,lesson,last,versions:[...versions.values()].sort((a,b)=>b.version-a.version)};
}
