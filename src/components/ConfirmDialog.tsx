import { useEffect, useRef, type ReactNode } from "react";

type Props = { title: string; children: ReactNode; confirmLabel: string; onConfirm: () => void; onCancel: () => void };

export function ConfirmDialog({ title, children, confirmLabel, onConfirm, onCancel }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const confirming = useRef(false);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  return <dialog ref={dialog} className="confirm-dialog" aria-labelledby="confirm-dialog-title"
    onCancel={event => { event.preventDefault(); onCancel(); }}>
    <h3 id="confirm-dialog-title">{title}</h3>
    {children}
    <div className="actions">
      <button type="button" className="secondary" autoFocus onClick={onCancel}>キャンセル</button>
      <button type="button" onClick={() => {
        if (confirming.current) return;
        confirming.current = true;
        onConfirm();
      }}>{confirmLabel}</button>
    </div>
  </dialog>;
}
