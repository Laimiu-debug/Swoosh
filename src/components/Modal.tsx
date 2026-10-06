import { useEffect, useRef } from 'react';
import type { MouseEvent, ReactNode } from 'react';
import { Icon } from './Icon';

type Props = { title: string; onClose: () => void; children: ReactNode };

/** Native modal dialog that closes on Escape or backdrop click and restores focus on unmount. */
export function Modal({ title, onClose, children }: Props) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current!;
    const opener = document.activeElement as HTMLElement | null;
    dialog.showModal();
    return () => { dialog.close(); opener?.focus(); };
  }, []);

  function closeOnBackdrop(event: MouseEvent<HTMLDialogElement>) {
    if (event.target !== event.currentTarget) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const inside = event.clientX >= rect.left && event.clientX <= rect.right && event.clientY >= rect.top && event.clientY <= rect.bottom;
    if (!inside) onClose();
  }

  return (
    <dialog ref={ref} onCancel={event => { event.preventDefault(); onClose(); }} onClick={closeOnBackdrop}>
      <div className="section-heading">
        <h2>{title}</h2>
        <button className="icon-button" onClick={onClose} aria-label="关闭"><Icon name="close" /></button>
      </div>
      {children}
    </dialog>
  );
}
