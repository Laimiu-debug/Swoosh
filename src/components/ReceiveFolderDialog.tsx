import { useState } from 'react';
import type { Run } from '../types';
import { displayPath } from '../format';
import { Icon } from './Icon';
import { Modal } from './Modal';

type Props = { receiveDir: string; error: string; run: Run; notify: (message: string) => void; onClose: () => void };

export function ReceiveFolderDialog({ receiveDir, error, run, notify, onClose }: Props) {
  const [saving, setSaving] = useState(false);

  async function change(reset: boolean) {
    setSaving(true);
    try {
      // `choose_receive_dir` resolves to null when the picker is dismissed; no notice then.
      const path = await run<string | null>(reset ? 'reset_receive_dir' : 'choose_receive_dir');
      if (path) notify(reset ? '已恢复默认接收位置' : '接收位置已保存');
    } finally { setSaving(false); }
  }

  return (
    <Modal title="接收文件夹" onClose={onClose}>
      <p className="muted">收到的文件，放在你喜欢的位置。</p>
      <div className="receive-location">
        <span className="muted">当前保存位置</span>
        <p title={receiveDir}>{displayPath(receiveDir)}</p>
        <button className="text-button" disabled={saving} onClick={() => void run('open_received', { path: receiveDir })}><Icon name="folder" />打开文件夹</button>
      </div>
      <p className="receive-location-hint">修改后自动保存，下次打开仍会使用。当前接收继续保存到原位置，旧文件不会搬动。</p>
      {error && <p className="error-text" role="alert">{error}</p>}
      <div className="receive-location-buttons">
        <button className="button primary" disabled={saving} onClick={() => void change(false)}>{saving ? '正在处理…' : '选择文件夹'}</button>
        <button className="button" disabled={saving} onClick={() => void change(true)}>恢复默认</button>
      </div>
      <p className="field-hint">默认位置：系统下载目录下的 Swoosh 文件夹。</p>
    </Modal>
  );
}
