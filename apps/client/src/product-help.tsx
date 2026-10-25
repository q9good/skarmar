import React,{useEffect,useState} from 'react';
import {Linking,Platform,View} from 'react-native';
import {DetailOverlay} from './overlay';
import {Pressable,Text} from './readability';
import {request} from './api';
import {needsFields,type EvidenceRecord} from './prd-observations';
const sections=[
 ['自然情境、家长实施、强化与示范','NCAEP 2020有相应循证实践类别；NSP2 2015的自然教学、家长培训、行为干预与示范等列为Established。研究对象、培训和实施条件有范围，不能据此保证某个具体家庭做法有效。'],
 ['功能沟通与辅助沟通','NCAEP 2020有相应循证类别；NSP2相关类别的证据等级不同。接受手势、图片、辅助与替代沟通（AAC）等表达，不把口语作为获得帮助的门槛。AAC与Facilitated Communication不是同一种做法。'],
 ['感觉相关的做法','NCAEP 2020所说的特定Ayres感觉统合方案，与NSP2 2015广泛的感觉干预包不是同一分类。普通“感统课”的名称不足以直接套用研究结论。'],
 ['自然结果与小步挑战','没有仅凭“自然结果”这个名字即可认定有效或无效的独立分类。记录具体做法、支持、条件及孩子的反应，优先保持温和、愉快、可求助和可休息。'],
];
const sources=[['NCAEP 2020报告','https://ncaep.fpg.unc.edu/research-resources'],['AFIRM实践模块','https://afirm.fpg.unc.edu/'],['NICE儿童与青少年支持建议','https://www.nice.org.uk/guidance/cg170/chapter/recommendations'],['AAP 2020临床报告','https://doi.org/10.1542/peds.2019-3447']];
export function HelpButton({title='查看说明'}:{title?:string}){
 const [open,setOpen]=useState(false),[needs,setNeeds]=useState<EvidenceRecord|null>(null),[error,setError]=useState('');
 useEffect(()=>{if(!open)return;let live=true;setError('');void request<{items:EvidenceRecord[]}>('/prd/process?view=observations&object_kind=needs').then(v=>{if(live)setNeeds(v.items.find(r=>!r.deleted)??null);}).catch(()=>{if(live)setError('共享需要说明暂未读到，可以稍后在我的账号中查看。');});return()=>{live=false;};},[open]);
 return <><Pressable accessibilityRole="button" onPress={()=>setOpen(true)} style={{padding:12,minHeight:48,borderRadius:10,backgroundColor:'#E9F2E9'}}><Text>{title}</Text></Pressable>{open&&<DetailOverlay title="使用与方法说明" close={()=>setOpen(false)}>
  <Text style={{fontSize:20,fontWeight:'600'}}>看见进展，帮助调整</Text><Text>这份记录帮助家人围绕沟通、生活参与、自主与舒适度观察变化。记录次数表示覆盖，不等于能力进步；没有记录也不等于没有陪伴。软件不作诊断或治疗承诺。</Text>
  <Text style={{fontSize:18,fontWeight:'600'}}>怎样理解方法的证据</Text><Text>以下是PRD已核对资料的分类摘要，注明原始年份，不声称是持续自动更新的临床指南。循证实践仍需结合孩子、家庭目标、专业指导及实际反应来选择。</Text>{sections.map(([heading,body])=><View key={heading} style={{gap:5}}><Text style={{fontWeight:'600'}}>{heading}</Text><Text>{body}</Text></View>)}
  <Text>涉及药物、饮食限制、补充剂、仪器或排毒类做法，先与合适的专业人员讨论。治疗共存的身体问题与改善ASD核心特征是不同的目的；这里不会根据方法名称阻止保存。</Text>
  <Text>经验阶段、三次可比观察提示、色阶和复盘日程属于家庭及产品的安排，不是经过验证的临床评分或统一训练剂量。</Text>
  {sources.map(([label,url])=><Pressable accessibilityRole="link" accessibilityLabel={label} key={url} onPress={()=>void Linking.openURL(url)} style={{paddingVertical:12}}><Text style={{color:'#245D48',textDecorationLine:'underline'}}>{label} ↗</Text></Pressable>)}
  <Text style={{fontSize:18,fontWeight:'600'}}>家人共同记住的需要与帮助</Text>{error&&<Text>{error}</Text>}{needs?needsFields.filter(([k])=>needs.input[k]).map(([k,label])=><Text selectable key={k}>{label}：{String(needs.input[k])}</Text>):!error&&<Text>暂未填写。可在“我的账号 → 共享需要与帮助说明”维护；这些内容都可留空。</Text>}
  <Text style={{fontSize:18,fontWeight:'600'}}>添加到手机桌面</Text><Text>{Platform.OS==='web'?'iPhone：用Safari打开本站，点分享按钮，选择“添加到主屏幕”。安卓：用Chrome打开本站，在菜单中选择“添加到主屏幕”或“安装应用”；名称因浏览器版本而异。':'正式原生应用仍在后续批次，当前以手机网页为主。'}</Text><Text>图标名称为skarmar。桌面图标只是更方便打开本站，不保证离线启动或网页关闭后继续上传。离线课记须以待上传状态和服务器回执区分。</Text>
 </DetailOverlay>}</>;
}
