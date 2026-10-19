import React, { useMemo, useState } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import Svg, { Circle } from 'react-native-svg';
import type { AppState, Goal, Pathway } from './api';
import { buildPlanning, needsTraining, progressGroup, progressGroups, type PlanningLevel, type PlanningPeriod } from './planning';

const palette = ['#538A76', '#537DAD', '#C49661', '#9A80A3', '#6A9FA4', '#BE7C6B', '#A7AFA6'];
type Segment = { id: string; title: string; value: number; color: string };
function Action({ title, press, selected, disabled }: { title: string; press: () => void; selected?: boolean; disabled?: boolean }) {
  return <Pressable accessibilityRole="button" accessibilityState={{ selected, disabled }} disabled={disabled} onPress={press}
    style={[s.action, selected && s.selected, disabled && { opacity: 0.45 }]}><Text style={s.actionText}>{title}</Text></Pressable>;
}
function Ring({ segments, label, unit, size = 176, select, center }: { segments: Segment[]; label: string; unit: string; size?: number; select?: (id: string) => void; center?: string }) {
  const total = segments.reduce((sum, segment) => sum + segment.value, 0);
  const circumference = Math.PI * 140;
  let offset = 0;
  return <View style={{ width: size, height: size, alignSelf: 'center' }} accessibilityRole="image"
    accessibilityLabel={`${label}：${total ? segments.filter(segment => segment.value).map(segment => `${segment.title} ${segment.value}`).join('，') : '暂无数据'}`}>
    <Svg width={size} height={size} viewBox="0 0 180 180" accessible={false}>
      <Circle cx={90} cy={90} r={70} stroke="#EDF0EA" strokeWidth={20} fill="none" />
      {segments.filter(segment => segment.value > 0).map(segment => {
        const length = total ? circumference * segment.value / total : 0;
        const start = offset; offset += length;
        return <Circle key={segment.id} cx={90} cy={90} r={70} fill="none" stroke={segment.color} strokeWidth={20}
          strokeDasharray={`${length} ${circumference - length}`} strokeDashoffset={-start} rotation={-90} origin="90,90"
          onPress={select ? () => select(segment.id) : undefined} />;
      })}
    </Svg>
    <View pointerEvents="none" style={s.ringCenter}><Text style={size < 100 ? s.smallNumber : s.number}>{center ?? total}</Text><Text style={s.hint}>{unit}</Text></View>
  </View>;
}
function Chart({ title, segments, unit, explanation, select, grid }: { title: string; segments: Segment[]; unit: string; explanation: string; select?: (id: string) => void; grid?: boolean }) {
  const total = segments.reduce((sum, segment) => sum + segment.value, 0);
  return <View style={[s.card, grid && s.chart]}><Text style={s.title}>{title}</Text><Ring segments={segments} label={title} unit={unit} select={select} />
    {total ? segments.filter(segment => segment.value > 0).map(segment => {
      const content = <><View style={[s.dot, { backgroundColor: segment.color }]} /><Text style={[s.body, { flex: 1 }]}>{segment.title}</Text><Text style={s.label}>{segment.value} · {Math.round(segment.value / total * 100)}%</Text></>;
      return select ? <Pressable key={segment.id} accessibilityRole="button" accessibilityLabel={`查看分布：${segment.title}`} onPress={() => select(segment.id)} style={s.legend}>{content}</Pressable> :
        <View key={segment.id} style={s.legend}>{content}</View>;
    }) : <Text style={s.hint}>暂无数据，目标列表仍会显示尚未覆盖的方向。</Text>}
    <Text style={s.hint}>{explanation}</Text>
  </View>;
}

