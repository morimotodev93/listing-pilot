import { useState } from "react";
import type { Product } from "../types";
import { ProductImages } from "./ProductImages";

type Props = { product: Product; onBack: () => void; onEdit: () => void };
const yen = new Intl.NumberFormat("ja-JP", { style: "currency", currency: "JPY" });
const date = new Intl.DateTimeFormat("ja-JP", { dateStyle: "medium", timeStyle: "short" });

export function ProductDetail({ product, onBack, onEdit }: Props) {
  const [importing, setImporting] = useState(false);
  return <section className="panel" aria-labelledby="detail-heading">
    <div className="section-heading">
      <div><h2 id="detail-heading">{product.name}</h2><p className="muted">商品詳細</p></div>
      <div className="detail-actions">
        <button type="button" className="secondary" disabled={importing} onClick={onBack}>一覧へ戻る</button>
        <button type="button" disabled={importing} onClick={onEdit}>商品を編集</button>
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
    <ProductImages key={product.id} productId={product.id} onBusyChange={setImporting} />
  </section>;
}
