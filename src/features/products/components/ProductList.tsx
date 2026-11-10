import { useRef, useState } from "react";
import { getProduct } from "../../../lib/api/products";
import { useProducts } from "../hooks/useProducts";
import type { Product } from "../types";
import { ProductForm } from "./ProductForm";
import { ProductDetail } from "./ProductDetail";

const yen = new Intl.NumberFormat("ja-JP", { style: "currency", currency: "JPY" });
const date = new Intl.DateTimeFormat("ja-JP", { dateStyle: "medium", timeStyle: "short" });
export function ProductList() {
  const { products, loading, error, refresh } = useProducts();
  const [editor, setEditor] = useState<{ product: Product | null } | null>(null);
  const [detail, setDetail] = useState<Product | null>(null);
  const [editing, setEditing] = useState(false);
  const [editError, setEditError] = useState("");
  const [notice, setNotice] = useState("");
  const [warning, setWarning] = useState("");
  const editLock = useRef(false);
  async function edit(id: string, showDetail = false) {
    if (editLock.current) return;
    editLock.current = true; setEditing(true); setEditError(""); setNotice("");
    try {
      const product = await getProduct(id);
      if (showDetail) setDetail(product); else setEditor({ product });
    }
    catch (error: unknown) { setEditError(error instanceof Error ? error.message : "商品を取得できませんでした。"); }
    finally { setEditing(false); editLock.current = false; }
  }
  if (editor) return <ProductForm key={editor.product?.id ?? "new"} product={editor.product} onCancel={() => setEditor(null)} onSaved={() => {
    setEditor(null); setDetail(null); setNotice("商品を保存しました。"); void refresh();
  }} />;
  if (detail) return <ProductDetail key={detail.id} product={detail} onBack={() => setDetail(null)} onEdit={() => setEditor({ product: detail })}
    onDeleted={report => {
      setDetail(null); setNotice("商品を削除しました。");
      setWarning(report.pending_quarantine_count ? `管理画像${report.pending_quarantine_count}枚の隔離が完了していません。ファイルは保持され、次回起動時に再確認されます。` : "");
      void refresh();
    }} />;
  return <section className="panel" aria-labelledby="products-heading" aria-busy={loading || editing}>
    <div className="section-heading"><div><h2 id="products-heading">商品管理</h2><p className="muted">登録した商品はこのPCに保存されます。</p></div>
      <button type="button" disabled={editing} onClick={() => { setEditor({ product: null }); setNotice(""); setEditError(""); }}>商品を登録</button></div>
    {notice && <p className="success" role="status">{notice}</p>}
    {warning && <p className="warning" role="alert">{warning}</p>}
    {(error || editError) && <p className="error" role="alert">{error || editError}</p>}
    {loading ? <p role="status">商品を読み込んでいます…</p> : error ? null : products.length === 0 ?
      <div className="empty"><h3>商品はまだ登録されていません</h3><p>「商品を登録」から最初の商品を追加しましょう。</p></div> :
      <div className="table-scroll"><table><caption className="sr-only">登録商品一覧</caption>
        <thead><tr><th scope="col">SKU</th><th scope="col">商品名</th><th scope="col">ブランド</th><th scope="col">価格</th><th scope="col">更新日時</th><th scope="col">操作</th></tr></thead>
        <tbody>{products.map(product => <tr key={product.id}><td>{product.sku}</td><td><button type="button" className="product-link" disabled={editing} onClick={() => void edit(product.id, true)} aria-label={`${product.name}の詳細`}>{product.name}</button></td><td>{product.brand ?? "—"}</td>
          <td className="price">{product.price_yen === null ? "未設定" : yen.format(product.price_yen)}</td>
          <td><time dateTime={product.updated_at}>{date.format(new Date(product.updated_at))}</time></td>
          <td><button type="button" className="secondary" disabled={editing} onClick={() => void edit(product.id)} aria-label={`${product.name}を編集`}>編集</button></td></tr>)}</tbody>
      </table></div>}
    {editing && <p role="status">商品を読み込んでいます…</p>}
    <button type="button" className="secondary refresh" disabled={loading || editing} onClick={() => void refresh()}>一覧を再読み込み</button>
  </section>;
}