export function PlanningOverview({ data, today, plan, openGoal, openPathway, hasDraft, continueDraft, disabled }: {
  data: AppState; today: string; plan: (goal: Goal) => void; openGoal: (goal: Goal) => void;
  openPathway: (pathway: Pathway) => void; hasDraft: boolean; continueDraft: () => void; disabled: boolean;
}) {
  const [days, setDays] = useState<PlanningPeriod>(30);
  const [level, setLevel] = useState<PlanningLevel>('short');
  const [includeArchived, setIncludeArchived] = useState(false);
  const [view, setView] = useState<'goals' | 'pathways'>('goals');
  const [onlyUncovered, setOnlyUncovered] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const summary = useMemo(() => buildPlanning(data, { today, days, level, includeArchived }), [data, today, days, level, includeArchived]);
  const resetList = () => { setSelectedId(null); setOnlyUncovered(false); };
  const ranked = [...summary.rows].filter(row => row.count > 0).sort((a, b) => b.count - a.count || a.goal.id.localeCompare(b.goal.id));
  const top = ranked.slice(0, 6);
  const remaining = ranked.slice(6);
  const distribution: Segment[] = top.map((row, index) => ({ id: row.goal.id, title: row.goal.title, value: row.count, color: palette[index] }));
  if (remaining.length) distribution.push({ id: 'remaining', title: `其余 ${remaining.length} 个目标`, value: remaining.reduce((sum, row) => sum + row.count, 0), color: palette[6] });
  const remainingIds = new Set(remaining.map(row => row.goal.id));
  const rows = summary.rows.filter(row => (!onlyUncovered || summary.uncovered.includes(row)) && (!selectedId || (selectedId === 'remaining' ? remainingIds.has(row.goal.id) : row.goal.id === selectedId)));
  const maxCount = Math.max(1, ...summary.rows.map(row => row.count));
  return <>
    <Text style={s.body}>先看看哪些方向最近较少被照顾，再安排这一次。均衡推进不要求每个目标训练次数相同。</Text>
    {hasDraft && <Action title="继续当前训练计划" press={continueDraft} disabled={disabled} />}
    <View style={s.card}>
      <Text style={s.label}>训练统计时间</Text><View style={s.row}>
        {([7, 30, 90, null] as const).map(value => <Action key={String(value)} title={value === null ? '全部记录' : `近 ${value} 天`} selected={days === value} press={() => { setDays(value); resetList(); }} />)}
      </View><Text style={s.hint}>{summary.start ?? '最早记录'} — {today} · 按训练发生日期，含今天</Text>
      <View style={s.row}>{(['short', 'medium'] as const).map(value => <Action key={value} title={value === 'short' ? '短期目标' : '中期目标'} selected={level === value} press={() => { setLevel(value); resetList(); }} />)}
        <Action title={includeArchived ? '已包含归档' : '包含归档目标'} selected={includeArchived} press={() => { setIncludeArchived(!includeArchived); resetList(); }} /></View>
      <View style={s.row}><Action title="目标进展与训练" selected={view === 'goals'} press={() => setView('goals')} /><Action title="方法覆盖" selected={view === 'pathways'} press={() => setView('pathways')} /></View>
    </View>
    {view === 'goals' ? <>
      <View style={s.card} testID="planning-summary"><Text style={s.title}>这段时间照顾到了哪些目标？</Text>
        <Text style={s.body}>已覆盖 {summary.coveredGoals} / {summary.rows.length} 个目标 · 完成 {summary.completedCount} 场训练</Text>
        <Text style={s.body}>{summary.uncovered.length} 个仍待推进的目标近期没有训练记录</Text>
        <Text style={s.hint}>零次表示所选时间内未记录，不表示没有能力或从未练习。已达成、已完成及归档目标不列入待推进提醒。</Text>
        {summary.uncovered.length > 0 && <Action title="查看近期未覆盖目标" press={() => { setOnlyUncovered(true); setSelectedId(null); }} />}
      </View>
      {!onlyUncovered && !selectedId && <View style={s.charts}>
        <Chart grid title="训练分布" unit="次目标覆盖" segments={distribution}
          explanation="每场训练对同一目标只计一次；涉及多个目标会分别计数。图中比例表示训练覆盖分配，不代表时长或能力达成率。"
          select={id => { setOnlyUncovered(false); setSelectedId(id); }} />
        <Chart grid title="当前目标进展" unit="个目标" segments={summary.progress}
          explanation="根据目标当前确认的状态统计，不受训练时间筛选影响。归档本身不等于已达成；尚未设置量化达成百分比。" />
      </View>}
      {level === 'medium' && <Text style={s.hint}>中期次数包含直接训练及当时所属短期目标的训练；同一场不重复计数。下属短期达成数按当前目标关系计算。</Text>}
      <View style={s.card}><Text style={s.title}>安排前逐个看看</Text><Text style={s.hint}>默认按所选时间内次数从少到多排列；相同次数时，较久未记录的在前。</Text>
        {(onlyUncovered || selectedId) && <><Text style={s.label}>{onlyUncovered ? '正在查看：近期未覆盖且仍待推进' : `正在查看：${selectedId === 'remaining' ? `其余 ${remaining.length} 个目标` : summary.rows.find(row => row.goal.id === selectedId)?.goal.title ?? '所选目标'}`}</Text>
          <Action title="显示全部目标" press={resetList} /></>}
        {!summary.rows.length && <Text style={s.body}>暂无这个范围的目标，可到“目标”页创建，或切换层级与归档范围。</Text>}
        {summary.rows.length > 0 && !rows.length && <Text style={s.body}>当前筛选下没有目标。</Text>}
      </View>
      {rows.map(row => {
        const group = progressGroups.find(group => group.id === progressGroup(row.goal))!;
        return <View key={row.goal.id} style={s.card} testID={`planning-goal-${row.goal.id}`}>
          <Text style={s.title}>{row.goal.title}</Text>
          <Text style={[s.label, { color: group.color }]}>{row.goal.progress_status === 'achieved' ? '已达成' : row.goal.progress_status === 'completed' ? '已完成' : group.title}{row.goal.status === 'archived' ? ' · 已归档' : ''}</Text>
          <Text style={s.body}>所选时间 {row.count} 次 · 最近训练 {row.lastDate ?? '暂无记录'}</Text>
          <View style={s.track} accessibilityRole="image" accessibilityLabel={`${row.goal.title}训练覆盖 ${row.count} 次，本范围最多 ${maxCount === 1 && !ranked.length ? 0 : maxCount} 次`}>
            <View style={[s.fill, { width: `${row.count / maxCount * 100}%` }]} />
          </View>
          {level === 'medium' && <>
            <Text style={s.hint}>直接训练 {row.directCount} 次 · 仅通过下属短期覆盖 {row.childOnlyCount} 次</Text>
            <Text style={s.body}>下属短期已达成／完成 {row.completedChildren} / {row.totalChildren} 个</Text>
            {row.totalChildren > 0 ? <Ring size={88} center={`${row.completedChildren}/${row.totalChildren}`} label={`${row.goal.title}下属短期达成情况`} unit="已达成" segments={[
              { id: 'done', title: '已达成／完成', value: row.completedChildren, color: '#538A76' },
              { id: 'other', title: '其他状态', value: row.totalChildren - row.completedChildren, color: '#DCE3DA' },
            ]} /> : <Text style={s.hint}>尚无下属短期目标。</Text>}
          </>}
          {row.goal.criteria ? <Text style={s.hint}>达成标准：{row.goal.criteria}</Text> : null}
          {row.observation ? <Text style={s.observation}>最近观察 · {row.observationDate}：{row.observation}</Text> : <Text style={s.hint}>尚无逐目标观察，可先核对目标与活动是否合适。</Text>}
          <Text style={s.hint}>明确关联的使用中方法 {row.pathwayCount} 条</Text>
          <View style={s.row}>
            <Action title={hasDraft ? '加入当前计划' : '围绕此目标规划'} disabled={disabled} press={() => plan(row.goal)} />
            <Action title="查看目标与经验" press={() => openGoal(row.goal)} />
          </View>
          {!needsTraining(row.goal) && <Text style={s.hint}>此目标将以“复习”角色加入计划。</Text>}
        </View>;
      })}
    </> : <>
      <Chart title="方法对目标的覆盖" unit="个目标" segments={[
        { id: 'with', title: '已有使用中方法', value: summary.goalsWithPathways, color: '#538A76' },
        { id: 'without', title: '尚无使用中方法', value: summary.rows.length - summary.goalsWithPathways, color: '#C49661' },
      ]} explanation="按当前明确关联统计，与训练时间筛选无关。归档方法不计入覆盖；适用关系不表示训练时实际用过。" />
      <View style={s.card}><Text style={s.title}>有哪些方式可以尝试？</Text><Text style={s.body}>目前尚未记录每次实际使用的方法，因此不展示方法训练次数。</Text>
        <Text style={s.hint}>下面按明确绑定展示适用目标与经验数量。长期目标的绑定不会自动推算到中短期目标；经验数量不表示方法有效程度。</Text></View>
      {!summary.pathways.length && <View style={s.card}><Text style={s.body}>尚无此范围的方法，可到“方法”页创建并关联目标。</Text></View>}
      {summary.pathways.map(({ pathway, goals, experienceCount }) => <View key={pathway.id} style={s.card} testID={`planning-pathway-${pathway.id}`}>
        <Text style={s.title}>{pathway.title}</Text><Text style={s.hint}>{pathway.status === 'archived' ? '已归档' : '使用中'} · 当前范围适用 {goals.length} 个目标 · {experienceCount} 条经验</Text>
        <Text style={s.body}>{goals.map(goal => goal.title).join('、') || '当前范围暂无明确关联目标'}</Text>
        {pathway.description ? <Text style={s.hint}>{pathway.description}</Text> : null}
        <Action title="查看方法与细节" press={() => openPathway(pathway)} />
      </View>)}
    </>}
  </>;
}

