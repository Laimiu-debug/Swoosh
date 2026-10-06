import type { ReactNode } from 'react';

const paths = {
  file: <><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" /><path d="M14 3v6h6M8 13h8M8 17h5" /></>,
  folder: <path d="M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />,
  desktop: <><rect x="2" y="3" width="20" height="14" rx="2" /><path d="M8 21h8M12 17v4" /></>,
  phone: <><rect x="6" y="2" width="12" height="20" rx="3" /><path d="M10 5h4M11 18h2" /></>,
  arrow: <path d="M4 12h16m-6-6 6 6-6 6" />,
  close: <path d="m6 6 12 12M6 18 18 6" />,
  check: <path d="m5 12 4 4L19 6" />,
  link: <path d="m9 15 6-6M14 7l2-2a4 4 0 0 1 6 6l-4 4M10 17l-2 2a4 4 0 0 1-6-6l4-4" />,
  moon: <path d="M20.8 13a9 9 0 0 1-9.8-9.8A9 9 0 1 0 20.8 13z" />,
  sun: <><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M2 12h2M20 12h2m-3-9-1 1M5 19l-1 1M4 4l1 1m14 14 1 1" /></>,
  text: <path d="M4 5h16M12 5v14M8 19h8" />,
  receive: <path d="M12 3v12m-5-5 5 5 5-5M4 16v4h16v-4" />,
  send: <path d="M12 15V3m-5 5 5-5 5 5M4 16v4h16v-4" />,
  history: <path d="M3 11a9 9 0 1 1 2.6 7M3 4v7h7M12 7v5l3 2" />,
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof paths;

export function Icon({ name, className = '' }: { name: IconName; className?: string }) {
  return (
    <svg className={`icon ${className}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {paths[name]}
    </svg>
  );
}
