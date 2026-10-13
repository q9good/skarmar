import React, { createContext, useCallback, useContext, useEffect, useState } from 'react';
import { ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';
import { advanceAuthSession, ApiError, onUnauthorized, request } from './api';

export type Account = { id: string; username: string; display_name: string; role: 'admin' | 'member'; active: boolean };
export type Identity = { user: Account; space: string; demo: boolean };
type Auth = { identity: Identity; logout: () => Promise<void>; passwordChanged: () => void };
const Context = createContext<Auth | null>(null);
export function useAuth() { const value = useContext(Context); if (!value) throw new Error('尚未登录'); return value; }
const messageOf = (error: unknown) => error instanceof Error ? error.message : '操作未完成，请重试';

function Action({ title, onPress, disabled, secondary }: { title: string; onPress: () => void; disabled?: boolean; secondary?: boolean }) {
  return <Pressable accessibilityRole="button" accessibilityState={{ disabled }} disabled={disabled} onPress={onPress}
    style={[s.button, secondary && s.secondary, disabled && { opacity: 0.5 }]}>
    <Text style={[s.buttonText, secondary && { color: '#34765B' }]}>{title}</Text>
  </Pressable>;
}
function Input({ label, value, change, secret, username, disabled, submit }: {
  label: string; value: string; change: (text: string) => void; secret?: boolean; username?: boolean; disabled?: boolean; submit?: () => void;
}) {
  return <View style={s.field}><Text style={s.label}>{label}</Text><TextInput accessibilityLabel={label} value={value}
    onChangeText={change} secureTextEntry={secret} autoCapitalize="none" autoCorrect={false} editable={!disabled}
    autoComplete={username ? 'username' : secret ? 'current-password' : 'off'} maxLength={secret ? 128 : 120}
    onSubmitEditing={submit} style={s.input} /></View>;
}

export function AuthBoundary({ children }: { children: React.ReactNode }) {
  const [identity, setIdentity] = useState<Identity | null>(null);
  const [checking, setChecking] = useState(true);
  const [notice, setNotice] = useState('');
  const expire = useCallback(() => { advanceAuthSession(); setIdentity(null); setNotice('登录已失效，请重新登录。本机草稿仍为原账号保留。'); }, []);
  useEffect(() => { onUnauthorized(expire); return () => onUnauthorized(undefined); }, [expire]);
  useEffect(() => {
    let active = true;
    request<Identity>('/auth/me').then(value => { if (active) setIdentity(value); })
      .catch(error => { if (active && !(error instanceof ApiError && error.status === 401)) setNotice('暂时无法连接，请稍后重试登录。'); })
      .finally(() => { if (active) setChecking(false); });
    return () => { active = false; };
  }, []);
  useEffect(() => {
    if (!identity || identity.demo) return;
    let active = true;
    const timer = setInterval(() => {
      request<Identity>('/auth/me').catch(error => { if (active && error instanceof ApiError && error.status === 401) expire(); });
    }, 60_000);
    return () => { active = false; clearInterval(timer); };
  }, [identity, expire]);
  const logout = async () => {
    // Keep the authenticated view if the server cannot confirm session revocation.
    await request('/auth/logout', 'POST'); advanceAuthSession(); setIdentity(null); setNotice('已退出登录。这个账号的本机草稿已保留。');
  };
  if (checking) return <View style={s.loading}><ActivityIndicator color="#34765B" /><Text style={s.hint}>正在连接你的记录空间…</Text></View>;
  if (!identity) return <Login notice={notice} signedIn={value => { advanceAuthSession(); setNotice(''); setIdentity(value); }} />;
  return <Context.Provider value={{ identity, logout, passwordChanged: () => { advanceAuthSession(); setIdentity(null); setNotice('密码已更新，请重新登录。'); } }}>
    <React.Fragment key={`${identity.space}:${identity.user.id}`}>{children}</React.Fragment>
  </Context.Provider>;
}

function Login({ notice, signedIn }: { notice: string; signedIn: (identity: Identity) => void }) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [visible, setVisible] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const submit = async () => {
    if (busy) return;
    if (!username.trim() || !password) { setError('请填写账号和密码'); return; }
    setBusy(true); setError('');
    try { const value = await request<Identity>('/auth/login', 'POST', { username, password }); setPassword(''); signedIn(value); }
    catch (error) { setError(messageOf(error)); }
    finally { setBusy(false); }
  };
  return <ScrollView style={s.page} contentContainerStyle={s.loginContainer} keyboardShouldPersistTaps="handled">
    <View style={s.loginCard}>
      <View style={s.brand}><Text style={s.star}>✦</Text><Text style={s.brandName}>skarma</Text></View>
      <Text style={s.eyebrow}>记录小进步，陪伴每一天。</Text>
      <Text style={s.title}>欢迎回来</Text><Text style={s.body}>登录你的账号，继续今天的安排与观察。</Text>
      {notice ? <Text accessibilityRole="alert" style={s.notice}>{notice}</Text> : null}
      {error ? <Text accessibilityRole="alert" style={s.error}>{error}</Text> : null}
      <Input label="账号" value={username} change={setUsername} username disabled={busy} />
      <Input label="密码" value={password} change={setPassword} secret={!visible} disabled={busy} submit={() => void submit()} />
      <Pressable accessibilityRole="button" onPress={() => setVisible(value => !value)}><Text style={s.link}>{visible ? '隐藏密码' : '显示密码'}</Text></Pressable>
      <Action title={busy ? '正在登录…' : '登录'} onPress={() => void submit()} disabled={busy} />
      <Text style={s.hint}>首次使用或忘记密码，请联系空间管理员。</Text>
      <Text style={s.footer}>Every star shines at its own pace.</Text>
    </View>
  </ScrollView>;
}

