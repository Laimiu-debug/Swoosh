import { useState } from 'react';
import type { FormEvent } from 'react';
import type { Run } from '../types';
import { Modal } from './Modal';

type Props = { addresses: string[]; error: string; run: Run; onConnected: () => void; onClose: () => void };

export function ConnectDialog({ addresses, error, run, onConnected, onClose }: Props) {
  const [address, setAddress] = useState('');
  const [busy, setBusy] = useState(false);

  async function connect(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    const peer = await run('connect_device', { address }, '已连接，选好内容就能发送');
    setBusy(false);
    if (peer) onConnected();
  }

  return (
    <Modal title="连接设备" onClose={onClose}>
      <p className="muted">两台电脑不用扫码，直接输入对方的地址。</p>
      <div className="local-addresses">
        <label>这台电脑的地址</label>
        {addresses.length
          ? addresses.map(value => (
            <div className="address-row" key={value}>
              <code>{value}</code>
              <button className="text-button" onClick={() => void run('copy_text', { text: value }, '已复制')}>复制</button>
            </div>
          ))
          : <p className="muted">没有可用的局域网地址，请检查网络连接。</p>}
      </div>
      <form onSubmit={event => void connect(event)}>
        <label htmlFor="address">另一台设备的地址</label>
        <input id="address" value={address} onChange={event => setAddress(event.target.value)} placeholder="192.168.1.8:53318" autoComplete="off" required />
        <p className="field-hint">在对方的「连接设备」里查看。</p>
        {error && <p className="error-text" role="alert">{error}</p>}
        <div className="actions dialog-actions">
          <button className="button" type="button" onClick={onClose}>关闭</button>
          <button className="button primary" type="submit" disabled={busy || !address.trim()}>{busy ? '连接中…' : '连接'}</button>
        </div>
      </form>
    </Modal>
  );
}
