import { useCallback, useEffect, useRef, useState } from "react";
import { importProductImages, listProductImages } from "../../../lib/api/images";
import type { ImportReport, ProductImage } from "../imageTypes";

function ImageCard({ image }: { image: ProductImage }) {
  const [failed, setFailed] = useState(false);
  return <li className="image-card"><figure>
    {image.preview_url && !failed ?
      <img src={image.preview_url} alt={image.original_filename} loading="lazy" onError={() => setFailed(true)} /> :
      <div className="image-placeholder" role="img" aria-label={`${image.original_filename}：画像を表示できません`}>画像が見つからないか、読み込めません</div>}
    <figcaption><strong>{image.original_filename}</strong>
      <span>{image.width.toLocaleString()} × {image.height.toLocaleString()} px · {(image.file_size_bytes / 1024 / 1024).toFixed(2)} MiB</span>
      {(!image.preview_url || failed) && <span className="missing-image">画像ファイルの状態とバックアップを確認してください。</span>}
    </figcaption>
  </figure></li>;
}

type Props = { productId: string; onBusyChange: (busy: boolean) => void };
export function ProductImages({ productId, onBusyChange }: Props) {
  const [images, setImages] = useState<ProductImage[]>([]);
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState("");
  const [report, setReport] = useState<ImportReport | null>(null);
  const operation = useRef(false);
  const generation = useRef(0);
  const refresh = useCallback(async () => {
    const current = ++generation.current;
    setLoading(true); setError("");
    try {
      const result = await listProductImages(productId);
      if (current === generation.current) setImages(result);
    } catch (error: unknown) {
      if (current === generation.current) setError(error instanceof Error ? error.message : "画像を取得できませんでした。");
    } finally { if (current === generation.current) setLoading(false); }
  }, [productId]);
  useEffect(() => { void refresh(); return () => { generation.current++; }; }, [refresh]);

  async function importImages() {
    if (operation.current) return;
    operation.current = true; setImporting(true); onBusyChange(true); setError(""); setReport(null);
    try {
      setReport(await importProductImages(productId));
      await refresh();
    } catch (error: unknown) { setError(error instanceof Error ? error.message : "画像を取り込めませんでした。"); }
    finally { operation.current = false; setImporting(false); onBusyChange(false); }
  }
  const imported = report?.items.filter(item => item.status === "imported").length ?? 0;
  const duplicates = report?.items.filter(item => item.status === "duplicate").length ?? 0;
  const failures = report?.items.filter(item => item.status === "failed").length ?? 0;
  return <section className="product-images" aria-labelledby="images-heading" aria-busy={loading || importing}>
    <div className="section-heading"><div><h3 id="images-heading">商品画像</h3>
      <p className="muted">JPEG・PNG・WebP / 1枚20MiB・4,000万画素 / 1回20枚まで</p></div>
      <button type="button" disabled={loading || importing} onClick={() => void importImages()}>{importing ? "画像を選択・取り込み中…" : "画像を取り込む"}</button></div>
    {importing && <p role="status">画像を選択してください。取り込みが終わるまでこの画面でお待ちください。</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {report && <div className="import-report" role="status">
      <p className={failures ? "warning" : "success"}>{report.cancelled ? "画像の選択をキャンセルしました。" : `取り込み成功 ${imported}枚・重複スキップ ${duplicates}枚・失敗 ${failures}枚`}</p>
      {report.items.some(item => item.message) && <ul>{report.items.map((item, index) => item.message &&
        <li key={index}>{item.filename}：{item.message}</li>)}</ul>}
    </div>}
    {loading ? <p role="status">画像を読み込んでいます…</p> : images.length ?
      <ul className="image-grid">{images.map(image => <ImageCard key={`${image.id}:${image.preview_url}`} image={image} />)}</ul> :
      !error && <div className="empty"><h4>画像はまだありません</h4><p>「画像を取り込む」から複数の写真を選択できます。元画像はそのまま保持されます。</p></div>}
    <button type="button" className="secondary" disabled={loading || importing} onClick={() => void refresh()}>画像を再読み込み</button>
  </section>;
}
