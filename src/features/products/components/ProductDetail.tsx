import { useEffect, useRef, useState } from "react";
import { deleteProduct, getProductDeletePreview } from "../../../lib/api/products";
import type { DeleteProductReport, ProductDeletePreview } from "../../../lib/api/products";
import type { Product } from "../types";
import { ProductImages } from "./ProductImages";

type Props = { product: Product; onBack: () => void; onEdit: () => void; onDeleted: (report: DeleteProductReport) => void };
const yen = new Intl.NumberFormat("ja-JP", { style: "currency", currency: "JPY" });
const date = new Intl.DateTimeFormat("ja-JP", { dateStyle: "medium", timeStyle: "short" });

export function ProductDetail({ product, onBack, onEdit, onDeleted }: Props) {
  const [importing, setImporting] = useState(false);
  const imageBusy = useRef(false);
  const operation = useRef(false);
  const [busy, setBusy] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [preview, setPreview] = useState<ProductDeletePreview | null>(null);
  const [error, setError] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);
  const locked = importing || busy || preview !== null;
  useEffect(() => {
    if (preview && !busy && !dialog.current?.open) dialog.current?.showModal();
  }, [preview, busy]);
  function cancel() {
    if (operation.current) return;
    dialog.current?.close(); setPreview(null);
  }
  async function askDelete() {
    if (operation.current || imageBusy.current || preview) return;
    operation.current = true; setBusy(true); setError("");
    try {
      setPreview(await getProductDeletePreview(product.id));
    } catch (error: unknown) {
      setError(error instanceof Error ? error.message : "削除内容を取得できませんでした。");
    } finally { operation.current = false; setBusy(false); }
  }
  async function confirmDelete() {
    if (operation.current || imageBusy.current || !preview) return;
    operation.current = true; setBusy(true); setDeleting(true); setError("");
    try {
      const report = await deleteProduct({ productId: preview.product_id, expectedUpdatedAt: preview.updated_at,
        expectedImageIds: preview.image_ids, expectedProduct: preview.product });
      dialog.current?.close(); setPreview(null); onDeleted(report);
    } catch (error: unknown) {
      dialog.current?.close(); setPreview(null);
      setError(error instanceof Error ? error.message : "商品を削除できませんでした。内容を確認し直してください。");
    } finally { operation.current = false; setBusy(false); setDeleting(false); }
  }
  return <section className="panel" aria-labelledby="detail-heading">
    <div className="section-heading">
      <div><h2 id="detail-heading">{product.name}</h2><p className="muted">商品詳細</p></div>
      <div className="detail-actions">
        <button type="button" className="secondary" disabled={locked} onClick={() => { if (!operation.current && !imageBusy.current && !preview) onBack(); }}>一覧へ戻る</button>
        <button type="button" disabled={locked} onClick={() => { if (!operation.current && !imageBusy.current && !preview) onEdit(); }}>商品を編集</button>
        <button type="button" className="secondary" disabled={locked} onClick={() => void askDelete()}>商品を削除</button>
      </div>
    </div>
    <dl className="product-information">
      <dt>SKU</dt><dd>{product.sku}</dd>
      <dt>ブランド</dt><dd>{product.brand ?? "未設定"}</dd>
      <dt>価格</dt><dd>{product.price_yen === null ? "未設定" : yen.format(product.price_yen)}</dd>
      <dt>説明メモ</dt><dd className="product-note">{product.description_note ?? "未設定"}</dd>
      <dt>登録日時</dt><dd><time dateTime={product.created_at}>{date.format(new Date(product.created_at))}</time></dd>
      <dt>更新日時</dt><dd><time dateTime={product.updated_at}>{date.format(new Date(product.updated_at))}</time></dd>
    </dl>
    {error && <p className="error" role="alert">{error}</p>}
    {busy && <p role="status">{deleting ? "商品を削除しています…" : "最新の削除内容を確認しています…"}</p>}
    <dialog ref={dialog} aria-labelledby="delete-product-heading" onCancel={event => { event.preventDefault(); cancel(); }}>
      <h3 id="delete-product-heading">商品を削除しますか？</h3>
      <dl><dt>商品名</dt><dd>{preview?.name}</dd><dt>SKU</dt><dd>{preview?.sku}</dd><dt>登録画像</dt><dd>{preview?.image_count}枚</dd></dl>
      <p>この商品と登録画像{preview?.image_count}枚を削除します。管理画像ファイルは保護領域に保持されますが、商品情報と画像の登録をアプリ内から元に戻すことはできません。取り込み元の画像ファイルは変更されません。</p>
      <div className="detail-actions"><button type="button" className="secondary" autoFocus disabled={busy} onClick={cancel}>キャンセル</button>
        <button type="button" disabled={busy || !preview} onClick={() => void confirmDelete()}>{deleting ? "削除中…" : "商品を削除"}</button></div>
    </dialog>
    <ProductImages key={product.id} productId={product.id} disabled={busy || preview !== null}
      isBlocked={() => operation.current || preview !== null}
      onBusyChange={value => { imageBusy.current = value; setImporting(value); }} />
  </section>;
}
