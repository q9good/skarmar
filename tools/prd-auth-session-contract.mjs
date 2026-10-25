import assert from 'node:assert/strict';
export async function authSessionContract(label,send,admin,prdTestPassword){
 const headers={'Content-Type':'application/json','X-Skarma-Request':'1',Origin:'https://prd-contract.local'};
 async function call(method,path,body,cookie=admin,status=200){const r=await send('/api'+path,{method,headers:{...headers,...(cookie?{Cookie:cookie}:{})},body:body===undefined?undefined:JSON.stringify(body)});const data=await r.json();assert.equal(r.status,status,`${label} sessions ${JSON.stringify(data)}`);return {data,headers:r.headers};}
 const normal=await call('POST','/auth/login',{username:'owner',password:prdTestPassword,trusted_device:false},null);assert(normal.headers.get('set-cookie').includes('Max-Age=604800'));
 const trusted=await call('POST','/auth/login',{username:'owner',password:prdTestPassword,trusted_device:true},null);assert(trusted.headers.get('set-cookie').includes('Max-Age=31536000'));const trustedCookie=trusted.headers.get('set-cookie').split(';')[0];
 const info=(await call('GET','/auth/me',undefined,trustedCookie)).data;assert.equal(info.session.trusted_device,true);assert(info.session.expires_at>Date.now()/1000+364*86400);assert(!('token_hash' in info.session));
 await call('POST','/auth/sessions/revoke-others',undefined,null,401);await call('POST','/auth/sessions/revoke-others');await call('GET','/auth/me',undefined,trustedCookie,401);await call('GET','/auth/me');
 const owner=(await call('GET','/auth/me')).data;
 await call('POST','/auth/users',{username:'scope-guard-family',display_name:'合成身份核对家人',password:'s',role:'member'});
 const switched=await call('POST','/auth/login',{username:'scope-guard-family',password:'s'},null),memberCookie=switched.headers.get('set-cookie').split(';')[0],member=(await call('GET','/auth/me',undefined,memberCookie)).data;
 const gid=globalThis.crypto.randomUUID(),payload={id:gid,operation_id:globalThis.crypto.randomUUID(),expected_version:null,change_note:'',mutation:{kind:'goal',value:{title:'合成身份绑定目标',level:'short'}}};
 const guarded=async(method,path,body,actor,space=owner.space)=>{const r=await send('/api'+path,{method,headers:{...headers,Cookie:memberCookie,'X-Skarma-Actor':actor,'X-Skarma-Space':space,'X-Skarma-Contract':'skarmar.prd.v1'},body:body===undefined?undefined:JSON.stringify(body)});return {status:r.status,data:await r.json()};};
 for(const [method,path,body] of [['GET','/prd/state'],['PUT','/prd/records/'+gid,payload],['GET','/prd/export.xlsx'],['POST','/auth/logout'],['POST','/auth/sessions/revoke-others']])assert.equal((await guarded(method,path,body,owner.user.id)).status,409,`${label} cross-actor ${path}`);
 assert.equal((await guarded('GET','/prd/state',undefined,member.user.id,'wrong-space')).status,409);
 assert(!(await call('GET','/prd/state')).data.goals.some(g=>g.id===gid));
 const written=await guarded('PUT','/prd/records/'+gid,payload,member.user.id);assert.equal(written.status,200);assert.equal((await guarded('PUT','/prd/records/'+gid,payload,owner.user.id)).status,409);assert.equal((await guarded('PUT','/prd/records/'+gid,payload,member.user.id)).data.value.version,1);
 assert.equal((await call('GET','/auth/me',undefined,memberCookie)).data.user.id,member.user.id);
 console.log(`PASS ${label}: expected actor/space rejects stale reads, writes, export, account logout and replay; matching actor retries exactly once`);
 console.log(`PASS ${label}: opt-in trusted TTL, public expiry metadata and scoped session revocation`);
}
