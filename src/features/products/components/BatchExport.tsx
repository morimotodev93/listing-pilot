import { useEffect, useRef, useState } from "react";
import { exportProducts, getExportBatchPreview, type BatchPreview, type ExportReport } from "../../../lib/api/productExport";
import { errorMessage } from "../../../lib/api/request";

export function BatchExport({ productIds, onBack }: { productIds: string[]; onBack: () => void }) {
  const [preview, setPreview] = useState<BatchPreview | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [revision, setRevision] = useState(0);
  const [warningToken, setWarningToken] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [report, setReport] = useState<ExportReport | null>(null);
  const busy = useRef(true);
  const pending = useRef<{ productIds: string[]; revision: number; promise: Promise<BatchPreview> } | null>(null);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    let active = true;
    // Reuse the request during StrictMode's effect replay; the backend permit is exclusive.
    if (!pending.current || pending.current.productIds !== productIds || pending.current.revision !== revision) {
      pending.current = { productIds, revision, promise: getExportBatchPreview(productIds) };
    }
    pending.current.promise.then(value => { if (active) setPreview(value); })
      .catch((cause: unknown) => { if (active) setError(cause instanceof Error ? cause.message : "一括プレビューを取得できませんでした。"); })
      .finally(() => { if (active) { busy.current = false; setLoading(false); } });
    return () => { active = false; };
  }, [productIds, revision]);
  function refresh() {
    if (busy.current) return;
    busy.current = true;
    setWarningToken(null); setPreview(null); setReport(null); setError(""); setLoading(true);
    setRevision(value => value + 1);
  }
  async function run() {
    if (busy.current || !preview || preview.blocked || (preview.hasWarnings && warningToken !== preview.snapshotToken)) return;
    busy.current = true; setExporting(true); setError(""); setReport(null);
    try {
      const result = await exportProducts(productIds, preview.snapshotToken, warningToken);
      if (mounted.current) setReport(result);
    } catch (cause: unknown) {
      if (mounted.current) {
        setError(cause instanceof Error ? cause.message : "出力に失敗しました。");
        // Require a fresh preview after a failed confirmation, never auto retry.
        setPreview(null);
      }
    } finally {
      busy.current = false;
      if (mounted.current) { setExporting(false); setWarningToken(null); }
    }
  }
  return <section className="panel" aria-labelledby="batch-heading" aria-busy={loading || exporting}>
    <div className="section-heading"><h2 id="batch-heading">一括エクスポートの確認</h2><div className="detail-actions">
      <button type="button" className="secondary" disabled={loading || exporting} onClick={() => { if (!busy.current) onBack(); }}>商品一覧へ戻る</button>
      <button type="button" className="secondary" disabled={loading || exporting} onClick={refresh}>一括プレビューを再読み込み</button>
    </div></div>
    <p>対象：{productIds.length}商品。すべての商品を確認し、まとめて出力します。</p>
    {loading && <p role="status">一括プレビューを読み込んでいます…</p>}
    {error && <p className="error" role="alert">{error} 最新の一括プレビューを読み込み直してください。</p>}
    {preview && <>
      <p>出力予定の画像：合計 {preview.imageCount}枚</p>
      {preview.products.map(({ name, groupName, preview: p }) => <article className="batch-product" key={p.productId}>
        <h3>{p.sku} — {name}</h3><p>所属グループ：{groupName} ／ 状態：{p.status}</p>
        <p>出品タイトル：{p.title}</p><p className="listing-description">説明文：{p.description ?? "未設定"}</p>
        <p>価格：{p.priceYen === null ? "未設定" : `${p.priceYen.toLocaleString("ja-JP")}円`} ／ 画像 {p.images.length}枚</p>
        {p.attributes.length > 0 && <ul aria-label="出力するカスタム項目">{p.attributes.map(a => <li key={a.attributeId}>{a.name}：{a.value}{a.unit ? ` ${a.unit}` : ""}</li>)}</ul>}
        {p.issues.length > 0 && <ul aria-label="商品別の確認事項">{p.issues.map(issue => <li key={issue.code} className={issue.severity === "error" ? "error" : "warning"}>{issue.severity === "error" ? "エラー" : "警告"}：{issue.message}</li>)}</ul>}
      </article>)}
      {preview.blocked && <p className="error">出力不可の商品があります。問題を修正するまで全体を出力できません。</p>}
      {preview.hasWarnings && <label><input type="checkbox" checked={warningToken === preview.snapshotToken} disabled={exporting}
        onChange={event => { if (!busy.current) setWarningToken(event.target.checked ? preview.snapshotToken : null); }} />全対象商品の内容と警告を確認し、この内容で一括出力します</label>}
      <p><button type="button" disabled={exporting || preview.blocked || (preview.hasWarnings && warningToken !== preview.snapshotToken)} onClick={() => void run()}>{exporting ? "一括エクスポート中…" : "確認した全商品を出力"}</button></p>
    </>}
    {report?.status === "cancelled" && <p role="status">出力先の選択をキャンセルしました。警告の承認は解除しました。</p>}
    {report?.status === "success" && <p className="success" role="status">一括エクスポートが完了しました。商品 {report.product_count}件・カスタム項目 {report.attribute_count}件・画像 {report.image_count}枚{report.has_warnings ? "（警告確認済み）" : ""}<br />出力先：{report.folder}</p>}
    {report?.status === "failed" && <p className="error" role="alert">{errorMessage(report.error)} {report.cleanup_failed ? "一時フォルダーの清掃に失敗しました。" : "確定結果が不明なため保持しています。"} 確認場所：{report.retained_folder}</p>}
  </section>;
}
