import React, { useState } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { request, uploadFile } from './api';
import { pickImportFile } from './import-file';

type Counts = { rows: number; new: number; updated: number; skipped: number; invalid: number };
type Warning = { sheet?: string; row?: number; message: string };
type Sheet = Counts & {
  name: string; target: string; columns: { source: string; target: string }[];
  warnings: Warning[]; sample?: { row: number; label: string; action: string }[];
};
type Preview = {
  preview_id: string; filename: string; expires_at: string; totals: Counts;
  sheets: Sheet[]; warnings: Warning[]; can_commit: boolean;
};
type CommitResult = { batch_id: string; totals: Counts; sheets: Sheet[]; warnings?: Warning[] };

const maxBytes = 10 * 1024 * 1024;
const messageOf = (error: unknown) => error instanceof Error ? error.message : '操作未完成，请重试';
const counts = (value: Counts) =>
  `共 ${value.rows} 行 · 新增 ${value.new} · 更新 ${value.updated} · 跳过 ${value.skipped} · 无效 ${value.invalid}`;
const warningText = (warning: Warning) =>
  `${warning.sheet ? `${warning.sheet} · ` : ''}${warning.row == null ? '' : `第 ${warning.row} 行 · `}${warning.message}`;
const businessSummary = (sheets: Sheet[]) => {
  const added = (target: string) => sheets.filter(sheet => sheet.target === target).reduce((sum, sheet) => sum + sheet.new, 0);
  return `新增目标 ${added('目标')} 个，训练记录 ${added('训练记录')} 条；原始资料存档 ${added('原始资料存档')} 行。`;
};

function Action({ title, onPress, disabled, secondary }: {
  title: string; onPress: () => void; disabled?: boolean; secondary?: boolean;
}) {
  return <Pressable accessibilityRole="button" accessibilityState={{ disabled }} disabled={disabled} onPress={onPress}
    style={[s.button, secondary && s.secondary, disabled && s.disabled]}>
    <Text style={[s.buttonText, secondary && s.secondaryText]}>{title}</Text>
  </Pressable>;
}

