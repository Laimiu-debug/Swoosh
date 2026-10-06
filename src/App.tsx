import { useCallback, useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { History, Peer, Selection, Snapshot, Transfer } from './types';
import appIcon from '../design/brand/app-icon.svg';

const native = isTauri();
const terminal = (status: string) => ['completed', 'failed', 'cancelled', 'rejected'].includes(status);
const statusLabels: Record<string, string> = {
  connecting: '正在连接', awaiting_confirmation: '核对短码', awaiting_sender: '等待对方确认',
  awaiting_receiver: '等待对方接收', transferring: '正在传输', verifying: '正在校验',
  completed: '已完成', failed: '传输失败', cancelled: '已取消', rejected: '已拒绝',
};
function size(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024, unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++; }
  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
}

function Icon({ name, className = '' }: { name: string; className?: string }) {
  const paths: Record<string, ReactNode> = {
    file: <><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><path d="M14 3v6h6M8 13h8M8 17h5" /></>,
    folder: <path d="M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />,
    desktop: <><rect x="2" y="3" width="20" height="14" rx="2" /><path d="M8 21h8M12 17v4" /></>,
    phone: <><rect x="6" y="2" width="12" height="20" rx="3" /><path d="M10 5h4M11 18h2" /></>,
    arrow: <path d="M4 12h16m-6-6 6 6-6 6" />,
    close: <path d="m6 6 12 12M6 18 18 6" />,
    check: <path d="m5 12 4 4L19 6" />,
    link: <><path d="m9 15 6-6M14 7l2-2a4 4 0 0 1 6 6l-4 4M10 17l-2 2a4 4 0 0 1-6-6l4-4" /></>,
    moon: <path d="M20.8 13a9 9 0 0 1-9.8-9.8A9 9 0 1 0 20.8 13z" />,
    sun: <><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M2 12h2M20 12h2m-3-9-1 1M5 19l-1 1M4 4l1 1m14 14 1 1" /></>,
    text: <><path d="M4 5h16M12 5v14M8 19h8" /></>,
    receive: <path d="M12 3v12m-5-5 5 5 5-5M4 16v4h16v-4" />,
    send: <path d="M12 15V3m-5 5 5-5 5 5M4 16v4h16v-4" />,
    history: <><path d="M3 11a9 9 0 1 1 2.6 7M3 4v7h7M12 7v5l3 2" /></>,
  };
  return <svg className={`icon ${className}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name] ?? paths.file}</svg>;
}

function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => { const dialog = ref.current!; dialog.showModal(); return () => dialog.close(); }, []);
  return <dialog ref={ref} onCancel={onClose} onClick={event => { if (event.target === event.currentTarget) { const rect = event.currentTarget.getBoundingClientRect(); if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) onClose(); } }}>
    <div className="section-heading"><h2>{title}</h2><button className="icon-button" onClick={onClose} aria-label="关闭"><Icon name="close" /></button></div>{children}
  </dialog>;
}

function TransferCard({ task, run }: { task: Transfer; run: (command: string, args?: Record<string, unknown>) => Promise<unknown> }) {
  const done = terminal(task.status);
  const needsConfirm = task.status === 'awaiting_confirmation';
  return <article className={`transfer-card ${task.status === 'failed' ? 'has-error' : ''}`}>
    <div className="transfer-top"><span className={`round-icon ${task.status === 'completed' ? 'success' : ''}`}><Icon name={task.status === 'completed' ? 'check' : task.direction} /></span>
      <div className="flex-copy"><strong>{task.title}</strong><span>{task.direction === 'send' ? '发给' : '来自'} {task.peerName} · {task.kind === 'text' ? '文字' : `${task.count} 个文件`} · {size(task.totalBytes)}</span></div>
      <span className={`status-label ${task.status}`}>{statusLabels[task.status] ?? task.status}</span>
      {done && <button className="icon-button" aria-label="收起传输" onClick={() => void run('dismiss_transfer', { id: task.id })}><Icon name="close" /></button>}
    </div>
    {needsConfirm && <div className="confirmation"><p>确认两台设备的短码一致</p><div className="code">{task.code}</div><div className="actions">
      <button className="button primary" onClick={() => void run(task.direction === 'send' ? 'confirm_send' : 'respond_transfer', { id: task.id, accept: true })}>{task.direction === 'send' ? '一致，继续发送' : '一致，接收'}</button>
      <button className="button" onClick={() => void run(task.direction === 'send' ? 'cancel_transfer' : 'respond_transfer', { id: task.id, accept: false })}>{task.direction === 'send' ? '取消' : '拒绝'}</button>
    </div><small>两端确认后开始 · 60 秒内有效</small></div>}
    {!needsConfirm && !done && <div className="progress-row"><div className="progress-copy"><span>{size(task.transferredBytes)} / {size(task.totalBytes)}</span><button className="text-button" onClick={() => void run('cancel_transfer', { id: task.id })}>取消</button></div><progress value={task.transferredBytes} max={task.totalBytes || 1} /></div>}
    {task.error && <p className="error-text">{task.error}</p>}
    {task.savedPath && <button className="text-button open-action" onClick={() => void run('open_received', { path: task.savedPath })}><Icon name="folder" />打开收到的文件</button>}
    {task.text !== null && <div className="received-text"><pre>{task.text}</pre><button className="button" onClick={() => void run('copy_text', { text: task.text })}>复制文字</button></div>}
  </article>;
}

function HistoryRow({ item, run }: { item: History; run: (command: string, args?: Record<string, unknown>) => Promise<unknown> }) {
  return <div className="history-row"><span className="history-direction"><Icon name={item.direction} /></span><div className="flex-copy"><strong>{item.title}</strong><span>{item.peerName} · {size(item.totalBytes)} · {new Date(item.time * 1000).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</span></div><span className={`status-label ${item.status}`}>{statusLabels[item.status]}</span>{item.savedPath && <button className="icon-button" aria-label="打开接收文件夹" onClick={() => void run('open_received', { path: item.savedPath })}><Icon name="folder" /></button>}</div>;
}

export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [tab, setTab] = useState<'files' | 'text'>('files');
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [connecting, setConnecting] = useState(false);
  const [address, setAddress] = useState('');
  const [theme, setTheme] = useState(() => localStorage.getItem('swoosh-theme') ?? 'system');
  const previousFocus = useRef<HTMLElement | null>(null);
  const refresh = useCallback(async () => {
    if (!native) return;
    try { setSnapshot(await invoke<Snapshot>('snapshot')); } catch (error) { setError(String(error)); }
  }, []);
  useEffect(() => { void refresh(); const timer = window.setInterval(() => void refresh(), 600); return () => clearInterval(timer); }, [refresh]);
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'system') delete root.dataset.theme; else root.dataset.theme = theme;
    localStorage.setItem('swoosh-theme', theme);
  }, [theme]);
  useEffect(() => {
    if (!notice) return;
    const timeout = window.setTimeout(() => setNotice(''), 2500);
    return () => clearTimeout(timeout);
  }, [notice]);
  useEffect(() => {
    if (!native) return;
    let disposed = false, cleanup: (() => void) | undefined;
    void getCurrentWebviewWindow().onDragDropEvent(event => {
      setDragging(event.payload.type === 'over' || event.payload.type === 'enter');
      if (event.payload.type === 'drop') {
        setTab('files'); setBusy(true); setError('');
        void invoke<Selection>('select_dropped', { paths: event.payload.paths }).then(setSelection).catch(error => setError(String(error))).finally(() => setBusy(false));
      }
    }).then(unlisten => { if (disposed) unlisten(); else cleanup = unlisten; });
    return () => { disposed = true; cleanup?.(); };
  }, []);
  const run = useCallback(async (command: string, args?: Record<string, unknown>) => {
    setError('');
    try { const value = await invoke(command, args); await refresh(); if (command === 'copy_text') setNotice('已复制'); return value; }
    catch (error) { setError(String(error)); return null; }
  }, [refresh]);
  async function choose(folder: boolean) {
    setBusy(true); setError('');
    try { const chosen = await invoke<Selection | null>('choose_files', { folder }); if (chosen) setSelection(chosen); }
    catch (error) { setError(String(error)); } finally { setBusy(false); }
  }
  async function send(peer: Peer) {
    setBusy(true);
    await run(tab === 'files' ? 'send_files' : 'send_text', tab === 'files' ? { peerId: peer.device.id, selectionId: selection?.id } : { peerId: peer.device.id, text });
    setBusy(false);
  }
  function openConnection() { previousFocus.current = document.activeElement as HTMLElement; setConnecting(true); }
  function closeConnection() { setConnecting(false); previousFocus.current?.focus(); }
  const canSend = native && !busy && (tab === 'files' ? !!selection : !!text.trim() && new TextEncoder().encode(text).length <= 1024 * 1024);
  const peers = snapshot?.peers ?? [];
  const transfers = snapshot?.transfers ?? [];
  const history = snapshot?.history ?? [];
  const incoming = transfers.filter(task => task.direction === 'receive' && task.status === 'awaiting_confirmation').length;
  return <div className="app">
    <header className="app-header"><div className="brand"><img src={appIcon} alt="" /><div><strong>Swoosh</strong><span>选一下，嗖过去。</span></div></div>
      <div className="header-actions"><button className="icon-button" aria-label={`主题：${{ system: '跟随系统', light: '浅色', dark: '深色' }[theme]}，点击切换`} title={`主题：${{ system: '跟随系统', light: '浅色', dark: '深色' }[theme]}`} onClick={() => setTheme(theme === 'system' ? 'light' : theme === 'light' ? 'dark' : 'system')}><Icon name={theme === 'dark' ? 'moon' : theme === 'light' ? 'sun' : 'desktop'} /></button><button className="button" onClick={openConnection} disabled={!native}><Icon name="link" />连接设备</button></div>
    </header>
    <main>
      <div className="greeting"><div><h1>把东西递给身边的设备。</h1><p>同一网络，打开 Swoosh 就能相遇。</p></div><span className="self-device"><span className={`online-dot ${native ? '' : 'offline'}`} /><Icon name="desktop" /><span>{snapshot?.device.name ?? (native ? '正在启动…' : '桌面界面预览')}</span></span></div>
      {!native && <p className="banner">这是界面预览。请运行 Swoosh 桌面应用，使用文件选择和局域网传输。</p>}
      {snapshot?.networkWarning && <p className="banner">{snapshot.networkWarning}</p>}
      {error && !connecting && <div className="error-banner" role="alert"><span>{error}</span><button className="icon-button" onClick={() => setError('')} aria-label="关闭错误提示"><Icon name="close" /></button></div>}
      <div className="workspace">
        <section className="panel send-panel" aria-label="发送内容"><div className="tabs" role="tablist" aria-label="内容类型"><button role="tab" id="files-tab" aria-controls="files-panel" aria-selected={tab === 'files'} tabIndex={tab === 'files' ? 0 : -1} onClick={() => setTab('files')} onKeyDown={event => { if (event.key === 'ArrowRight') { setTab('text'); document.getElementById('text-tab')?.focus(); } }}><Icon name="file" />文件</button><button role="tab" id="text-tab" aria-controls="text-panel" aria-selected={tab === 'text'} tabIndex={tab === 'text' ? 0 : -1} onClick={() => setTab('text')} onKeyDown={event => { if (event.key === 'ArrowLeft') { setTab('files'); document.getElementById('files-tab')?.focus(); } }}><Icon name="text" />文字</button></div>
          {tab === 'files' ? <div role="tabpanel" id="files-panel" aria-labelledby="files-tab"><div className={`dropzone ${dragging ? 'dragging' : ''}`}><span className="file-art"><Icon name={selection ? 'check' : 'file'} /></span><h2>{busy ? '正在准备文件…' : selection ? '准备好了，发给谁？' : '文件、照片，都放这里。'}</h2><p>{selection ? `${selection.count} 个文件 · ${size(selection.totalBytes)}` : '拖进来，或从电脑里选一下'}</p><div className="actions"><button className="button primary" disabled={!native || busy} onClick={() => void choose(false)}>选择文件</button><button className="button" disabled={!native || busy} onClick={() => void choose(true)}>文件夹</button></div></div>
            {selection ? <div className="selection"><Icon name="file" /><div className="flex-copy"><strong title={selection.title}>{selection.title}</strong><span>{selection.entries.length} 项内容 · 保留目录结构</span></div><button className="icon-button" onClick={() => { setSelection(null); void run('clear_selection'); }} aria-label="移除选中文件"><Icon name="close" /></button></div> : <p className="send-hint">选好内容，再点右边的设备发送。</p>}</div> : <div role="tabpanel" id="text-panel" aria-labelledby="text-tab" className="text-panel"><label htmlFor="message">想递过去的文字</label><textarea id="message" value={text} onChange={event => setText(event.target.value)} placeholder="一段文字、一个链接，或刚复制的内容…" spellCheck={false} /><div className="text-meta"><span>收到后可以一键复制</span><span>{size(new TextEncoder().encode(text).length)} / 1 MB</span></div></div>}
        </section>
        <section className="panel devices-panel"><div className="section-heading"><h2>发给谁</h2><span className="muted">{peers.length ? `${peers.length} 台设备` : '附近设备'}</span></div>
          {peers.length ? <div className="devices">{peers.map(peer => <button className="device-card" key={peer.device.id} disabled={!canSend} onClick={() => void send(peer)} title={canSend ? `发送给 ${peer.device.name}` : '请先选择内容'}><span className="device-icon"><Icon name={['android', 'ios'].includes(peer.device.platform) ? 'phone' : 'desktop'} /></span><span className="flex-copy"><strong>{peer.device.name}</strong><span>{peer.device.platform === 'windows' ? 'Windows' : peer.device.platform} · {peer.discovered ? '附近' : '手动连接'}</span></span><Icon name="arrow" /></button>)}</div> : <div className="empty-devices"><div className="device-orbit"><Icon name="desktop" /><span /><Icon name="phone" /></div><h3>附近还没有设备</h3><p>让另一台设备也打开 Swoosh，<br />连接同一个 Wi-Fi 或局域网。</p><button className="text-button" onClick={openConnection} disabled={!native}>没找到？手动连接<Icon name="arrow" /></button></div>}
        </section>
      </div>
      {!!transfers.length && <section className="transfers" aria-label="传输任务"><div className="section-heading"><h2>{incoming ? `有 ${incoming} 项内容等你接收` : '正在传递'}</h2><span className="muted">双方确认后开始</span></div>{[...transfers].reverse().map(task => <TransferCard key={task.id} task={task} run={run} />)}</section>}
      <section className="history"><div className="section-heading"><h2><Icon name="history" />最近传输</h2>{!!history.length && <button className="text-button" onClick={() => void run('clear_history')}>清空记录</button>}</div>{history.length ? history.slice(0, 5).map(item => <HistoryRow key={item.id} item={item} run={run} />) : <p className="empty-history">第一次传递，从一份小文件开始。</p>}</section>
    </main>
    <footer><span><span className="online-dot" />本地连接 · 加密传输</span><button className="text-button" disabled={!native || !snapshot} onClick={() => void run('open_received', { path: snapshot?.receiveDir })}><Icon name="folder" />接收文件夹</button><span>0.1.0</span></footer>
    {notice && <div className="toast" role="status"><Icon name="check" />{notice}</div>}
    {connecting && <Modal title="连接设备" onClose={closeConnection}><p className="muted">两台电脑不用扫码，直接输入对方的地址。</p><div className="local-addresses"><label>这台电脑的地址</label>{snapshot?.addresses.length ? snapshot.addresses.map(value => <div className="address-row" key={value}><code>{value}</code><button className="text-button" onClick={() => void run('copy_text', { text: value })}>复制</button></div>) : <p className="muted">没有可用的局域网地址，请检查网络连接。</p>}</div><form onSubmit={event => { event.preventDefault(); setBusy(true); void run('connect_device', { address }).then(value => { if (value) { closeConnection(); setNotice('已连接，选好内容就能发送'); } }).finally(() => setBusy(false)); }}><label htmlFor="address">另一台设备的地址</label><input id="address" value={address} onChange={event => setAddress(event.target.value)} placeholder="192.168.1.8:53318" autoComplete="off" required /><p className="field-hint">在对方的「连接设备」里查看。</p>{error && <p className="error-text" role="alert">{error}</p>}<div className="actions dialog-actions"><button className="button" type="button" onClick={closeConnection}>关闭</button><button className="button primary" type="submit" disabled={busy || !address.trim()}>{busy ? '连接中…' : '连接'}</button></div></form></Modal>}
  </div>;
}
