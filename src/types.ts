export type ContentKind = 'files' | 'text';
export type Device = { id: string; name: string; platform: string; fingerprint: string; signingKey: string; protocol: number };
export type Peer = { device: Device; address: string; discovered: boolean };
export type Selection = { id: string; title: string; count: number; totalBytes: number; entries: { path: string; size: number; directory: boolean }[] };
export type Transfer = {
  id: string; direction: 'send' | 'receive'; peerName: string; title: string; kind: ContentKind;
  status: string; code: string; totalBytes: number; transferredBytes: number; count: number;
  error: string | null; savedPath: string | null; text: string | null;
};
export type History = Omit<Transfer, 'code' | 'transferredBytes' | 'count' | 'error' | 'text'> & { time: number };
export type Snapshot = { device: Device; addresses: string[]; receiveDir: string; receiveDirWarning: string | null; peers: Peer[]; transfers: Transfer[]; history: History[]; networkWarning: string | null };