export function ImportPanel({ onImported, disabled, onBusyChange }: {
  onImported: () => Promise<unknown>; disabled: boolean; onBusyChange: (busy: boolean) => void;
}) {
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [result, setResult] = useState<CommitResult | null>(null);
  const [busy, setBusy] = useState<'preview' | 'commit' | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [expanded, setExpanded] = useState<string[]>([]);
  const [allWarnings, setAllWarnings] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  const choose = async () => {
    if (disabled || busy) return;
    setError('');
    try {
      const selected = await pickImportFile();
      if (!selected) return;
      if (!/\.(xlsx|csv)$/i.test(selected.name)) throw new Error('请选择腾讯文档导出的 .xlsx 或 .csv 文件');
      if (selected.size > maxBytes) throw new Error('文件超过 10 MiB，请从腾讯文档分批导出');
      if (!selected.size) throw new Error('文件为空，请重新导出');
      setFile(selected); setPreview(null); setResult(null); setConfirmed(false);
      setExpanded([]); setAllWarnings(false); setNotice('');
    } catch (cause) { setError(messageOf(cause)); }
  };

  const upload = async () => {
    if (!file || disabled || busy) return;
    setBusy('preview'); onBusyChange(true); setError(''); setNotice(''); setPreview(null); setConfirmed(false);
    try {
      const next = await uploadFile<Preview>('/import/preview', file);
      setPreview(next); setExpanded([]); setAllWarnings(false);
      setNotice(next.can_commit
        ? '预览已生成，业务记录尚未写入。请核对表格、字段映射、数量和提示。'
        : '预览已生成，但有无法确认的字段或记录。请查看提示并调整导出文件后重新预览。');
    } catch (cause) { setError(messageOf(cause)); }
    finally { setBusy(null); onBusyChange(false); }
  };

  const commit = async () => {
    if (!preview || !preview.can_commit || !confirmed || disabled || busy) return;
    setBusy('commit'); onBusyChange(true); setError(''); setNotice('');
    try {
      const saved = await request<CommitResult>('/import/commit', 'POST', { preview_id: preview.preview_id });
      setResult(saved); setPreview(null); setFile(null); setConfirmed(false);
      try { await onImported(); setNotice('导入已完成，页面数据已刷新。'); }
      catch { setNotice('导入已完成，但页面数据暂未刷新。请稍后点击刷新查看。'); }
    } catch (cause) {
      setError(`${messageOf(cause)}。如果提交时连接中断，请保留当前预览并重试；重复确认不会重复导入。`);
    } finally { setBusy(null); onBusyChange(false); }
  };

  return <View style={s.card}>
    <Text style={s.title}>从腾讯文档导入</Text>
    <Text style={s.body}>在腾讯文档中导出“个训记录”为 Excel 或 CSV，再选择文件。只有管理员可以导入。</Text>
    <Text style={s.hint}>当前支持目标和历史训练记录；使用说明仅保留原始资料。其他工作表须先核对映射，不会自动迁移困难、经验或关联关系。请以预览结果为准。</Text>
    <Text style={s.hint}>选择文件只在本机进行；点击“上传并预览”会把文件发送到此空间的服务器进行解析。确认导入前，不会更改业务记录。</Text>
    <Action title={file ? '重新选择文件' : '选择 Excel／CSV 文件'} secondary disabled={disabled || Boolean(busy)} onPress={() => void choose()} />
    {file && <Text style={s.hint}>已选：{file.name} · {(file.size / 1024).toFixed(1)} KiB</Text>}
    {file && <Action title={busy === 'preview' ? '正在生成预览…' : '上传并预览'} disabled={disabled || Boolean(busy)} onPress={() => void upload()} />}
    {error ? <Text accessibilityRole="alert" style={s.error}>{error}</Text> : null}
    {notice ? <Text accessibilityRole="alert" style={s.notice}>{notice}</Text> : null}
    {preview && <View style={s.section}>
      <Text style={s.title}>导入预览</Text>
      <Text style={s.body}>{preview.filename}</Text>
      <Text style={s.hint}>{counts(preview.totals)}</Text>
      <Text style={s.body}>{businessSummary(preview.sheets)}</Text>
      {preview.sheets.map((sheet, index) => {
        const key = `${index}:${sheet.name}`;
        const open = expanded.includes(key);
        return <View key={key} style={s.sheet}>
          <Text style={s.sheetTitle}>{sheet.name} → {sheet.target || '未识别目标'}</Text>
          <Text style={s.hint}>{counts(sheet)}</Text>
          {sheet.target === '原始资料存档' &&
            <Text style={s.hint}>此表只保留原始行用于核对，不会生成目标、训练或跟进记录。</Text>}
          {sheet.sample?.length ? <View style={s.sample}>
            <Text style={s.hint}>记录示例</Text>
            {sheet.sample.map((item, sampleIndex) =>
              <Text key={`${item.row}:${sampleIndex}`} style={s.mapping}>第 {item.row} 行 · {item.label} · {item.action}</Text>)}
          </View> : null}
          {sheet.columns.length > 0 && <>
            <Action title={open ? '收起字段映射' : `查看字段映射 · ${sheet.columns.length}`} secondary
              disabled={disabled || Boolean(busy)} onPress={() => setExpanded(current => open ? current.filter(item => item !== key) : [...current, key])} />
            {open && sheet.columns.map((column, columnIndex) =>
              <Text key={`${column.source}:${columnIndex}`} style={s.mapping}>{column.source} → {column.target || '未识别'}</Text>)}
          </>}
          {sheet.warnings.slice(0, allWarnings ? undefined : 5).map((warning, warningIndex) =>
            <Text key={warningIndex} style={s.warning}>{warningText(warning)}</Text>)}
          {sheet.warnings.length > 5 && !allWarnings && <Text style={s.hint}>本表还有 {sheet.warnings.length - 5} 条提示。</Text>}
        </View>;
      })}
      {preview.warnings.slice(0, allWarnings ? undefined : 10).map((warning, index) =>
        <Text key={index} style={s.warning}>{warningText(warning)}</Text>)}
      {(preview.warnings.length > 10 || preview.sheets.some(sheet => sheet.warnings.length > 5)) &&
        <Action title={allWarnings ? '收起提示' : '查看全部提示'} secondary disabled={disabled || Boolean(busy)} onPress={() => setAllWarnings(value => !value)} />}
      {!preview.can_commit ? <Text accessibilityRole="alert" style={s.error}>预览包含未解决的映射或无效记录，暂不能写入。请核对提示并重新导出。</Text> : <>
        <Pressable accessibilityRole="checkbox" accessibilityState={{ checked: confirmed, disabled: disabled || Boolean(busy) }}
          disabled={disabled || Boolean(busy)} onPress={() => setConfirmed(value => !value)} style={s.confirmRow}>
          <Text style={s.checkbox}>{confirmed ? '☑' : '□'}</Text>
          <Text style={s.body}>我已核对预览，确认写入本空间的业务记录。</Text>
        </Pressable>
        <Action title={busy === 'commit' ? '正在导入…' : '确认导入'} disabled={disabled || Boolean(busy) || !confirmed} onPress={() => void commit()} />
      </>}
      <Text style={s.hint}>预览有效期至 {new Date(preview.expires_at).toLocaleString('zh-CN')}。如已过期，请重新上传文件。</Text>
    </View>}
    {result && <View style={s.section}>
      <Text style={s.title}>导入完成</Text>
      <Text style={s.hint}>{counts(result.totals)}</Text>
      <Text style={s.body}>{businessSummary(result.sheets)}</Text>
      <Text style={s.hint}>批次编号：{result.batch_id}</Text>
      {result.warnings?.map((warning, index) => <Text key={index} style={s.warning}>{warningText(warning)}</Text>)}
    </View>}
  </View>;
}

