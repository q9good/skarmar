import type {Context,ReviewWrite} from './prd-followups';
import {seenEvidence} from './review-flow.ts';
export function assertSameContext(first:Context,next:Context){
 if(first.evidence_signature!==next.evidence_signature||first.schedule.version!==next.schedule.version||first.object_version!==next.object_version)throw new Error('依据或安排有更新，请保留当前填写，核对最新资料后再继续。');
}
/** Restore the records actually opened before exit. Newly arrived evidence is
 * available on explicit refresh/expand, never silently acknowledged on reload. */
export async function restoreReviewContext(r:ReviewWrite,readPage:(page:number)=>Promise<Context>):Promise<Context>{
 const first=await readPage(0),saved=r.seen_versions??{},wanted=new Set(Object.keys(saved).filter(k=>!k.startsWith('annotation:')));
 let last=first,rows=[...first.evidence];
 const found=()=>new Set(rows.map(e=>`${e.source.kind}:${e.source.id}`));
 while(last.has_more&&[...wanted].some(k=>!found().has(k))){
  if((last.page??0)>=199)throw new Error('此前查看的依据较多，请保留说明，选择重新核对最新依据。');
  last=await readPage((last.page??0)+1);assertSameContext(first,last);rows.push(...last.evidence);
 }
 const evidence=rows.filter(e=>saved[`${e.source.kind}:${e.source.id}`]===e.source.version);
 return {...last,restored:true,evidence,buffer:rows,server_more:last.has_more,has_more:evidence.length<rows.length||last.has_more,seen_versions:{...saved}};
}
export function revealBuffered(c:Context):Context{
 const ids=new Set(c.evidence.map(e=>`${e.source.kind}:${e.source.id}`)),add=(c.buffer??[]).filter(e=>!ids.has(`${e.source.kind}:${e.source.id}`)).slice(0,30),evidence=[...c.evidence,...add];
 return {...c,evidence,has_more:evidence.length<(c.buffer?.length??0)||Boolean(c.server_more),seen_versions:{...c.seen_versions,...seenEvidence(evidence)}};
}
