import type { Run, Transfer } from '../types';
import { formatSize, isTerminal, statusLabels } from '../format';
import { Icon } from './Icon';

export function TransferCard({ task, run }: { task: Transfer; run: Run }) {
  const { id, status, direction } = task;
  const sending = direction === 'send';
  const done = isTerminal(status);
  const needsConfirm = status === 'awaiting_confirmation';
  const completed = status === 'completed';
  const contents = task.kind === 'text' ? '文字' : `${task.count} 个文件`;

  const accept = () => run(sending ? 'confirm_send' : 'respond_transfer', { id, accept: true });
  const decline = () => run(sending ? 'cancel_transfer' : 'respond_transfer', { id, accept: false });

  return (
    <article className={`transfer-card ${status === 'failed' ? 'has-error' : ''}`}>
      <div className="transfer-top">
        <span className={`round-icon ${completed ? 'success' : ''}`}><Icon name={completed ? 'check' : direction} /></span>
        <div className="flex-copy">
          <strong>{task.title}</strong>
          <span>{sending ? '发给' : '来自'} {task.peerName} · {contents} · {formatSize(task.totalBytes)}</span>
        </div>
        <span className={`status-label ${status}`}>{statusLabels[status]}</span>
        {done && <button className="icon-button" aria-label="收起传输" onClick={() => void run('dismiss_transfer', { id })}><Icon name="close" /></button>}
      </div>

      {needsConfirm && (
        <div className="confirmation">
          <p>确认两台设备的短码一致</p>
          <div className="code">{task.code}</div>
          <div className="actions">
            <button className="button primary" onClick={() => void accept()}>{sending ? '一致，继续发送' : '一致，接收'}</button>
            <button className="button" onClick={() => void decline()}>{sending ? '取消' : '拒绝'}</button>
          </div>
          <small>两端确认后开始 · 60 秒内有效</small>
        </div>
      )}

      {!needsConfirm && !done && (
        <div className="progress-row">
          <div className="progress-copy">
            <span>{formatSize(task.transferredBytes)} / {formatSize(task.totalBytes)}</span>
            <button className="text-button" onClick={() => void run('cancel_transfer', { id })}>取消</button>
          </div>
          <progress value={task.transferredBytes} max={task.totalBytes || 1} />
        </div>
      )}

      {task.error && <p className="error-text">{task.error}</p>}
      {task.savedPath && (
        <button className="text-button open-action" onClick={() => void run('open_received', { path: task.savedPath })}>
          <Icon name="folder" />打开收到的文件
        </button>
      )}
      {task.text !== null && (
        <div className="received-text">
          <pre>{task.text}</pre>
          <button className="button" onClick={() => void run('copy_text', { text: task.text }, '已复制')}>复制文字</button>
        </div>
      )}
    </article>
  );
}
