import { useEffect, useRef, useState } from "react";
import { listAttributeGroups } from "../../../lib/api/attributeGroups";
import type { AttributeGroup } from "../../attributes/types";
import { getProduct } from "../../../lib/api/products";
import { useProducts } from "../hooks/useProducts";
import type { Product } from "../types";
import { ProductForm } from "./ProductForm";
import { ProductDetail } from "./ProductDetail";
import { BatchExport } from "./BatchExport";
import { ShopifyExport } from "./ShopifyExport";

const yen = new Intl.NumberFormat("ja-JP", { style: "currency", currency: "JPY" });
const date = new Intl.DateTimeFormat("ja-JP", { dateStyle: "medium", timeStyle: "short" });
export function ProductList({ groupRevision, onBatchChange }: { groupRevision: number; onBatchChange: (active: boolean) => void }) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [groupFilter, setGroupFilter] = useState("");
  const [batchIds, setBatchIds] = useState<string[] | null>(null);
  const [shopify, setShopify] = useState(false);
  const batchLock = useRef(false);
  const [groups, setGroups] = useState<AttributeGroup[]>([]);
  const [groupReload, setGroupReload] = useState(0);
  const { products, loading, error, refresh } = useProducts();
  const [editor, setEditor] = useState<{ product: Product | null } | null>(null);
  const [detail, setDetail] = useState<Product | null>(null);
  const [editing, setEditing] = useState(false);
  const [editError, setEditError] = useState("");
  const [notice, setNotice] = useState("");
  const [warning, setWarning] = useState("");
  const editLock = useRef(false);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    let active = true;
    listAttributeGroups().then(items => {
      if (active) {
        setGroups(items);
        setGroupFilter(current => items.some(group => group.id === current) ? current : "");
      }
    }).catch(() => {
      if (active) { setGroups([]); setGroupFilter(""); }
    });
    return () => { active = false; };
  }, [groupRevision, groupReload, editor, detail]);
  useEffect(() => {
    const ids = new Set(products.map(p => p.id));
    setSelected(previous => {
      const next = new Set([...previous].filter(id => ids.has(id)));
      return next.size === previous.size ? previous : next;
    });
  }, [products]);
  const visible = products.filter(p => !groupFilter || p.group_id === groupFilter);
  const selectedIds = products.filter(p => selected.has(p.id)).map(p => p.id);
  function selectVisible(checked: boolean) {
    if (batchLock.current || editLock.current || loading) return;
    setSelected(previous => {
      const next = new Set(previous);
      for (const product of visible) { if (checked) next.add(product.id); else next.delete(product.id); }
      return next;
    });
  }
  function startBatch(forShopify = false) {
    if (batchLock.current || editLock.current || loading || error || selectedIds.length === 0) return;
    batchLock.current = true;
    setShopify(forShopify);
    setBatchIds([...selectedIds]); onBatchChange(true);
  }
  const groupName = (id: string) => groups.find(g => g.id === id)?.name ?? "グループ情報を取得できません";
  async function edit(id: string, showDetail = false) {
    if (editLock.current || batchLock.current) return;
    editLock.current = true; setEditing(true); setEditError(""); setNotice("");
    try {
      const product = await getProduct(id);
      if (!mounted.current) return;
      if (showDetail) setDetail(product); else setEditor({ product });
    }
    catch (error: unknown) { if (mounted.current) setEditError(error instanceof Error ? error.message : "商品を取得できませんでした。"); }
    finally { if (mounted.current) setEditing(false); editLock.current = false; }
  }
  if (batchIds) {
    const Export = shopify ? ShopifyExport : BatchExport;
    return <Export productIds={batchIds} onBack={() => {
      batchLock.current = false; setBatchIds(null); onBatchChange(false);
    }} />;
  }
  if (editor) return <ProductForm key={editor.product?.id ?? "new"} product={editor.product} onCancel={() => setEditor(null)} onSaved={() => {
    setEditor(null); setDetail(null); setNotice("商品を保存しました。"); void refresh();
  }} />;
  if (detail) return <ProductDetail key={detail.id} product={detail} groups={groups} onBack={() => setDetail(null)} onEdit={() => setEditor({ product: detail })}
    onDeleted={report => {
      setDetail(null); setNotice("商品を削除しました。");
      setWarning(report.pending_quarantine_count ? `管理画像${report.pending_quarantine_count}枚の隔離が完了していません。ファイルは保持され、次回起動時に再確認されます。` : "");
      void refresh();
    }} />;
  return <section className="panel" aria-labelledby="products-heading" aria-busy={loading || editing}>
    <div className="section-heading"><div><h2 id="products-heading">商品管理</h2><p className="muted">登録した商品はこのPCに保存されます。</p></div>
      <button type="button" disabled={editing} onClick={() => { if (batchLock.current || editLock.current) return; setEditor({ product: null }); setNotice(""); setEditError(""); }}>商品を登録</button></div>
    <div className="batch-controls">
      <label htmlFor="product-group-filter">所属グループ</label>
      <select id="product-group-filter" value={groupFilter} disabled={editing || loading} onChange={event => { if (!batchLock.current && !editLock.current) setGroupFilter(event.target.value); }}>
        <option value="">すべてのグループ</option>
        {groups.map(group => <option key={group.id} value={group.id}>{group.name}{group.archivedAt ? "（非表示）" : ""}</option>)}
      </select>
      <p role="status">選択 {selectedIds.length}件</p>
      <button type="button" className="secondary" disabled={loading || editing || !!error || visible.length === 0} onClick={() => selectVisible(true)}>表示中をすべて選択</button>
      <button type="button" className="secondary" disabled={loading || editing || !!error || visible.length === 0} onClick={() => selectVisible(false)}>表示中の選択を解除</button>
      <button type="button" className="secondary" disabled={loading || editing || selectedIds.length === 0} onClick={() => { if (!batchLock.current && !editLock.current) setSelected(new Set()); }}>すべての選択を解除</button>
      <button type="button" disabled={loading || editing || !!error || selectedIds.length === 0} onClick={() => startBatch()}>選択した{selectedIds.length}件を一括エクスポート</button>
      <button type="button" disabled={loading || editing || !!error || selectedIds.length === 0 || selectedIds.length > 20} onClick={() => startBatch(true)}>選択商品をShopify用CSVで出力（{selectedIds.length}件）</button>
      {selectedIds.length > 20 && <p className="warning">Shopify用CSVは20商品まで選択できます。</p>}
    </div>
    {notice && <p className="success" role="status">{notice}</p>}
    {warning && <p className="warning" role="alert">{warning}</p>}
    {(error || editError) && <p className="error" role="alert">{error || editError}</p>}
    {loading ? <p role="status">商品を読み込んでいます…</p> : error ? null : products.length === 0 ?
      <div className="empty"><h3>商品はまだ登録されていません</h3><p>「商品を登録」から最初の商品を追加しましょう。</p></div> :
      <div className="table-scroll"><table><caption className="sr-only">登録商品一覧</caption>
        <thead><tr><th scope="col">選択</th><th scope="col">SKU</th><th scope="col">商品名</th><th scope="col">グループ</th><th scope="col">ブランド</th><th scope="col">価格</th><th scope="col">更新日時</th><th scope="col">操作</th></tr></thead>
        <tbody>{visible.map(product => <tr key={product.id}><td><input type="checkbox" aria-label={`${product.name}を選択`} checked={selected.has(product.id)} disabled={editing || loading} onChange={event => {
          if (batchLock.current || editLock.current) return;
          const checked = event.target.checked;
          setSelected(previous => { const next = new Set(previous); if (checked) next.add(product.id); else next.delete(product.id); return next; });
        }} /></td><td>{product.sku}</td><td><button type="button" className="product-link" disabled={editing} onClick={() => void edit(product.id, true)} aria-label={`${product.name}の詳細`}>{product.name}</button></td><td>{groupName(product.group_id)}</td><td>{product.brand ?? "—"}</td>
          <td className="price">{product.price_yen === null ? "未設定" : yen.format(product.price_yen)}</td>
          <td><time dateTime={product.updated_at}>{date.format(new Date(product.updated_at))}</time></td>
          <td><button type="button" className="secondary" disabled={editing} onClick={() => void edit(product.id)} aria-label={`${product.name}を編集`}>編集</button></td></tr>)}</tbody>
      </table></div>}
    {editing && <p role="status">商品を読み込んでいます…</p>}
    <button type="button" className="secondary refresh" disabled={loading || editing} onClick={() => { if (batchLock.current || editLock.current) return; setGroupReload(value => value + 1); void refresh(); }}>一覧を再読み込み</button>
  </section>;
}
