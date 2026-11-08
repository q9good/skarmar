import assert from 'node:assert/strict';
import {test} from 'node:test';
import {goalMethodDiagram} from '../apps/client/src/goal-method-diagram.ts';
const g=(id,level,parent_id=null)=>({id,version:1,input:{title:id,level,parent_id}});
const goals=[g('long','long'),g('medium','medium','long'),g('short','short','medium'),g('other','long')],m={id:'shared',input:{title:'shared method',goal_ids:['short','other']}};
test('diagram draws one shared method with two real incoming edges and direct parent edges',()=>{
 const r=goalMethodDiagram(goals,[m],new Set(goals.map(g=>g.id)),new Map());
 assert.equal(r.nodes.filter(n=>n.kind==='method').length,1);
 assert.deepEqual(r.edges.filter(e=>e.kind==='method').map(e=>e.from).sort(),['goal:other','goal:short']);
 assert(r.edges.some(e=>e.from==='goal:medium'&&e.to==='goal:short'));
 for(const n of r.nodes)assert(n.x>=0&&n.y>=0&&n.x+n.width<=r.width&&n.y+n.height<=r.height);
});
test('folding hides unrevealed children and search restores only actual matching paths',()=>{
 const folded=goalMethodDiagram(goals,[m],new Set(),new Map());assert.deepEqual(folded.nodes.map(n=>n.id).sort(),['long','other']);
 const search=goalMethodDiagram(goals,[m],new Set(),new Map(),'short');assert.deepEqual(search.nodes.map(n=>n.id).sort(),['long','medium','shared','short']);
 assert(!search.edges.some(e=>e.from==='goal:other'));
});
test('vertical mobile graph keeps nodes distinct and titles have measured room without truncation',()=>{
 const longTitle={...goals[2],input:{...goals[2].input,title:'长标题'.repeat(40)}};
 const r=goalMethodDiagram([...goals.slice(0,2),longTitle,goals[3]],[m],new Set(goals.map(g=>g.id)),new Map(),'','vertical');
 assert.equal(r.width,376);assert.equal(r.nodes.filter(n=>n.id==='shared').length,1);
 const ns=r.nodes.filter(n=>n.kind==='goal').sort((a,b)=>a.y-b.y);for(let i=1;i<ns.length;i++)assert(ns[i].y>=ns[i-1].y+ns[i-1].height);
 assert(r.nodes.find(n=>n.id==='short').height>500);
});
