import { Pressable, Text, TextInput } from './readability';
import React, { useState } from 'react';
import { View } from 'react-native';
import { request, type Lesson, type LessonOption, type NewLessonOption } from './api';
import { DateField, Select } from './controls';
import { lessonCounts, previousMonth, validDate } from './workflow';

export function LessonChoice({ label, kind, value, values, change, add, disabled }: {
  label: string; kind: LessonOption['kind']; value: string; values: string[]; change: (value: string) => void;
  add: (option: NewLessonOption) => Promise<void>; disabled: boolean;
}) {
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const options = [...new Set([...values, ...(value ? [value] : [])])].sort((a, b) => a.localeCompare(b, 'zh-CN'));
  return <View style={{ gap: 8 }}>
    <Select label={label} value={adding ? '__new__' : value} disabled={disabled || saving}
      options={[{ value: '', label: '请选择已有选项' }, ...options.map(value => ({ value, label: value })), { value: '__new__', label: '＋ 新增选项…' }]}
      onChange={selected => { setAdding(selected === '__new__'); setError(''); if (selected !== '__new__') change(selected); }} />
    {adding && <>
      <TextInput accessibilityLabel={`新增${kind === 'course' ? '课程' : '老师'}名称`} value={name} onChangeText={setName} editable={!disabled && !saving}
        style={{ borderWidth: 1, borderColor: '#E0E6DD', padding: 12, borderRadius: 10, minHeight: 45 }} />
      {error && <Text accessibilityRole="alert" style={{ color: '#925231' }}>{error}</Text>}
      <Pressable accessibilityRole="button" disabled={disabled || saving || !name.trim()} onPress={() => void (async () => {
        setSaving(true); setError('');
        try { await add({ kind, value: name.trim() }); change(name.trim()); setAdding(false); setName(''); }
        catch (e) { setError(e instanceof Error ? e.message : '新增选项失败，请重试'); }
        finally { setSaving(false); }
      })()} style={{ padding: 12, backgroundColor: '#E9F2E9', borderRadius: 10 }}><Text>{saving ? '正在保存…' : '保存并使用新选项'}</Text></Pressable>
      <Text style={{ color: '#53665A', fontSize: 12 }}>选项保存后，其他家人刷新即可选择；历史课程名称保持原样。</Text>
    </>}
  </View>;
}

export function LessonOptionManager({ options, saved }: { options: LessonOption[]; saved: (option: LessonOption) => void }) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [renaming, setRenaming] = useState<LessonOption | null>(null);
  const [newName, setNewName] = useState('');
  return <View style={{ gap: 10 }}>
    <Pressable accessibilityRole="button" onPress={() => setOpen(!open)} style={{ padding: 12, backgroundColor: '#E9F2E9', borderRadius: 10 }}><Text>{open ? '收起选项维护' : '维护课程和老师选项'}</Text></Pressable>
    {open && <><Text>停用只影响后续选择，不改变历史记录或课时统计。新增选项可在上课表单中操作。</Text>
      {error && <Text accessibilityRole="alert" style={{ color: '#925231' }}>{error}</Text>}
      <Text>改名只影响以后选择；历史上课记录保留当时名称。已更名的旧选项不能恢复，避免混用名称。</Text>
      {options.map(option => <View key={`${option.kind}:${option.value}`} style={{ padding: 10, borderBottomWidth: 1, borderColor: '#E0E6DD', gap: 6 }}>
        <Text>{option.kind === 'course' ? '课程' : '老师'}：{option.value}{option.active ? '' : ' · 已停用'}</Text>
        {option.renamed_to ? <Text>已更名为：{option.renamed_to}</Text> : <>
        <Pressable accessibilityRole="button" accessibilityLabel={`改名${option.kind === 'course' ? '课程' : '老师'}：${option.value}`} disabled={busy} onPress={() => {setRenaming(option);setNewName(option.value);setError('');}} style={{padding:10}}><Text>改名</Text></Pressable>
        <Pressable accessibilityRole="button" accessibilityLabel={`${option.active ? '停用' : '恢复'}${option.kind === 'course' ? '课程' : '老师'}：${option.value}`} disabled={busy} onPress={() => void (async () => {
          setBusy(true); setError('');
          try { saved(await request<LessonOption>('/lesson-options', 'PUT', { kind: option.kind, value: option.value, active: !option.active, expected_version: option.version })); }
          catch (e) { setError(e instanceof Error ? e.message : '选项更新失败'); }
          finally { setBusy(false); }
        })()} style={{ padding: 10 }}><Text>{option.active ? '停用' : '恢复使用'}</Text></Pressable>
        </>}
      </View>)}
      {renaming && <View style={{gap:8}}>
        <TextInput accessibilityLabel="选项新名称" value={newName} onChangeText={setNewName} editable={!busy} style={{borderWidth:1,padding:12}} />
        <Pressable accessibilityRole="button" accessibilityState={{disabled:busy || !newName.trim()}} disabled={busy || !newName.trim()} onPress={() => void (async () => {
          setBusy(true);setError('');
          try {
            const next=await request<LessonOption>('/lesson-options','PUT',{...renaming,expected_version:renaming.version,new_value:newName.trim()});
            if(next.value!==renaming.value) saved({...renaming,active:false,version:renaming.version+1,renamed_to:next.value});
            saved(next);setRenaming(null);
          } catch(e) {setError(e instanceof Error?e.message:'改名失败，请重试');} finally {setBusy(false);}
        })()} style={{padding:12}}><Text>{busy?'正在保存…':'保存新名称'}</Text></Pressable>
        <Pressable accessibilityRole="button" disabled={busy} onPress={()=>setRenaming(null)} style={{padding:12}}><Text>取消改名</Text></Pressable>
      </View>}
      {!options.length && <Text>还没有选项，可先新增上课记录。</Text>}
    </>}
  </View>;
}

export function LessonStatistics({ lessons, today }: { lessons: Lesson[]; today: string }) {
  const [range, setRange] = useState(() => previousMonth(today));
  const valid = validDate(range.start) && validDate(range.end) && range.start <= range.end;
  const rows = valid ? lessonCounts(lessons, range.start, range.end) : [];
  return <View testID="lesson-statistics" style={{ padding: 18, borderWidth: 1, borderColor: '#E0E6DD', borderRadius: 18, backgroundColor: 'white', gap: 10 }}>
    <Text style={{ fontSize: 17, fontWeight: '600', color: '#243D36' }}>机构课时统计</Text>
    <Text style={{ fontSize: 12, color: '#53665A' }}>默认上一个自然月，起止日期均计入；每条上课记录计一节课。</Text>
    <DateField label="统计开始日期" value={range.start} onChange={start => setRange({ ...range, start })} />
    <DateField label="统计结束日期" value={range.end} onChange={end => setRange({ ...range, end })} />
    <Pressable accessibilityRole="button" onPress={() => setRange(previousMonth(today))} style={{ padding: 12, backgroundColor: '#E9F2E9', borderRadius: 10 }}><Text>恢复上个自然月</Text></Pressable>
    {!valid ? <Text accessibilityRole="alert">请填写有效日期，结束日期不能早于开始日期。</Text> : <>
      <Text>共 {rows.reduce((sum, row) => sum + row.count, 0)} 节课 · {rows.length} 位老师</Text>
      {rows.length ? rows.map(row => <Text key={row.teacher}>{row.teacher}：{row.count} 节课</Text>) : <Text>所选时间内暂无上课记录。</Text>}
    </>}
  </View>;
}
