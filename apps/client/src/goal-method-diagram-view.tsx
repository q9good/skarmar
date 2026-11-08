import React,{useId} from 'react';
import {ScrollView,View} from 'react-native';
import Svg,{Defs,Marker,Path} from 'react-native-svg';
import {Pressable,Text} from './readability';
import {neutral,palettes} from './ui-palette';
import {levelNames} from './foundation';
import {goalMethodDiagram,type DiagramNode} from './goal-method-diagram';
import type {Attention} from './object-priority';
export function GoalMethodDiagramView({model,attention,select,toggle,expanded}:{model:ReturnType<typeof goalMethodDiagram>;attention:(node:DiagramNode)=>Attention;select:(node:DiagramNode)=>void;toggle:(id:string)=>void;expanded:Set<string>}){
 const arrow='arrow'+useId().replace(/[^a-z0-9]/gi,''),positions=new Map(model.nodes.map(n=>[n.key,n]));
 if(!model.nodes.length)return <Text>当前范围没有目标或方法。</Text>;
 return <View style={{gap:8}}><Text style={{fontSize:14,color:neutral.muted}}>实线：上级 → 下级；虚线：目标 → 方法。</Text>
 <View style={{height:460,maxWidth:'100%',overflow:'hidden',borderWidth:1,borderColor:neutral.border,borderRadius:12,backgroundColor:neutral.surface}}>
 <ScrollView horizontal testID="relationship-diagram-pan" style={{height:458}} contentContainerStyle={{width:model.width}}>
 <ScrollView testID="relationship-diagram-scroll" style={{width:model.width,height:458}} contentContainerStyle={{width:model.width,height:model.height}}>
 <View testID="relationship-diagram" style={{width:model.width,height:model.height}}>
 {(model.orientation==='vertical'?[0,3]:model.columns).map((column,i)=><Text key={column} style={{position:'absolute',left:16+i*(model.orientation==='vertical'?190:232),top:12,width:190,fontSize:15,fontWeight:'700',color:column===3?palettes.method.ink:palettes.goal.ink}}>{model.orientation==='vertical'&&column!==3?'目标层级':['长期方向','中期目标','短期目标','方法'][column]}</Text>)}
 <Svg accessible={false} pointerEvents="none" width={model.width} height={model.height} style={{position:'absolute',left:0,top:0}}><Defs><Marker id={arrow} markerWidth={9} markerHeight={9} refX={8} refY={4.5} orient="auto"><Path d="M0,0 L9,4.5 L0,9 Z" fill={neutral.muted}/></Marker></Defs>{model.edges.map(e=>{const a=positions.get(e.from)!,b=positions.get(e.to)!,x=a.x+a.width,y=a.y+a.height/2,to=b.y+b.height/2;return <Path key={e.from+'>'+e.to} d={model.orientation==='vertical'&&e.kind==='parent'?`M ${a.x} ${y} H ${Math.max(4,a.x-10)} V ${to} H ${b.x-6}`:`M ${x} ${y} C ${x+22} ${y}, ${b.x-24} ${to}, ${b.x-6} ${to}`} fill="none" stroke={e.kind==='parent'?palettes.goal.ink:palettes.method.ink} strokeWidth={2} strokeDasharray={e.kind==='method'?'6 4':undefined} markerEnd={`url(#${arrow})`}/>;})}</Svg>
 {model.nodes.map(n=>{const a=attention(n),tone=palettes[n.kind],emphasis=a.level==='high'||a.level==='watch';return <View key={n.key} testID={'flow-node-'+n.kind+'-'+n.id} style={{position:'absolute',left:n.x,top:n.y,width:n.width,minHeight:n.height,borderWidth:a.level==='high'?2:1,borderColor:emphasis?tone.ink:neutral.border,borderRadius:n.kind==='method'?18:8,backgroundColor:emphasis?tone.soft:neutral.surface,padding:10,gap:3}}>
 <Pressable accessibilityRole="button" accessibilityLabel={`关系${n.kind==='goal'?'目标':'方法'}：${n.title}`} onPress={()=>select(n)} style={{minHeight:44,gap:6}}><Text style={{fontSize:14,color:tone.ink,fontWeight:'600'}}>{n.kind==='method'?'方法':levelNames[n.goal!.input.level]}</Text><Text style={{fontSize:14,color:tone.ink}}>{a.level==='high'?'高关注':a.level==='watch'?'需留意':a.level==='low'?'低关注':'普通关注'}</Text><Text style={{fontSize:17,color:tone.ink,fontWeight:'700'}}>{n.title}</Text></Pressable>
 {n.kind==='goal'&&n.children>0&&<Pressable accessibilityRole="button" accessibilityLabel={`${expanded.has(n.id)?'收起':'展开'}图中下级：${n.title}`} accessibilityState={{expanded:expanded.has(n.id)}} onPress={()=>toggle(n.id)} style={{minHeight:44,justifyContent:'center'}}><Text style={{fontSize:14,color:tone.ink}}>{expanded.has(n.id)?'− 收起下级':'+ 展开下级'}</Text></Pressable>}
 </View>;})}
 </View></ScrollView></ScrollView></View><Text style={{fontSize:14,color:neutral.muted}}>在图内左右、上下滑动；共享方法只画一个节点。</Text></View>;
}
