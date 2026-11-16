import { Fragment, useEffect, useState } from "react";
import { getListingPreview } from "../../../lib/api/listingPreview";
import type { ListingPreview as Preview, ListingPreviewImage } from "../listingPreviewTypes";

const statusLabels = { ready: "準備完了", warning: "要確認", blocked: "出力不可" };
const yen = new Intl.NumberFormat("ja-JP", { style: "currency", currency: "JPY" });

function PreviewImage({ image, main }: { image: ListingPreviewImage; main: boolean }) {
  const [failed, setFailed] = useState(false);
  return <figure className={main ? "listing-image listing-main-image" : "listing-image"}>
    {image.previewUrl && !failed ? <img src={image.previewUrl} alt={main ? "メイン商品画像" : "商品画像"} onError={() => setFailed(true)} /> :
      <p className="error" role="status">画像を表示できません。再読み込みして状態を確認してください。</p>}
    <figcaption>{main ? "メイン画像" : `画像 ${image.sortOrder + 1}`}</figcaption>
  </figure>;
}

export function ListingPreview({ productId, onBack, onEdit }: { productId: string; onBack: () => void; onEdit: () => void }) {
  const [result, setResult] = useState<Preview | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let active = true;
    void getListingPreview(productId).then(value => { if (active) setResult(value); })
      .catch((cause: unknown) => { if (active) setError(cause instanceof Error ? cause.message : "プレビューを取得できませんでした。"); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [productId, revision]);
  function refresh() {
    setResult(null); setError(""); setLoading(true); setRevision(value => value + 1);
  }
  return <section className="panel" aria-labelledby="listing-heading" aria-busy={loading}>
    <div className="section-heading"><h2 id="listing-heading">出品プレビュー</h2>
      <div className="detail-actions">
        <button type="button" className="secondary" onClick={onBack}>商品詳細へ戻る</button>
        <button type="button" onClick={onEdit}>商品を編集</button>
        <button type="button" className="secondary" disabled={loading} onClick={refresh}>プレビューを再読み込み</button>
      </div>
    </div>
    <p className="muted">取得時点の確認用スナップショットです。すべてのECサイトでの出品を保証するものではありません。変更後は再読み込みしてください。</p>
    {loading ? <p role="status">プレビューを読み込んでいます…</p> : error ? <p className="error" role="alert">{error} 再読み込みしてください。</p> : result ? <>
      <h3>準備状況：{statusLabels[result.status]}</h3>
      {result.issues.length > 0 && <ul aria-label="確認事項">{result.issues.map(issue =>
        <li key={issue.code} className={issue.severity === "error" ? "error" : "warning"}>
          {issue.severity === "error" ? "エラー" : "警告"}：{issue.message}
        </li>)}</ul>}
      <dl className="product-information">
        <dt>SKU</dt><dd>{result.sku}</dd>
        <dt>出品候補タイトル</dt><dd>{result.title}</dd>
        <dt>タイトルの出典</dt><dd>{result.titleSource === "listing_title" ? "公開用タイトル" : "管理用の商品名（候補）"}</dd>
        <dt>公開用説明文</dt><dd className="product-note">{result.description ?? "未設定"}</dd>
        <dt>価格</dt><dd>{result.priceYen === null ? "未設定" : yen.format(result.priceYen)}</dd>
      </dl>
      <h3>カスタム商品項目</h3>
      {result.attributes.length === 0 ? <p>設定済みの項目はありません。</p> : <dl className="product-information">
        {result.attributes.map(attribute => <Fragment key={attribute.attributeId}><dt>{attribute.name}</dt>
          <dd>{attribute.value}{attribute.unit ? ` ${attribute.unit}` : ""}</dd></Fragment>)}
      </dl>}
      <h3>商品画像</h3>
      {result.images.length === 0 ? <p>画像は登録されていません。</p> : <div className="listing-images">
        {result.images.map((image, index) => <PreviewImage key={`${revision}:${image.imageId}`} image={image} main={index === 0} />)}
      </div>}
    </> : null}
  </section>;
}
