import { useRef, useState, type FormEvent } from "react";
import { createProduct, updateProduct } from "../../../lib/api/products";
import { RequestError } from "../../../lib/api/request";
import type { Product } from "../types";

type Props = { product: Product | null; onSaved: () => void; onCancel: () => void };
export function ProductForm({ product, onSaved, onCancel }: Props) {
  const [sku, setSku] = useState(product?.sku ?? "");
  const [name, setName] = useState(product?.name ?? "");
  const [brand, setBrand] = useState(product?.brand ?? "");
  const [price, setPrice] = useState(product?.price_yen?.toString() ?? "");
  const [note, setNote] = useState(product?.description_note ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const submitting = useRef(false);
  const [expectedUpdatedAt] = useState(product?.updated_at ?? "");
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting.current) return;
    setError("");
    if (!sku.trim() || !name.trim()) { setError("SKUと商品名を入力してください。"); return; }
    const priceText = price.trim();
    if (priceText && (!/^\d+$/.test(priceText) || !Number.isSafeInteger(Number(priceText)))) {
      setError("価格は0〜9,007,199,254,740,991円の整数で入力してください。"); return;
    }
    submitting.current = true; setSaving(true);
    try {
      const input = { sku, name, brand: brand || null, price_yen: priceText ? Number(priceText) : null, description_note: note || null };
      if (product) await updateProduct({ id: product.id, input, expectedUpdatedAt }); else await createProduct(input);
      onSaved();
    } catch (error: unknown) {
      setError(error instanceof RequestError && error.code === "stale_data"
        ? "この商品は別の操作で更新されています。現在の編集内容は保存されませんでした。最新の商品情報を確認してください。"
        : error instanceof Error ? error.message : "保存できませんでした。");
    }
    finally { submitting.current = false; setSaving(false); }
  }
  return <section className="product-form panel" aria-labelledby="form-heading">
    <h2 id="form-heading">{product ? "商品を編集" : "商品を登録"}</h2>
    <p className="muted">必須項目はSKUと商品名です。価格は日本円で入力してください。</p>
    <form onSubmit={submit} aria-busy={saving}><fieldset disabled={saving}>
      <label htmlFor="sku">SKU <span>必須</span></label>
      <input id="sku" value={sku} onChange={e => setSku(e.target.value)} required autoFocus aria-describedby="sku-help" />
      <small id="sku-help">前後の空白は除去します。大文字と小文字は区別します。</small>
      <label htmlFor="name">商品名 <span>必須</span></label>
      <input id="name" value={name} onChange={e => setName(e.target.value)} required />
      <label htmlFor="brand">ブランド</label>
      <input id="brand" value={brand} onChange={e => setBrand(e.target.value)} />
      <label htmlFor="price">価格（円）</label>
      <input id="price" inputMode="numeric" value={price} onChange={e => setPrice(e.target.value)} aria-describedby="price-help" />
      <small id="price-help">任意・0以上の整数。未入力は価格未設定になります。</small>
      <label htmlFor="note">説明メモ</label>
      <textarea id="note" rows={5} value={note} onChange={e => setNote(e.target.value)} />
      {error && <p className="error" role="alert">{error}</p>}
      <div className="actions"><button type="submit">{saving ? "保存中…" : "保存する"}</button>
        <button type="button" className="secondary" onClick={onCancel}>キャンセル</button></div>
    </fieldset></form>
  </section>;
}
