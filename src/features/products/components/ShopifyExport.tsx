import { useEffect, useRef, useState } from "react";
import { exportShopifyProducts, getShopifyExportPreview, type ShopifyPreview } from "../../../lib/api/shopifyExport";
import type { ExportReport } from "../../../lib/api/productExport";
import { errorMessage } from "../../../lib/api/request";

const statusLabels = { ready: "出力可能", warning: "警告あり", blocked: "出力不可" };

export function ShopifyExport({ productIds, onBack }: { productIds: string[]; onBack: () => void }) {
  const [preview, setPreview] = useState<ShopifyPreview | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [revision, setRevision] = useState(0);
  const [warningToken, setWarningToken] = useState<string | null>(null);
  const [currencyConfirmed, setCurrencyConfirmed] = useState(false);
  const [error, setError] = useState("");
  const [report, setReport] = useState<ExportReport | null>(null);
  const busy = useRef(true);
  const pending = useRef<{ productIds: string[]; revision: number; promise: Promise<ShopifyPreview> } | null>(null);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    let active = true;
    // The same request is reused during StrictMode's effect replay.
    busy.current = true;
    setPreview(null); setWarningToken(null); setCurrencyConfirmed(false); setReport(null); setError(""); setLoading(true);
    if (!pending.current || pending.current.productIds !== productIds || pending.current.revision !== revision) {
      pending.current = { productIds, revision, promise: getShopifyExportPreview(productIds) };
    }
    pending.current.promise.then(value => { if (active) setPreview(value); })
      .catch((cause: unknown) => { if (active) setError(cause instanceof Error ? cause.message : "Shopify用プレビューを取得できませんでした。"); })
      .finally(() => { if (active) { busy.current = false; setLoading(false); } });
    return () => { active = false; };
  }, [productIds, revision]);

  function refresh() {
    if (busy.current) return;
    busy.current = true;
    setWarningToken(null); setCurrencyConfirmed(false); setPreview(null); setReport(null); setError(""); setLoading(true);
    setRevision(value => value + 1);
  }
  async function run() {
    if (busy.current || !preview || preview.blocked || !currencyConfirmed || (preview.hasWarnings && warningToken !== preview.snapshotToken)) return;
    busy.current = true; setExporting(true); setError(""); setReport(null);
    try {
      const result = await exportShopifyProducts(productIds, preview.snapshotToken, warningToken);
      if (mounted.current) {
        setReport(result);
        if (result.status === "failed") setPreview(null);
      }
    } catch (cause: unknown) {
      if (mounted.current) {
        setError(cause instanceof Error ? cause.message : "Shopify用CSVの出力に失敗しました。");
        setPreview(null);
      }
    } finally {
      busy.current = false;
      if (mounted.current) { setExporting(false); setWarningToken(null); setCurrencyConfirmed(false); }
    }
  }
  return <section className="panel" aria-labelledby="shopify-heading" aria-busy={loading || exporting}>
    <div className="section-heading"><h2 id="shopify-heading">Shopify用CSVの確認</h2><div className="detail-actions">
      <button type="button" className="secondary" disabled={loading || exporting} onClick={() => { if (!busy.current) onBack(); }}>商品一覧へ戻る</button>
      <button type="button" className="secondary" disabled={loading || exporting} onClick={refresh}>Shopify用プレビューを再読み込み</button>
    </div></div>
    <p>対象：{productIds.length}商品。新規商品向けに1つのCSVを出力します。下書き（draft）・非公開（false）です。</p>
    <p>画像・カスタム属性は含まれません。Shopifyインポート時は「一致するHandleを上書き」をオフにしてください。既存Handleとの衝突は検出できません。</p>
    <p>価格はJPYの整数です。ストア通貨は自動取得できません。Shopify管理画面で通貨を確認してください。非JPYストアには使用しないでください。通貨変換は行いません。</p>
    <p>Vendor未設定は空欄です。Shopify側で既定値が付与される場合があります。</p>
    {loading && <p role="status">Shopify用プレビューを読み込んでいます…</p>}
    {error && <p className="error" role="alert">{error} 最新のShopify用プレビューを読み込み直してください。</p>}
    {preview && <>
      <details><summary>確認対象の一時トークン</summary><code>{preview.snapshotToken}</code></details>
      {preview.products.map(p => <article className="batch-product" key={p.productId}>
        <h3>{p.sku} — {p.name}</h3><p>状態：{statusLabels[p.status]}</p>
        <p>出品タイトル：{p.title}</p><p className="listing-description">公開説明文：{p.description ?? "未設定（空欄）"}</p>
        <p>Vendor：{p.vendor ?? "未設定（空欄）"} ／ 価格：{p.priceYen === null ? "未設定" : `${p.priceYen.toLocaleString("ja-JP")}円（JPY）`}</p>
        <p>URL handle：<code>{p.handle}</code></p>
        {p.issues.length > 0 && <ul aria-label="Shopify出力の確認事項">{p.issues.map(issue => <li key={issue.code} className={issue.severity === "error" ? "error" : "warning"}>{issue.severity === "error" ? "出力不可" : "警告"}：{issue.message}</li>)}</ul>}
      </article>)}
      {preview.blocked && <p className="error">出力不可の商品があります。価格などの問題を修正するまで全体を出力できません。</p>}
      {preview.hasWarnings && <p><label><input type="checkbox" checked={warningToken === preview.snapshotToken} disabled={exporting}
        onChange={event => { if (!busy.current) setWarningToken(event.target.checked ? preview.snapshotToken : null); }} />全対象商品の内容と警告を確認し、この内容でShopify用CSVを出力します</label></p>}
      <p><label><input type="checkbox" checked={currencyConfirmed} disabled={exporting}
        onChange={event => { if (!busy.current) setCurrencyConfirmed(event.target.checked); }} />対象ストアの通貨がJPYであることを確認した</label></p>
      <p><button type="button" disabled={exporting || preview.blocked || !currencyConfirmed || (preview.hasWarnings && warningToken !== preview.snapshotToken)} onClick={() => void run()}>{exporting ? "Shopify用CSVを出力中…" : "確認した商品をShopify用CSVで出力"}</button></p>
    </>}
    {report?.status === "cancelled" && <p role="status">出力先の選択をキャンセルしました。警告と通貨の確認は解除しました。</p>}
    {report?.status === "success" && <p className="success" role="status">Shopify用CSVの出力が完了しました。商品 {report.product_count}件{report.has_warnings ? "（警告確認済み）" : ""}。画像・属性は含まれません。<br />CSV：{report.folder}/shopify-products.csv</p>}
    {report?.status === "failed" && <p className="error" role="alert">{errorMessage(report.error)} {report.cleanup_failed ? "一時フォルダーの清掃に失敗しました。" : "確定結果が不明なため保持しています。"} 確認場所：{report.retained_folder}</p>}
  </section>;
}
