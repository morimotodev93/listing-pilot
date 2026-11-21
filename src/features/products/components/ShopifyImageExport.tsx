import { useEffect, useRef, useState } from "react";
import { exportShopifyProductsWithImages, getShopifyImageExportPreview, type ShopifyImagePreview } from "../../../lib/api/shopifyExport";
import type { ExportReport } from "../../../lib/api/productExport";
import { errorMessage } from "../../../lib/api/request";
import { listProductImages } from "../../../lib/api/images";
import type { ProductImage } from "../imageTypes";

type Props = { productIds: string[]; onBack: () => void; onExportingChange: (value: boolean) => void };
const statusLabels = { ready: "出力可能", warning: "警告あり", blocked: "出力不可" };

function ExportThumbnail({ image }: { image: ProductImage | undefined }) {
  const [failed, setFailed] = useState(false);
  return image?.preview_url && !failed
    ? <img className="shopify-export-thumbnail" src={image.preview_url} alt={image.original_filename} loading="lazy" decoding="async" onError={() => setFailed(true)} />
    : <div className="shopify-export-thumbnail image-placeholder" role="img" aria-label="ローカル画像を表示できません">サムネイルを表示できません</div>;
}

export function ShopifyImageExport({ productIds, onBack, onExportingChange }: Props) {
  const [store, setStore] = useState("");
  const [preview, setPreview] = useState<ShopifyImagePreview | null>(null);
  const [thumbnails, setThumbnails] = useState<Map<string, Map<string, ProductImage>>>(new Map());
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [warningToken, setWarningToken] = useState<string | null>(null);
  const [currencyConfirmed, setCurrencyConfirmed] = useState(false);
  const [error, setError] = useState("");
  const [report, setReport] = useState<ExportReport | null>(null);
  const generation = useRef(0);
  const outputLock = useRef(false);
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    function preventLeave(event: BeforeUnloadEvent) {
      if (outputLock.current) { event.preventDefault(); event.returnValue = ""; }
    }
    window.addEventListener("beforeunload", preventLeave);
    return () => { mounted.current = false; generation.current++; window.removeEventListener("beforeunload", preventLeave); };
  }, []);
  // A changed selection invalidates outstanding responses as well as confirmations.
  useEffect(() => {
    generation.current++; setPreview(null); setWarningToken(null); setCurrencyConfirmed(false); setReport(null); setError(""); setLoading(false);
  }, [productIds]);

  function invalidate() {
    setThumbnails(new Map());
    generation.current++; setPreview(null); setWarningToken(null); setCurrencyConfirmed(false); setReport(null); setError(""); setLoading(false);
  }
  async function loadThumbnails(productId: string, current: number) {
    try {
      const images = await listProductImages(productId);
      if (mounted.current && current === generation.current) {
        setThumbnails(previous => new Map(previous).set(productId,
          new Map(images.filter(image => image.product_id === productId).map(image => [image.id, image])),
        ));
      }
    } catch {
      // Supplementary images keep their placeholders; CSV validation is independent.
    }
  }
  async function refresh() {
    if (outputLock.current || !store.trim()) return;
    const current = ++generation.current;
    setThumbnails(new Map());
    setPreview(null); setWarningToken(null); setCurrencyConfirmed(false); setReport(null); setError(""); setLoading(true);
    for (const productId of productIds) void loadThumbnails(productId, current);
    try {
      const result = await getShopifyImageExportPreview(productIds, store);
      if (mounted.current && current === generation.current) {
        setPreview(result); setStore(result.storeKey);
      }
    } catch (cause: unknown) {
      if (mounted.current && current === generation.current) setError(cause instanceof Error ? cause.message : "画像付きプレビューを取得できませんでした。");
    } finally {
      if (mounted.current && current === generation.current) setLoading(false);
    }
  }
  async function run() {
    if (outputLock.current || loading || !preview || preview.blocked || !currencyConfirmed || (preview.hasWarnings && warningToken !== preview.snapshotToken)) return;
    outputLock.current = true; onExportingChange(true); setExporting(true); setError(""); setReport(null);
    const current = ++generation.current;
    try {
      const result = await exportShopifyProductsWithImages(productIds, preview.storeKey, preview.snapshotToken, warningToken);
      if (mounted.current && current === generation.current) {
        setReport(result);
        if (result.status === "failed") setPreview(null);
      }
    } catch (cause: unknown) {
      if (mounted.current && current === generation.current) {
        setError(cause instanceof Error ? cause.message : "画像付きCSVを出力できませんでした。"); setPreview(null);
      }
    } finally {
      outputLock.current = false;
      if (mounted.current) { setExporting(false); setWarningToken(null); setCurrencyConfirmed(false); onExportingChange(false); }
    }
  }

  return <section className="panel shopify-image-export" aria-labelledby="shopify-image-export-heading" aria-busy={loading || exporting}>
    <div className="section-heading"><h2 id="shopify-image-export-heading">Shopify画像付き13列CSVの確認</h2>
      <button type="button" className="secondary" disabled={exporting} onClick={() => { if (!outputLock.current) { generation.current++; onBack(); } }}>商品一覧へ戻る</button></div>
    <p>対象：{productIds.length}商品。新規登録用・下書き（draft）・非公開（false）。インポート時は「一致するHandleを上書き」をオフにしてください。既存Handleとの衝突は検出できません。</p>
    <p>全管理画像のURLが必要です。1商品1〜250画像。原本をコピーせず、画像URLをCSVへ記録します。URLの実在・画像内容・ストア所有権は確認しません。</p>
    <p>価格はJPY整数です。ストア通貨は自動取得・変換しません。Shopify管理画面で対象ストアの通貨を確認してください。</p>
    <p className="muted">URLを変更する場合は商品一覧へ戻り、商品詳細→商品画像→「Shopify画像URLを設定」を開いてください。</p>
    <fieldset disabled={exporting} className="shopify-store-controls">
      <legend>画像URL設定の対象ストア</legend><label htmlFor="shopify-export-store">ストアのドメイン</label>
      <input id="shopify-export-store" value={store} placeholder="example-store.myshopify.com" spellCheck={false} autoCapitalize="none"
        onChange={event => { if (!outputLock.current) { setStore(event.target.value); invalidate(); } }} />
      <button type="button" disabled={loading || !store.trim()} onClick={() => void refresh()}>画像付きプレビューを読み込む</button>
    </fieldset>
    {loading && <p role="status">画像付きプレビューを読み込んでいます…</p>}
    {error && <p role="alert" className="error">{error} 最新の画像付きプレビューを読み込み直してください。</p>}
    {preview && <>
      <p className="shopify-url-text">対象ストア：<strong>{preview.storeKey}</strong> ／ 画像URL行数：{preview.imageCount}行</p>
      {preview.products.map(p => <article key={p.productId} className="batch-product">
        <h3>{p.sku} — {p.name}</h3><p>状態：{statusLabels[p.status]}</p><small>商品UUID：{p.productId}</small>
        <p>出品タイトル：{p.title}</p><p className="product-note">公開説明文：{p.description ?? "未設定（空欄）"}</p>
        <p>Vendor：{p.vendor ?? "未設定（空欄）"} ／ 価格：{p.priceYen === null ? "未設定" : `${p.priceYen.toLocaleString("ja-JP")}円（JPY）`}</p>
        <p className="shopify-url-text">Handle：<code>{p.handle}</code></p>
        <p>管理画像：{p.imageCount}枚 ／ URL登録済み：{p.urlConfiguredCount}枚{p.urlConfiguredCount < p.imageCount ? ` ／ 未設定：${p.imageCount - p.urlConfiguredCount}枚` : ""}（有効性は確認事項を参照）</p>
        <p>メイン画像：{p.mainImageId ? `1枚目・UUID ${p.mainImageId.slice(0, 8)}` : "なし"}</p>
        <details open><summary>画像と順序・URL／altの設定状態を確認（{p.imageCount}枚）</summary>
          <ol className="shopify-export-image-list">{p.images.map(image => <li key={image.imageId || image.position}>
            <ExportThumbnail key={`${preview.snapshotToken}-${image.imageId}`} image={thumbnails.get(p.productId)?.get(image.imageId)} />
            <span>{image.position}枚目{image.position === 1 ? "（メイン）" : ""}・UUID {image.imageId.slice(0, 8) || "不正"} ／ URL：{image.urlConfigured ? "設定済み" : "未設定"} ／ alt：{image.altConfigured ? "設定済み" : "空欄"}{image.issues.length ? " ／ 要修正" : ""}</span>
          </li>)}</ol>
        </details>
        <small>ローカル画像だけを表示します。表示できない場合もCSVの出力可否は変更しません。必要に応じて商品画像画面で原本の状態を確認してください。</small>
        {p.issues.length > 0 && <ul aria-label="商品情報の確認事項">{p.issues.map(issue => <li key={issue.code} className={issue.severity === "error" ? "error" : "warning"}>{issue.message}</li>)}</ul>}
        {p.imageIssues.length > 0 && <ul aria-label="画像付き出力の確認事項">{p.imageIssues.map((issue, index) => <li key={`${issue.code}-${issue.imageId}-${index}`} className="error">{issue.position ? `画像${issue.position}枚目：` : ""}{issue.message}</li>)}</ul>}
      </article>)}
      {preview.blocked && <p role="alert" className="error">出力不可の商品があります。全商品の画像URL・画像情報・価格などを修正するまで全件の出力を停止します。</p>}
      <fieldset disabled={exporting}>
        <legend className="sr-only">画像付きCSVの出力確認</legend>
        {preview.hasWarnings && <label><input type="checkbox" checked={warningToken === preview.snapshotToken} onChange={event => { if (!outputLock.current) setWarningToken(event.target.checked ? preview.snapshotToken : null); }} />全対象商品の内容と警告を確認し、この画像付きプレビューの出力を承認します</label>}
        <label><input type="checkbox" checked={currencyConfirmed} onChange={event => { if (!outputLock.current) setCurrencyConfirmed(event.target.checked); }} />対象ストアの通貨がJPYであることを確認した</label>
        <button type="button" disabled={preview.blocked || !currencyConfirmed || (preview.hasWarnings && warningToken !== preview.snapshotToken)} onClick={() => void run()}>{exporting ? "画像付きCSVを出力中…" : "確認した商品を画像付きCSVで出力"}</button>
      </fieldset>
    </>}
    {report?.status === "cancelled" && <p role="status">出力先の選択をキャンセルしました。警告承認とJPY確認は解除しました。</p>}
    {report?.status === "success" && <p role="status" className="success shopify-url-text">画像付きCSVを出力しました。商品{report.product_count}件・画像URL{report.image_count}件。<br />CSV：{report.folder}/shopify-products.csv</p>}
    {report?.status === "failed" && <p role="alert" className="error">{errorMessage(report.error)} {report.cleanup_failed ? "一時フォルダーの清掃に失敗しました。" : "確定結果が不明なため保持しています。"} 確認場所：{report.retained_folder}</p>}
  </section>;
}
