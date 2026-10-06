import { useCallback, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Peer, Selection } from './types';
import { byteLength, displayPath, TEXT_LIMIT } from './format';
import { native, useCore, useFileDrop, useTheme } from './hooks';
import { Icon } from './components/Icon';
import { SendPanel } from './components/SendPanel';
import type { ContentTab } from './components/SendPanel';
import { DevicesPanel } from './components/DevicesPanel';
import { TransferCard } from './components/TransferCard';
import { HistoryList } from './components/HistoryList';
import { ConnectDialog } from './components/ConnectDialog';
import { ReceiveFolderDialog } from './components/ReceiveFolderDialog';
import appIcon from '../design/brand/app-icon.svg';
import { version } from '../package.json';

const themes = {
  system: { label: '跟随系统', icon: 'desktop' },
  light: { label: '浅色', icon: 'sun' },
  dark: { label: '深色', icon: 'moon' },
} as const;

export default function App() {
  const { snapshot, run, error, setError, notice, setNotice } = useCore();
  const { theme, cycleTheme } = useTheme();
  const [tab, setTab] = useState<ContentTab>('files');
  const [selection, setSelection] = useState<Selection | null>(null);
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [dialog, setDialog] = useState<'connect' | 'receive-folder' | null>(null);

  /** Prepares a selection from the picker or a drop; the scan can take a while for large folders. */
  const select = useCallback(async (pick: () => Promise<Selection | null>) => {
    setTab('files'); setBusy(true); setError('');
    try { const chosen = await pick(); if (chosen) setSelection(chosen); }
    catch (error) { setError(String(error)); }
    finally { setBusy(false); }
  }, [setError]);

  const dragging = useFileDrop(useCallback(paths => void select(() => invoke<Selection>('select_dropped', { paths })), [select]));

  async function send(peer: Peer) {
    setBusy(true);
    const peerId = peer.device.id;
    await (tab === 'files' ? run('send_files', { peerId, selectionId: selection?.id }) : run('send_text', { peerId, text }));
    setBusy(false);
  }

  function clearSelection() {
    setSelection(null);
    void run('clear_selection');
  }

  function openDialog(name: NonNullable<typeof dialog>) {
    setError('');
    setDialog(name);
  }

  const hasContent = tab === 'files' ? !!selection : !!text.trim() && byteLength(text) <= TEXT_LIMIT;
  const canSend = native && !busy && hasContent;
  const peers = snapshot?.peers ?? [];
  const transfers = snapshot?.transfers ?? [];
  const incoming = transfers.filter(task => task.direction === 'receive' && task.status === 'awaiting_confirmation').length;
  const receiveDir = snapshot?.receiveDir ?? '';
  const { label: themeLabel, icon: themeIcon } = themes[theme];

  return (
    <div className="app">
      <header className="app-header">
        <div className="brand">
          <img src={appIcon} alt="" />
          <div><strong>Swoosh</strong><span>选一下，嗖过去。</span></div>
        </div>
        <div className="header-actions">
          <button className="icon-button" aria-label={`主题：${themeLabel}，点击切换`} title={`主题：${themeLabel}`} onClick={cycleTheme}><Icon name={themeIcon} /></button>
          <button className="button" onClick={() => openDialog('connect')} disabled={!native}><Icon name="link" />连接设备</button>
        </div>
      </header>

      <main>
        <div className="greeting">
          <div><h1>把东西递给身边的设备。</h1><p>同一网络，打开 Swoosh 就能相遇。</p></div>
          <span className="self-device">
            <span className={`online-dot ${native ? '' : 'offline'}`} />
            <Icon name="desktop" />
            <span>{snapshot?.device.name ?? (native ? '正在启动…' : '桌面界面预览')}</span>
          </span>
        </div>

        {!native && <p className="banner">这是界面预览。请运行 Swoosh 桌面应用，使用文件选择和局域网传输。</p>}
        {snapshot?.networkWarning && <p className="banner">{snapshot.networkWarning}</p>}
        {snapshot?.receiveDirWarning && (
          <div className="banner receive-warning">
            <span>{snapshot.receiveDirWarning}</span>
            <button className="text-button" onClick={() => openDialog('receive-folder')}>选择文件夹</button>
          </div>
        )}
        {error && !dialog && (
          <div className="error-banner" role="alert">
            <span>{error}</span>
            <button className="icon-button" onClick={() => setError('')} aria-label="关闭错误提示"><Icon name="close" /></button>
          </div>
        )}

        <div className="workspace">
          <SendPanel tab={tab} onTab={setTab} selection={selection} text={text} onText={setText} dragging={dragging} busy={busy} native={native}
            onChoose={folder => void select(() => invoke<Selection | null>('choose_files', { folder }))} onClear={clearSelection} />
          <DevicesPanel peers={peers} canSend={canSend} native={native} onSend={peer => void send(peer)} onConnect={() => openDialog('connect')} />
        </div>

        {transfers.length > 0 && (
          <section className="transfers" aria-label="传输任务">
            <div className="section-heading">
              <h2>{incoming ? `有 ${incoming} 项内容等你接收` : '正在传递'}</h2>
              <span className="muted">双方确认后开始</span>
            </div>
            {transfers.slice().reverse().map(task => <TransferCard key={task.id} task={task} run={run} />)}
          </section>
        )}

        <HistoryList history={snapshot?.history ?? []} run={run} />
      </main>

      <footer>
        <span><span className="online-dot" />本地连接 · 加密传输</span>
        <div className="receive-folder-actions">
          <button className="text-button" title={displayPath(receiveDir)} disabled={!snapshot} onClick={() => void run('open_received', { path: receiveDir })}><Icon name="folder" />接收文件夹</button>
          <button className="text-button" disabled={!snapshot} onClick={() => openDialog('receive-folder')} aria-label="设置接收文件夹">更改</button>
        </div>
        <span>{version}</span>
      </footer>

      {notice && <div className="toast" role="status"><Icon name="check" />{notice}</div>}
      {dialog === 'receive-folder' && <ReceiveFolderDialog receiveDir={receiveDir} error={error} run={run} notify={setNotice} onClose={() => setDialog(null)} />}
      {dialog === 'connect' && <ConnectDialog addresses={snapshot?.addresses ?? []} error={error} run={run} onConnected={() => setDialog(null)} onClose={() => setDialog(null)} />}
    </div>
  );
}
