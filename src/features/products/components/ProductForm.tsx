import { useEffect, useRef, useState, type FormEvent } from "react";
import { createProduct, updateProduct, getProductEditSnapshot } from "../../../lib/api/products";
import { RequestError } from "../../../lib/api/request";
import { listAttributeDefinitions } from "../../../lib/api/attributes";
import type { AttributeDefinition } from "../../attributes/types";
import type { Product } from "../types";

type Props = { product: Product | null; onSaved: () => void; onCancel: () => void };
export function ProductForm({ product, onSaved, onCancel }: Props) {
  const [sku, setSku] = useState(product?.sku ?? "");
  const [name, setName] = useState(product?.name ?? "");
  const [brand, setBrand] = useState(product?.brand ?? "");
  const [price, setPrice] = useState(product?.price_yen?.toString() ?? "");
  const [note, setNote] = useState(product?.description_note ?? "");
  const [listingTitle, setListingTitle] = useState(product?.listing_title ?? "");
  const [listingDescription, setListingDescription] = useState(product?.listing_description ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const submitting = useRef(false);
  const [expectedUpdatedAt, setExpectedUpdatedAt] = useState(product?.updated_at ?? "");
  const [definitions, setDefinitions] = useState<AttributeDefinition[]>([]);
  const [values, setValues] = useState<Record<string, string>>({});
  const [ready, setReady] = useState(false);
  const [loadError, setLoadError] = useState("");
  const [conflicted, setConflicted] = useState(false);
  useEffect(() => {
    let active = true;
    async function load() {
      try {
        if (product) {
          const snapshot = await getProductEditSnapshot(product.id);
          if (!active) return;
          const p = snapshot.product;
          setSku(p.sku); setName(p.name); setBrand(p.brand ?? "");
          setPrice(p.price_yen?.toString() ?? ""); setNote(p.description_note ?? "");
          setListingTitle(p.listing_title ?? ""); setListingDescription(p.listing_description ?? "");
          setExpectedUpdatedAt(p.updated_at); setDefinitions(snapshot.definitions.filter(a => !a.archivedAt));
          setValues(Object.fromEntries(snapshot.values.map(v => [v.attributeId, v.value])));
        } else {
          const items = await listAttributeDefinitions();
          if (!active) return;
          setDefinitions(items.filter(a => !a.archivedAt));
        }
        setReady(true);
      } catch (error: unknown) {
        if (active) setLoadError(error instanceof Error ? error.message : "カスタム項目を取得できませんでした。");
      }
    }
    void load();
    return () => { active = false; };
  }, [product]);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting.current || !ready || conflicted) return;
    setError("");
    if (!sku.trim() || !name.trim()) { setError("SKUと商品名を入力してください。"); return; }
    const priceText = price.trim();
    if (priceText && (!/^\d+$/.test(priceText) || !Number.isSafeInteger(Number(priceText)))) {
      setError("価格は0〜9,007,199,254,740,991円の整数で入力してください。"); return;
    }
    submitting.current = true; setSaving(true);
    try {
      const input = { sku, name, brand: brand || null, price_yen: priceText ? Number(priceText) : null, description_note: note || null,
        listing_title: listingTitle || null, listing_description: listingDescription || null };
      const attributeValues = {
        editableDefinitions: definitions.map(a => ({ attributeId: a.id, expectedUpdatedAt: a.updatedAt })),
        values: definitions.map(a => ({ attributeId: a.id, value: values[a.id] ?? "" })),
      };
      if (product) await updateProduct({ id: product.id, input, expectedUpdatedAt, attributeValues }); else await createProduct(input, attributeValues);
      onSaved();
    } catch (error: unknown) {
      if (error instanceof RequestError && error.code === "stale_data") setConflicted(true);
      setError(error instanceof RequestError && error.code === "stale_data"
        ? "商品またはカスタム項目が更新されています。入力内容は保持しています。保存されていません。内容を控えてキャンセルし、最新情報を開き直してください。"
        : error instanceof Error ? error.message : "保存できませんでした。");
    }
    finally { submitting.current = false; setSaving(false); }
  }
  return <section className="product-form panel" aria-labelledby="form-heading">
    <h2 id="form-heading">{product ? "商品を編集" : "商品を登録"}</h2>
    <p className="muted">必須項目はSKUと商品名です。価格は日本円で入力してください。</p>
    {!ready && !loadError && <p role="status">商品・カスタム項目を読み込んでいます…</p>}
    {loadError && <p className="error" role="alert">{loadError} 保存できません。キャンセルして開き直してください。</p>}
    <form onSubmit={submit} aria-busy={saving}><fieldset disabled={saving || !ready}>
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
      <textarea id="note" rows={5} value={note} onChange={e => setNote(e.target.value)} aria-describedby="note-help" />
      <small id="note-help">内部管理用・公開用説明文には自動転記されません。</small>
      <h3>EC掲載用の文章</h3>
      <label htmlFor="listing-title">公開用タイトル</label>
      <input id="listing-title" value={listingTitle} onChange={e => setListingTitle(e.target.value)} aria-describedby="listing-title-help listing-title-status" />
      <small id="listing-title-help">任意・200文字以内。改行・制御文字は使えません。</small>
      <small id="listing-title-status">{listingTitle.trim() ? "公開用タイトルを設定しています。" : "未設定。未入力の場合、将来の出品データ生成では管理用の商品名を候補として使用します。"}</small>
      <label htmlFor="listing-description">公開用説明文</label>
      <textarea id="listing-description" rows={8} value={listingDescription} onChange={e => setListingDescription(e.target.value)} aria-describedby="listing-description-help" />
      <small id="listing-description-help">任意・10,000文字以内。複数行とタブを使用できます。プレーンテキストとして保存します。</small>
      <h3>カスタム商品項目</h3>
      {ready && definitions.length === 0 && <p className="muted">カスタム項目設定から項目を追加できます。</p>}
      {definitions.map(a => <div key={a.id}>
        <label htmlFor={`attribute-${a.id}`}>{a.name}{a.unit ? `（${a.unit}）` : ""}</label>
        <input id={`attribute-${a.id}`} type="text" inputMode={a.dataType === "number" ? "decimal" : "text"}
          value={values[a.id] ?? ""} onChange={e => setValues(current => ({ ...current, [a.id]: e.target.value }))} />
        <small>{a.dataType === "number" ? "任意・整数15桁、小数6桁以内。負数も入力できます。" : "任意・2,000文字以内。改行は使えません。"}</small>
      </div>)}
      {error && <p className="error" role="alert">{error}</p>}
    </fieldset>
    <div className="actions">
      <button type="submit" disabled={saving || !ready || conflicted}>{saving ? "保存中…" : "保存する"}</button>
      <button type="button" className="secondary" disabled={saving} onClick={() => { if (!submitting.current) onCancel(); }}>キャンセル</button>
    </div></form>
  </section>;
}
