import { Fragment, useEffect, useRef, useState } from "react";
import { getListingPreview } from "../../../lib/api/listingPreview";
import { exportProduct, type ExportReport } from "../../../lib/api/productExport";
import { errorMessage } from "../../../lib/api/request";
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
  const busy = useRef(false);
  const [exporting, setExporting] = useState(false);
  const [warningToken, setWarningToken] = useState<string | null>(null);
  const [exportError, setExportError] = useState("");
  const [report, setReport] = useState<ExportReport | null>(null);
  useEffect(() => {
    let active = true;
    void getListingPreview(productId).then(value => { if (active) setResult(value); })
      .catch((cause: unknown) => { if (active) setError(cause instanceof Error ? cause.message : "プレビューを取得できませんでした。"); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [productId, revision]);
  function refresh() {
    if (busy.current) return;
    setWarningToken(null); setReport(null); setExportError("");
    setResult(null); setError(""); setLoading(true); setRevision(value => value + 1);
  }
  async function runExport() {
    if (busy.current || !result || result.status === "blocked") return;
    busy.current = true; setExporting(true); setExportError(""); setReport(null);
    try { setReport(await exportProduct(productId, result.snapshotToken, warningToken)); }
    catch (cause: unknown) { setExportError(cause instanceof Error ? cause.message : "出力に失敗しました。"); }
    finally { busy.current = false; setExporting(false); setWarningToken(null); }
  }
  return <section className="panel" aria-labelledby="listing-heading" aria-busy={loading || exporting}>
    <div className="section-heading"><h2 id="listing-heading">出品プレビュー</h2>
      <div className="detail-actions">
        <button type="button" disabled={exporting} onClick={() => { if (!busy.current) onEdit(); }}>商品を編集</button>
        <button type="button" className="secondary" disabled={loading || exporting} onClick={refresh}>プレビューを再読み込み</button>
      </div>
    </div>
    <p className="muted">取得時点の確認用スナップショットです。すべてのECサイトでの出品を保証するものではありません。変更後は再読み込みしてください。</p>
    {loading ? <p role="status">プレビューを読み込んでいます…</p> : error ? <p className="error" role="alert">{error} 再読み込みしてください。</p> : result ? <>
      <h3>準備状況：{statusLabels[result.status]}</h3>
      {result.issues.length > 0 && <ul aria-label="確認事項">{result.issues.map(issue =>
        <li key={issue.code} className={issue.severity === "error" ? "error" : "warning"}>
          {issue.severity === "error" ? "エラー" : "警告"}：{issue.message}
        </li>)}</ul>}
      {result.status === "warning" && <label id="listing-export-confirmation" tabIndex={-1}><input type="checkbox" disabled={exporting}
        checked={warningToken === result.snapshotToken}
        onChange={event => setWarningToken(event.target.checked ? result.snapshotToken : null)} />上記の警告と現在の出品候補を確認し、この内容で出力します</label>}
      {result.status === "blocked" && <p className="error">問題を修正し、再読み込みしてから出力してください。</p>}
      {exportError && <p className="error" role="alert">{exportError}</p>}
      {report?.status === "cancelled" && <p role="status">出力先の選択をキャンセルしました。</p>}
      {report?.status === "success" && <p role="status">エクスポートが完了しました。商品 {report.product_count}件・カスタム項目 {report.attribute_count}件・画像 {report.image_count}枚{report.has_warnings ? "（警告確認済み）" : ""}<br />出力先：{report.folder}</p>}
      {report?.status === "failed" && <p className="error" role="alert">{errorMessage(report.error)} {report.cleanup_failed ? "一時フォルダーの清掃に失敗しました。" : "結果が不明のためファイルを保持しています。"}<br />確認先：{report.retained_folder}</p>}
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
    <div className="page-action-bar">
      {result?.status === "warning" && warningToken !== result.snapshotToken && <p className="muted">本文の警告を確認し、<a href="#listing-export-confirmation">出力を承認</a>してください。</p>}
      {result?.status === "blocked" && <p className="muted">本文の問題を修正し、再読み込みしてください。</p>}
      <div className="detail-actions">
        <button type="button" className="secondary" disabled={exporting} onClick={() => { if (!busy.current) onBack(); }}>商品詳細へ戻る</button>
        <button type="button" disabled={exporting || !result || result.status === "blocked" || (result.status === "warning" && warningToken !== result.snapshotToken)} onClick={() => void runExport()}>
          {exporting ? "エクスポート中…" : "エクスポート"}</button>
      </div>
    </div>
  </section>;
}
