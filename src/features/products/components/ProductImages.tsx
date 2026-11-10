import { useCallback, useEffect, useRef, useState } from "react";
import { importProductImages, listProductImages, reorderProductImages } from "../../../lib/api/images";
import type { ImportReport, ProductImage } from "../imageTypes";

type CardProps = { image: ProductImage; index: number; count: number; busy: boolean; onMove: (id: string, direction: -1 | 1) => void };
function ImageCard({ image, index, count, busy, onMove }: CardProps) {
  const [failed, setFailed] = useState(false);
  return <li className="image-card"><figure>
    {image.preview_url && !failed ?
      <img src={image.preview_url} alt={image.original_filename} loading="lazy" onError={() => setFailed(true)} /> :
      <div className="image-placeholder" role="img" aria-label={`${image.original_filename}：画像を表示できません`}>画像が見つからないか、読み込めません</div>}
    <figcaption><strong>{image.original_filename}</strong>
      {index === 0 && <span>メイン画像</span>}
      <span>{image.width.toLocaleString()} × {image.height.toLocaleString()} px · {(image.file_size_bytes / 1024 / 1024).toFixed(2)} MiB</span>
      {(!image.preview_url || failed) && <span className="missing-image">画像ファイルの状態とバックアップを確認してください。</span>}
    </figcaption>
  </figure>{count > 1 && <div className="detail-actions">
    <button type="button" className="secondary" disabled={busy || index === 0} aria-label={`${index + 1}枚目の${image.original_filename}を前へ移動`} onClick={() => onMove(image.id, -1)}>前へ移動</button>
    <button type="button" className="secondary" disabled={busy || index === count - 1} aria-label={`${index + 1}枚目の${image.original_filename}を後ろへ移動`} onClick={() => onMove(image.id, 1)}>後ろへ移動</button>
  </div>}</li>;
}

type Props = { productId: string; onBusyChange: (busy: boolean) => void };
export function ProductImages({ productId, onBusyChange }: Props) {
  const [images, setImages] = useState<ProductImage[]>([]);
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [orderReady, setOrderReady] = useState(false);
  const [error, setError] = useState("");
  const [report, setReport] = useState<ImportReport | null>(null);
  const operation = useRef(false);
  const generation = useRef(0);
  const refresh = useCallback(async (preserveError = false) => {
    const current = ++generation.current;
    setLoading(true); setOrderReady(false); if (!preserveError) setError("");
    try {
      const result = await listProductImages(productId);
      if (current === generation.current) { setImages(result); setOrderReady(true); }
    } catch (error: unknown) {
      if (current === generation.current) setError(previous => [preserveError ? previous : "", error instanceof Error ? error.message : "画像を取得できませんでした。"].filter(Boolean).join(" "));
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
  async function moveImage(id: string, direction: -1 | 1) {
    if (operation.current || loading || !orderReady) return;
    const index = images.findIndex(image => image.id === id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= images.length) return;
    const expectedImageIds = images.map(image => image.id);
    const orderedImageIds = [...expectedImageIds];
    [orderedImageIds[index], orderedImageIds[target]] = [orderedImageIds[target], orderedImageIds[index]];
    operation.current = true; setSaving(true); onBusyChange(true); setError(""); setReport(null);
    try {
      setImages(await reorderProductImages({ productId, expectedImageIds, orderedImageIds }));
    } catch (error: unknown) {
      setError(error instanceof Error ? error.message : "画像の並び順を保存できませんでした。");
      await refresh(true);
    } finally { operation.current = false; setSaving(false); onBusyChange(false); }
  }
  const busy = loading || importing || saving;
  const imported = report?.items.filter(item => item.status === "imported").length ?? 0;
  const duplicates = report?.items.filter(item => item.status === "duplicate").length ?? 0;
  const failures = report?.items.filter(item => item.status === "failed").length ?? 0;
  return <section className="product-images" aria-labelledby="images-heading" aria-busy={busy}>
    <div className="section-heading"><div><h3 id="images-heading">商品画像</h3>
      <p className="muted">JPEG・PNG・WebP / 1枚20MiB・4,000万画素 / 1回20枚まで</p></div>
      <button type="button" disabled={busy} onClick={() => void importImages()}>{importing ? "画像を選択・取り込み中…" : "画像を取り込む"}</button></div>
    {saving && <p role="status">画像の並び順を保存しています…</p>}
    {importing && <p role="status">画像を選択してください。取り込みが終わるまでこの画面でお待ちください。</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {report && <div className="import-report" role="status">
      <p className={failures ? "warning" : "success"}>{report.cancelled ? "画像の選択をキャンセルしました。" : `取り込み成功 ${imported}枚・重複スキップ ${duplicates}枚・失敗 ${failures}枚`}</p>
      {report.items.some(item => item.message) && <ul>{report.items.map((item, index) => item.message &&
        <li key={index}>{item.filename}：{item.message}</li>)}</ul>}
    </div>}
    {loading ? <p role="status">画像を読み込んでいます…</p> : images.length ?
      <ul className="image-grid">{images.map((image, index) => <ImageCard key={image.id} image={image} index={index} count={images.length} busy={busy || !orderReady} onMove={(id, direction) => void moveImage(id, direction)} />)}</ul> :
      !error && <div className="empty"><h4>画像はまだありません</h4><p>「画像を取り込む」から複数の写真を選択できます。元画像はそのまま保持されます。</p></div>}
    <button type="button" className="secondary" disabled={busy} onClick={() => void refresh()}>画像を再読み込み</button>
  </section>;
}