const s = StyleSheet.create({
  card: { padding: 18, borderRadius: 18, borderWidth: 1, borderColor: '#E0E6DD', backgroundColor: '#FFFFFF', gap: 12 },
  section: { borderTopWidth: 1, borderColor: '#E0E6DD', paddingTop: 14, gap: 10 },
  sheet: { borderWidth: 1, borderColor: '#E0E6DD', borderRadius: 10, padding: 12, gap: 7 },
  sample: { backgroundColor: '#F8F9F5', borderRadius: 8, padding: 10, gap: 5 },
  title: { fontSize: 16, fontWeight: '600', color: '#243D36' }, sheetTitle: { fontSize: 14, fontWeight: '600', color: '#243D36' },
  body: { fontSize: 14, color: '#243D36', lineHeight: 23 }, hint: { color: '#6C7B73', fontSize: 12, lineHeight: 21 },
  mapping: { fontSize: 12, color: '#475C51', lineHeight: 19 }, warning: { fontSize: 12, color: '#925231', lineHeight: 20 },
  button: { backgroundColor: '#34765B', borderRadius: 12, padding: 13, minHeight: 46, alignItems: 'center', justifyContent: 'center' },
  secondary: { backgroundColor: '#E9F2E9' }, disabled: { opacity: 0.5 }, buttonText: { color: '#FFFFFF', fontSize: 14, fontWeight: '600' },
  secondaryText: { color: '#34765B' },
  error: { backgroundColor: '#FBECE2', padding: 12, borderRadius: 10, color: '#925231', fontSize: 13, lineHeight: 21 },
  notice: { backgroundColor: '#E9F2E9', padding: 12, borderRadius: 10, color: '#34765B', fontSize: 13, lineHeight: 21 },
  confirmRow: { flexDirection: 'row', alignItems: 'center', gap: 8, paddingVertical: 5 }, checkbox: { color: '#34765B', fontSize: 23 },
});
