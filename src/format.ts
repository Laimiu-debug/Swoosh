import type { TransferStatus } from './types';

/** Mirrors `TEXT_LIMIT` in swoosh-core. */
export const TEXT_LIMIT = 1024 * 1024;

const encoder = new TextEncoder();
export const byteLength = (text: string) => encoder.encode(text).length;

export const statusLabels: Record<TransferStatus, string> = {
  connecting: '正在连接', awaiting_confirmation: '核对短码', awaiting_sender: '等待对方确认',
  awaiting_receiver: '等待对方接收', transferring: '正在传输', verifying: '正在校验',
  completed: '已完成', failed: '传输失败', cancelled: '已取消', rejected: '已拒绝',
};

const terminal = new Set<TransferStatus>(['completed', 'failed', 'cancelled', 'rejected']);
export const isTerminal = (status: TransferStatus) => terminal.has(status);

const units = ['KB', 'MB', 'GB', 'TB'];
export function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  let value = bytes / 1024, unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++; }
  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
}

/** Strips Windows extended-length prefixes (`\\?\`, `\\?\UNC\`) for display. */
export function displayPath(path: string) {
  return path.replace(/^\\\\\?\\UNC\\/, '\\\\').replace(/^\\\\\?\\/, '');
}

export function formatTime(seconds: number) {
  return new Date(seconds * 1000).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}
