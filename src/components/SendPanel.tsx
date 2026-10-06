import { useRef } from 'react';
import type { KeyboardEvent } from 'react';
import type { Selection } from '../types';
import { byteLength, formatSize } from '../format';
import { Icon } from './Icon';
import type { IconName } from './Icon';

export type ContentTab = 'files' | 'text';

const tabs: { id: ContentTab; label: string; icon: IconName }[] = [
  { id: 'files', label: '文件', icon: 'file' },
  { id: 'text', label: '文字', icon: 'text' },
];

type Props = {
  tab: ContentTab;
  onTab: (tab: ContentTab) => void;
  selection: Selection | null;
  onChoose: (folder: boolean) => void;
  onClear: () => void;
  text: string;
  onText: (text: string) => void;
  dragging: boolean;
  busy: boolean;
  native: boolean;
};

export function SendPanel({ tab, onTab, ...props }: Props) {
  const refs = useRef<Partial<Record<ContentTab, HTMLButtonElement | null>>>({});

  function moveFocus(event: KeyboardEvent, index: number) {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    const next = tabs[index + step];
    if (!step || !next) return;
    onTab(next.id);
    refs.current[next.id]?.focus();
  }

  return (
    <section className="panel send-panel" aria-label="发送内容">
      <div className="tabs" role="tablist" aria-label="内容类型">
        {tabs.map(({ id, label, icon }, index) => (
          <button key={id} ref={node => { refs.current[id] = node; }} role="tab" id={`${id}-tab`} aria-controls={`${id}-panel`}
            aria-selected={tab === id} tabIndex={tab === id ? 0 : -1} onClick={() => onTab(id)} onKeyDown={event => moveFocus(event, index)}>
            <Icon name={icon} />{label}
          </button>
        ))}
      </div>
      {tab === 'files' ? <FilesTab {...props} /> : <TextTab text={props.text} onText={props.onText} />}
    </section>
  );
}

function FilesTab({ selection, onChoose, onClear, dragging, busy, native }: Omit<Props, 'tab' | 'onTab' | 'text' | 'onText'>) {
  const heading = busy ? '正在准备文件…' : selection ? '准备好了，发给谁？' : '文件、照片，都放这里。';
  return (
    <div role="tabpanel" id="files-panel" aria-labelledby="files-tab">
      <div className={`dropzone ${dragging ? 'dragging' : ''}`}>
        <span className="file-art"><Icon name={selection ? 'check' : 'file'} /></span>
        <h2>{heading}</h2>
        <p>{selection ? `${selection.count} 个文件 · ${formatSize(selection.totalBytes)}` : '拖进来，或从电脑里选一下'}</p>
        <div className="actions">
          <button className="button primary" disabled={!native || busy} onClick={() => onChoose(false)}>选择文件</button>
          <button className="button" disabled={!native || busy} onClick={() => onChoose(true)}>文件夹</button>
        </div>
      </div>
      {selection ? (
        <div className="selection">
          <Icon name="file" />
          <div className="flex-copy">
            <strong title={selection.title}>{selection.title}</strong>
            <span>{selection.entries.length} 项内容 · 保留目录结构</span>
          </div>
          <button className="icon-button" onClick={onClear} aria-label="移除选中文件"><Icon name="close" /></button>
        </div>
      ) : <p className="send-hint">选好内容，再点右边的设备发送。</p>}
    </div>
  );
}

function TextTab({ text, onText }: { text: string; onText: (text: string) => void }) {
  return (
    <div role="tabpanel" id="text-panel" aria-labelledby="text-tab" className="text-panel">
      <label htmlFor="message">想递过去的文字</label>
      <textarea id="message" value={text} onChange={event => onText(event.target.value)} placeholder="一段文字、一个链接，或刚复制的内容…" spellCheck={false} />
      <div className="text-meta"><span>收到后可以一键复制</span><span>{formatSize(byteLength(text))} / 1 MB</span></div>
    </div>
  );
}
