export type ContentKind = 'files' | 'text';
export type Direction = 'send' | 'receive';
export type TransferStatus =
  | 'connecting' | 'awaiting_confirmation' | 'awaiting_sender' | 'awaiting_receiver'
  | 'transferring' | 'verifying' | 'completed' | 'failed' | 'cancelled' | 'rejected';

export type Device = { id: string; name: string; platform: string; fingerprint: string; signingKey: string; protocol: number };
export type Peer = { device: Device; address: string; discovered: boolean };
export type Selection = { id: string; title: string; count: number; totalBytes: number; entries: { path: string; size: number; directory: boolean }[] };
export type Transfer = {
  id: string; direction: Direction; peerName: string; title: string; kind: ContentKind;
  status: TransferStatus; code: string; totalBytes: number; transferredBytes: number; count: number;
  error: string | null; savedPath: string | null; text: string | null;
};
export type History = Omit<Transfer, 'code' | 'transferredBytes' | 'count' | 'error' | 'text'> & { time: number };
export type Snapshot = { device: Device; addresses: string[]; receiveDir: string; receiveDirWarning: string | null; peers: Peer[]; transfers: Transfer[]; history: History[]; networkWarning: string | null };

/** Runs a native command; resolves to `null` on failure after surfacing the error. */
export type Run = <T = unknown>(command: string, args?: Record<string, unknown>, success?: string) => Promise<T | null>;
