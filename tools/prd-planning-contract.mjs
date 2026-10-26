// Synthetic only. The same HTTP checks run against native SQLite and Rust Wasm/DO.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';

export async function planningContract(label,send,admin){
  const call=async(method,path,body)=>{
    const response=await send('/api'+path,{method,headers:{Cookie:admin,Origin:'https://prd-contract.local','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});
    const value=await response.json();assert.equal(response.status,200,`${label}: ${JSON.stringify(value)}`);return value;
  };
  const today=(await call('GET','/prd/coverage?days=30&source=family')).end;
  const before=days=>new Date(Date.parse(today+'T12:00:00Z')-days*86400000).toISOString().slice(0,10);
  const inputs=[
    {title:'合成重点九天未记',level:'short',state:'active'},
    {title:'合成维持二十天未记',level:'medium',state:'maintaining'},
    {title:'合成明确不设频率',level:'short',state:'active',frequency:'none'},
    {title:'合成暂停目标',level:'short',state:'paused',frequency:'weekly'},
    {title:'合成收起目标',level:'short',state:'active',archived:true,frequency:'weekly'},
    {title:'合成已达成目标',level:'short',state:'achieved',frequency:'weekly'},
    {title:'合成未开始目标',level:'short',state:'not_started',frequency:'weekly'},
    {title:'合成长期方向',level:'long',state:'active',frequency:'weekly'},
  ],ids=[];
  for(const input of inputs){
    const id=randomUUID();ids.push(id);
    await call('PUT','/prd/records/'+id,{id,operation_id:randomUUID(),expected_version:null,change_note:'',mutation:{kind:'goal',value:{...input,attention_start:before(40)}}});
  }
  for(const [goal,age] of [[ids[0],9],[ids[1],20]]){
    const id=randomUUID();
    await call('PUT','/prd/sessions/'+id,{id,operation_id:randomUUID(),expected_version:null,action:'complete',input:{business_date:before(age),time_slot:'晚上',summary:{targets:[{goal_id:goal,performed:true,observation:'合成已确认的实际表现'}],confirmation:'none'}}});
  }
  for(const days of [7,30]){
    const report=await call('GET',`/prd/coverage?days=${days}&source=family`);
    assert.deepEqual(report.suggested_ids.filter(id=>ids.includes(id)),ids.slice(0,2));
    for(const [i,age,interval] of [[0,9,7],[1,20,30]]){
      const goal=report.goals.find(g=>g.id===ids[i]);
      assert.ok(Math.abs(goal.ratio-age/interval)<1e-12);
      assert.equal(goal.last_date,before(age));
      assert.equal(goal.counts.selected,days===30?1:0);
    }
    for(const id of ids.slice(2))assert.equal(report.goals.find(g=>g.id===id).ratio,null);
  }
  console.log(`PASS ${label}: 9/7 outranks 20/30; latest practice survives the window; paused, archived, achieved, not-started, long and frequency-none goals are not suggested`);
}
