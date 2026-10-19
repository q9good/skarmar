import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { request } from './api';
import { useAuth } from './auth';

export type DeletedHandler = (kind: string, id: string) => void;
export function DeleteRecordButton({ kind, id, version, label, removed, hint }: {
  kind: string; id: string; version: number; label: string; removed: DeletedHandler; hint?: string;
}) {
  const { identity } = useAuth();
  const [confirmVersion, setConfirmVersion] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  if (identity.user.role !== 'admin' || identity.demo) return null;
  const button = (title: string, action: () => void) => <Pressable accessibilityRole="button" accessibilityState={{ disabled: busy }} disabled={busy}
    onPress={action} style={{ minHeight: 44, padding: 12, borderRadius: 10, backgroundColor: '#FBECE2' }}><Text style={{ color: '#925231', textAlign: 'center' }}>{title}</Text></Pressable>;
  const submit = async () => {
    if (busy || confirmVersion === null) return;
    setBusy(true); setError('');
    try { await request(`/${kind}/${id}`, 'DELETE', { expected_version: confirmVersion }); removed(kind, id); }
    catch (error) { setError(error instanceof Error ? error.message : '删除未完成，请重试'); }
    finally { setBusy(false); }
  };
  return <View style={{ gap: 10, marginTop: 12 }}>
    {confirmVersion === null ? button(`删除${label}`, () => { setConfirmVersion(version); setError(''); }) : <>
      <Text style={{ color: '#925231', lineHeight: 22 }}>确认删除{label}？页面不提供撤销。{hint ?? '其他关联记录会保留。'}</Text>
      {button(busy ? '正在删除…' : '确认删除', () => void submit())}
      {button('取消删除', () => { setConfirmVersion(null); setError(''); })}
    </>}
    {error ? <Text accessibilityRole="alert" style={{ color: '#925231' }}>{error}</Text> : null}
  </View>;
}
