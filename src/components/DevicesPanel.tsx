import type { Peer } from '../types';
import { Icon } from './Icon';

const platformNames: Record<string, string> = { windows: 'Windows', macos: 'macOS', linux: 'Linux', android: 'Android', ios: 'iOS' };
const isMobile = (platform: string) => platform === 'android' || platform === 'ios';

type Props = { peers: Peer[]; canSend: boolean; native: boolean; onSend: (peer: Peer) => void; onConnect: () => void };

export function DevicesPanel({ peers, canSend, native, onSend, onConnect }: Props) {
  return (
    <section className="panel devices-panel">
      <div className="section-heading">
        <h2>发给谁</h2>
        <span className="muted">{peers.length ? `${peers.length} 台设备` : '附近设备'}</span>
      </div>
      {peers.length ? (
        <div className="devices">
          {peers.map(peer => {
            const { id, name, platform } = peer.device;
            return (
              <button className="device-card" key={id} disabled={!canSend} onClick={() => onSend(peer)} title={canSend ? `发送给 ${name}` : '请先选择内容'}>
                <span className="device-icon"><Icon name={isMobile(platform) ? 'phone' : 'desktop'} /></span>
                <span className="flex-copy">
                  <strong>{name}</strong>
                  <span>{platformNames[platform] ?? platform} · {peer.discovered ? '附近' : '手动连接'}</span>
                </span>
                <Icon name="arrow" />
              </button>
            );
          })}
        </div>
      ) : (
        <div className="empty-devices">
          <div className="device-orbit"><Icon name="desktop" /><span /><Icon name="phone" /></div>
          <h3>附近还没有设备</h3>
          <p>让另一台设备也打开 Swoosh，<br />连接同一个 Wi-Fi 或局域网。</p>
          <button className="text-button" onClick={onConnect} disabled={!native}>没找到？手动连接<Icon name="arrow" /></button>
        </div>
      )}
    </section>
  );
}
