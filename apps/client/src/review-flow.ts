import type {Snapshot} from './foundation';
import type {Context,Evidence,ReviewWrite,Schedule,Review} from './prd-followups';
import {reviewObjectTitle} from './review-queue.ts';
export function seenEvidence(rows:Evidence[]){return Object.assign({},...rows.map(e=>e.seen_versions??{[`${e.source.kind}:${e.source.id}`]:e.source.version})) as Record<string,number>;}
export function previewContext(c:Context):Context{return {...c,buffer:c.evidence,evidence:c.evidence.slice(0,3),seen_versions:seenEvidence(c.evidence.slice(0,3)),has_more:c.evidence.length>3||c.has_more,server_more:c.has_more};}
export function nextReviewDate(r:ReviewWrite,c:Context|undefined,data:Snapshot):string|null|undefined{
 if(!r.reminders||['achieve','resolve'].includes(r.decision))return null;
 if(r.decision==='pause')return r.next_date;
 const d=c?.next_defaults;if(!d)return undefined;let automatic:string|null=d.current;
 if(r.object_kind==='goal'&&['maintain','restore'].includes(r.decision)){const g=data.goals.find(g=>g.id===r.object_id);automatic=g?.input.archived||r.decision==='restore'&&r.state==='not_started'?null:g?.input.level==='long'?d.monthly:d.fortnightly;}
 else if(['restore','reopen'].includes(r.decision)){
  if(r.object_kind==='method')automatic=d.monthly;
  else if(r.object_kind==='goal')automatic=r.state==='not_started'?null:data.goals.find(g=>g.id===r.object_id)?.input.level==='long'?d.monthly:d.fortnightly;
  else {const k=data.knowledge.find(k=>k.input.id===r.object_id);automatic=k?.input.kind==='difficulty'?(k.details?.priority?d.weekly:d.fortnightly):k?.input.stage==='idea'?null:d.monthly;}
 }
 return automatic===null?null:r.next_date??automatic;
}
export type BatchItem={kind:Schedule['object_kind'];id:string;generation:number;due:string|null;title:string;state:'pending'|'completed'|'deferred'|'changed';reviewId?:string;decision?:string;note?:string;actionIds?:string[];nextDate?:string|null};
export type ReviewBatch={schema:'skarmar.review-batch.v1';started:string;items:BatchItem[]};
export function startReviewBatch(schedules:Schedule[],data:Snapshot):ReviewBatch{
 const seen=new Set<string>();return {schema:'skarmar.review-batch.v1',started:new Date().toISOString(),items:schedules.flatMap(s=>{const key=s.object_kind+':'+s.object_id;if(seen.has(key))return [];seen.add(key);return [{kind:s.object_kind,id:s.object_id,generation:s.generation,due:s.due_date,title:reviewObjectTitle(s,data),state:'pending' as const}];})};
}
export function batchResult(batch:ReviewBatch,r:ReviewWrite,receipt:ReviewReceipt):ReviewBatch{
 if('object_kind' in receipt){return {...batch,items:batch.items.map(i=>i.reviewId===receipt.id?reviewReceiptItem(i,receipt):i)};}
 return {...batch,items:batch.items.map(i=>{
  const matching=r.action==='defer_group'?r.group_tokens?.some(t=>t.object_kind===i.kind&&t.object_id===i.id&&t.generation===i.generation):i.kind===r.object_kind&&i.id===r.object_id&&i.generation===r.generation;
  if(!matching)return i;if(r.action==='complete'&&receipt.review)return {...i,state:'completed',reviewId:receipt.review.id,decision:receipt.review.decision,note:receipt.review.note,actionIds:receipt.review.action_ids,nextDate:receipt.schedule?.due_date??null};
  return ['defer','defer_group'].includes(r.action)?{...i,state:'deferred',note:r.note,nextDate:receipt.schedule?.due_date??r.next_date}:i;
 })};
}
export function decodeReviewBatch(raw:string|null):ReviewBatch|null{
 if(!raw)return null;const v=JSON.parse(raw);if(!v||v.schema!=='skarmar.review-batch.v1'||typeof v.started!=='string'||!Array.isArray(v.items)||v.items.some((i:BatchItem)=>!i||!['goal','method','knowledge'].includes(i.kind)||typeof i.id!=='string'||typeof i.title!=='string'||['reviewId','decision','note'].some(k=>k in i&&typeof (i as unknown as Record<string,unknown>)[k]!=='string')||i.nextDate!==undefined&&i.nextDate!==null&&typeof i.nextDate!=='string'||!Number.isSafeInteger(i.generation)||i.generation<1||!(i.due===null||typeof i.due==='string')||!['pending','completed','deferred','changed'].includes(i.state)||i.actionIds!==undefined&&(!Array.isArray(i.actionIds)||i.actionIds.some(id=>typeof id!=='string'))))throw new Error('本机复盘批次无法读取，原内容已保留。');return v;
}

export type ReviewReceipt={review?:Review;schedule?:Schedule}|Review;
export function reviewReceiptItem(i:BatchItem,r:Review,nextDate=i.nextDate):BatchItem{
 if(r.deleted)return {...i,state:'changed',reviewId:r.id,note:'这条复盘已标为误建，请核对当前安排。',actionIds:[],nextDate};
 return {...i,state:'completed',reviewId:r.id,decision:r.decision,note:r.note,actionIds:r.action_ids,nextDate};
}
/** Re-read completed receipts too: an edited/deleted review cannot remain a
 * stale successful recap. Errors preserve the whole previous batch snapshot. */
export async function reconcileReviewBatch(batch:ReviewBatch,schedules:Schedule[],getContext:(kind:Schedule['object_kind'],id:string)=>Promise<Context>,getReview:(id:string)=>Promise<Review>):Promise<ReviewBatch>{
 const items=await Promise.all(batch.items.map(async i=>{
  const s=schedules.find(s=>s.object_kind===i.kind&&s.object_id===i.id);
  if(i.state==='completed'&&i.reviewId)return reviewReceiptItem(i,await getReview(i.reviewId));
  if(!['pending','changed'].includes(i.state))return i;
  if(s&&s.generation===i.generation&&s.due_date===i.due)return i;
  const c=await getContext(i.kind,i.id),r=c.last_review;
  return r&&!r.deleted&&r.generation===i.generation?reviewReceiptItem(i,r,c.schedule.due_date):{...i,state:'changed' as const};
 }));return {...batch,items};
}