const s = StyleSheet.create({
  card: { backgroundColor: '#FFFFFF', borderRadius: 18, padding: 18, borderWidth: 1, borderColor: '#E0E6DD', gap: 10, minWidth: 0 },
  title: { color: '#243D36', fontWeight: '600', fontSize: 17, lineHeight: 25 }, body: { color: '#243D36', fontSize: 14, lineHeight: 23 },
  label: { color: '#34765B', fontSize: 13, lineHeight: 21 }, hint: { color: '#6C7B73', fontSize: 12, lineHeight: 20 },
  number: { color: '#243D36', fontSize: 31, fontWeight: '600' }, smallNumber: { color: '#243D36', fontSize: 19, fontWeight: '600' },
  row: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 }, charts: { flexDirection: 'row', flexWrap: 'wrap', gap: 13 }, chart: { flexGrow: 1, flexBasis: 260 },
  action: { borderRadius: 11, backgroundColor: '#F5F6F2', borderWidth: 1, borderColor: '#E0E6DD', paddingHorizontal: 12, paddingVertical: 11, minHeight: 44, justifyContent: 'center', maxWidth: '100%' },
  selected: { borderColor: '#34765B', backgroundColor: '#E9F2E9' }, actionText: { color: '#34765B', fontSize: 13, lineHeight: 21 },
  ringCenter: { position: 'absolute', top: 0, right: 0, bottom: 0, left: 0, alignItems: 'center', justifyContent: 'center' },
  legend: { flexDirection: 'row', alignItems: 'center', gap: 9, minHeight: 44 }, dot: { width: 10, height: 10, borderRadius: 5 },
  track: { height: 8, borderRadius: 4, backgroundColor: '#EDF0EA', overflow: 'hidden' }, fill: { height: '100%', backgroundColor: '#538A76', borderRadius: 4 },
  observation: { color: '#536A5C', fontSize: 13, lineHeight: 22, backgroundColor: '#F4F7F0', padding: 12, borderRadius: 10 },
});
