import React from 'react';
import {View} from 'react-native';
import {Text,Pressable} from './readability';
import type {Snapshot} from './foundation';
import type {Session} from './prd-sessions';
import type {Lesson} from './prd-lessons';
import {methodUsage} from './method-usage';
import {localDate} from './draft';
export function MethodUsage({data,id,openVersion}:{data:Snapshot;id:string;openVersion:(version:number)=>void}){
 const usage=methodUsage(data,id,localDate());
 return <View style={{gap:8}}><Text style={{fontWeight:'600'}}>实际使用概况（已保存以来）</Text><Text>家庭{usage.family}次／机构{usage.lesson}次；最近使用：{usage.last??'尚无明确使用记录'}</Text><Text>只计明确实际使用，计划选择不计；改名或改版本不增加一次使用。</Text>{usage.versions.map(v=><View key={v.version} style={{gap:5}}><Text>方法第{v.version}版 · {v.title}：家庭{v.family}／机构{v.lesson}，最近{v.last}</Text><Pressable accessibilityRole="button" onPress={()=>openVersion(v.version)} style={{padding:10,borderRadius:8,backgroundColor:'#E9F2E9'}}><Text>查看方法第{v.version}版的使用记录</Text></Pressable></View>)}</View>;
}
