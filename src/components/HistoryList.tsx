import type { History, Run } from '../types';
import { formatSize, formatTime, statusLabels } from '../format';
import { Icon } from './Icon';

const VISIBLE = 5;

export function HistoryList({ history, run }: { history: History[]; run: Run }) {
  return (
    <section className="history">
      <div className="section-heading">
        <h2><Icon name="history" />最近传输</h2>
        {history.length > 0 && <button className="text-button" onClick={() => void run('clear_history')}>清空记录</button>}
      </div>
      {history.length
        ? history.slice(0, VISIBLE).map(item => <HistoryRow key={item.id} item={item} run={run} />)
        : <p className="empty-history">第一次传递，从一份小文件开始。</p>}
    </section>
  );
}

function HistoryRow({ item, run }: { item: History; run: Run }) {
  return (
    <div className="history-row">
      <span className="history-direction"><Icon name={item.direction} /></span>
      <div className="flex-copy">
        <strong>{item.title}</strong>
        <span>{item.peerName} · {formatSize(item.totalBytes)} · {formatTime(item.time)}</span>
      </div>
      <span className={`status-label ${item.status}`}>{statusLabels[item.status]}</span>
      {item.savedPath && (
        <button className="icon-button" aria-label="打开接收文件夹" onClick={() => void run('open_received', { path: item.savedPath })}>
          <Icon name="folder" />
        </button>
      )}
    </div>
  );
}