export function AccountPanel({ back, logout }: { back: () => void; logout: () => Promise<void> }) {
  const auth = useAuth();
  const [users, setUsers] = useState<Account[]>([]);
  const [username, setUsername] = useState('');
  const [name, setName] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [admin, setAdmin] = useState(false);
  const [currentPassword, setCurrentPassword] = useState('');
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const load = useCallback(async () => { setUsers(await request<Account[]>('/auth/users')); }, []);
  useEffect(() => {
    if (auth.identity.user.role === 'admin') load().catch(error => setError(messageOf(error)));
  }, [auth.identity.user.role, load]);
  const act = async (action: () => Promise<void>) => {
    setBusy(true); setError(''); setNotice('');
    try { await action(); return true; } catch (error) { setError(messageOf(error)); return false; } finally { setBusy(false); }
  };
  return <>
    <View style={s.row}><Text style={s.title}>我的账号</Text><Action title="返回" secondary onPress={back} disabled={busy} /></View>
    <Text style={s.body}>{auth.identity.user.display_name} · {auth.identity.user.role === 'admin' ? '管理员' : '成员'}</Text>
    <Text style={s.hint}>账号：{auth.identity.user.username} · 空间：{auth.identity.space}</Text>
    {error ? <Text accessibilityRole="alert" style={s.error}>{error}</Text> : null}
    {notice ? <Text accessibilityRole="alert" style={s.notice}>{notice}</Text> : null}
    <View style={s.card}><Text style={s.cardTitle}>修改密码</Text>
      <Input label="当前密码" value={currentPassword} change={setCurrentPassword} secret disabled={busy} />
      <Input label="新密码" value={password} change={setPassword} secret disabled={busy} />
      <Input label="再次输入新密码" value={confirmation} change={setConfirmation} secret disabled={busy} />
      <Text style={s.hint}>至少 12 个字符。修改后所有设备需要重新登录。</Text>
      <Action title="更新密码" disabled={busy} onPress={() => void act(async () => {
        if (password !== confirmation) throw new Error('两次新密码不一致');
        await request('/auth/password', 'PUT', { current_password: currentPassword, password });
        setPassword(''); setCurrentPassword(''); setConfirmation(''); auth.passwordChanged();
      })} />
    </View>
    {auth.identity.user.role === 'admin' && <>
      <View style={s.card}><Text style={s.cardTitle}>添加成员</Text><Text style={s.hint}>成员可查看和填写本空间的全部记录。管理员还可管理账号。</Text>
        <Input label="新成员账号" value={username} change={setUsername} username disabled={busy} />
        <Input label="成员称呼" value={name} change={setName} disabled={busy} />
        <Input label="初始密码" value={newPassword} change={setNewPassword} secret disabled={busy} />
        <View style={s.row}><Action title="成员" secondary={admin} onPress={() => setAdmin(false)} disabled={busy} />
          <Action title="管理员" secondary={!admin} onPress={() => setAdmin(true)} disabled={busy} /></View>
        <Action title="创建账号" disabled={busy} onPress={() => void act(async () => {
          await request('/auth/users', 'POST', { username, display_name: name, password: newPassword, role: admin ? 'admin' : 'member' });
          setUsername(''); setName(''); setNewPassword(''); setAdmin(false); await load(); setNotice('账号已创建，请私下告知成员登录信息。');
        })} />
      </View>
      <Text style={s.cardTitle}>空间成员 · {users.length}</Text>
      {users.map(user => <Member key={user.id} user={user} current={user.id === auth.identity.user.id} busy={busy}
        update={input => act(async () => { await request(`/auth/users/${user.id}`, 'PUT', input); await load(); setNotice('成员信息已更新，原登录会话已撤销。'); })} />)}
    </>}
    <Action title="退出登录" secondary disabled={busy} onPress={() => void act(logout)} />
    <Text style={s.hint}>本机草稿按账号保留。共用设备时请及时退出，并使用各自的浏览器用户配置。</Text>
  </>;
}
function Member({ user, current, busy, update }: { user: Account; current: boolean; busy: boolean; update: (input: { active?: boolean; password?: string }) => Promise<boolean> }) {
  const [resetting, setResetting] = useState(false);
  const [password, setPassword] = useState('');
  return <View style={s.card}><Text style={s.cardTitle}>{user.display_name} {current ? '（你）' : ''}</Text>
    <Text style={s.hint}>{user.username} · {user.role === 'admin' ? '管理员' : '成员'} · {user.active ? '正常使用' : '已停用'}</Text>
    {!current && <>
      <View style={s.row}><Action title={user.active ? '停用账号' : '启用账号'} secondary disabled={busy} onPress={() => void update({ active: !user.active })} />
        <Action title={resetting ? '取消重置' : '重置密码'} secondary disabled={busy} onPress={() => setResetting(value => !value)} /></View>
      {resetting && <><Input label={`${user.display_name}的新密码`} value={password} change={setPassword} secret disabled={busy} />
        <Action title="确认重置密码" disabled={busy} onPress={() => void update({ password }).then(ok => { if (ok) { setPassword(''); setResetting(false); } })} /></>}
    </>}
  </View>;
}
const s = StyleSheet.create({
  page: { flex: 1, backgroundColor: '#F6F5EF' }, loginContainer: { flexGrow: 1, justifyContent: 'center', alignItems: 'center', padding: 22 },
  loginCard: { width: '100%', maxWidth: 430, backgroundColor: '#FFFFFF', borderRadius: 24, borderWidth: 1, borderColor: '#E0E6DD', padding: 26, gap: 14 },
  loading: { flex: 1, justifyContent: 'center', alignItems: 'center', backgroundColor: '#F6F5EF', gap: 16 },
  brand: { flexDirection: 'row', alignItems: 'center', gap: 8 }, star: { fontSize: 34, color: '#34765B' }, brandName: { color: '#243D36', fontSize: 28, fontWeight: '700' },
  eyebrow: { color: '#6C7B73', fontSize: 13, marginBottom: 15 }, title: { color: '#243D36', fontSize: 25, fontWeight: '700', flexShrink: 1 },
  body: { fontSize: 14, color: '#243D36', lineHeight: 23 }, hint: { color: '#6C7B73', fontSize: 12, lineHeight: 21 },
  card: { padding: 18, borderRadius: 18, borderWidth: 1, borderColor: '#E0E6DD', backgroundColor: '#FFFFFF', gap: 12 }, cardTitle: { fontSize: 16, fontWeight: '600', color: '#243D36' },
  field: { gap: 7 }, label: { fontSize: 13, color: '#243D36', fontWeight: '500' }, input: { minHeight: 48, borderWidth: 1, borderColor: '#E0E6DD', borderRadius: 10, padding: 12, color: '#243D36', backgroundColor: '#FCFCF9' },
  row: { flexDirection: 'row', flexWrap: 'wrap', alignItems: 'center', justifyContent: 'space-between', gap: 10 },
  button: { backgroundColor: '#34765B', borderRadius: 12, padding: 13, minHeight: 46, alignItems: 'center', justifyContent: 'center' }, secondary: { backgroundColor: '#E9F2E9' }, buttonText: { color: '#FFFFFF', fontSize: 14, fontWeight: '600' },
  link: { color: '#34765B', fontSize: 12, paddingVertical: 5, alignSelf: 'flex-end' },
  error: { backgroundColor: '#FBECE2', padding: 12, borderRadius: 10, color: '#925231', fontSize: 13, lineHeight: 21 },
  notice: { backgroundColor: '#E9F2E9', padding: 12, borderRadius: 10, color: '#34765B', fontSize: 13, lineHeight: 21 }, footer: { color: '#97A098', fontSize: 11, textAlign: 'center', marginTop: 12 },
});
