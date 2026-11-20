import { useEffect, useRef, useState } from "react";
import { listProductImages } from "../../../lib/api/images";
import { RequestError } from "../../../lib/api/request";
import { getShopifyImageUrls, saveShopifyImageUrls } from "../../../lib/api/shopifyImageUrls";
import type { ProductImage } from "../imageTypes";
import type { ImageUrlEdit, ImageUrlSnapshot } from "../shopifyImageUrlTypes";
import { editIssues, pasteLines, scalarCount, snapshotEdits, snapshotMatches, urlBytes } from "../shopifyImageUrlEditor";

type Props = { productId: string; productName: string; sku: string; onBack: () => void };
type Loaded = { snapshot: ImageUrlSnapshot; images: ProductImage[]; version: number };
type Action = { kind: "back" } | { kind: "load"; store: string };
type Blocked = "" | "stale" | "unknown" | "load";

function LocalPreview({ image }: { image: ProductImage }) {
  const [failed, setFailed] = useState(false);
  return image.preview_url && !failed ? (
    <img className="shopify-url-thumbnail" src={image.preview_url} alt={image.original_filename}
      loading="lazy" onError={() => setFailed(true)} />
  ) : (
    <div className="shopify-url-thumbnail image-placeholder" role="img" aria-label={`${image.original_filename}：画像を表示できません`}>
      画像を表示できません。原本の状態とバックアップを確認してください。
    </div>
  );
}

export function ShopifyImageUrls({ productId, productName, sku, onBack }: Props) {
  const [storeDraft, setStoreDraft] = useState("");
  const [stores, setStores] = useState<string[]>([]);
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [edits, setEdits] = useState<Record<string, ImageUrlEdit>>({});
  const [hashConfirmed, setHashConfirmed] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(false);
  const [loadingStore, setLoadingStore] = useState("");
  const [saving, setSaving] = useState(false);
  const [blocked, setBlocked] = useState<Blocked>("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [bulkText, setBulkText] = useState("");
  const [bulkPreview, setBulkPreview] = useState<string[] | null>(null);
  const [bulkConfirmed, setBulkConfirmed] = useState(false);
  const [bulkError, setBulkError] = useState("");
  const [action, setAction] = useState<Action | null>(null);
  const discardDialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const mounted = useRef(false);
  const generation = useRef(0);
  const saveLock = useRef(false);

  useEffect(() => {
    mounted.current = true;
    heading.current?.focus();
    return () => { mounted.current = false; generation.current++; };
  }, []);
  useEffect(() => {
    if (action && !discardDialog.current?.open) discardDialog.current?.showModal();
  }, [action]);

  const rows = loaded?.snapshot.images ?? [];
  const orderedEdits = rows.map(row => edits[row.imageId]);
  const issues = editIssues(orderedEdits);
  const dirtyEdits = rows.some(row => {
    const edit = edits[row.imageId];
    return edit.url !== (row.setting?.url ?? "") || edit.altText !== (row.setting?.altText ?? "")
      || (row.setting?.imageSha256 !== row.sha256 && !!edit.url && hashConfirmed[row.imageId]);
  });
  const pendingPaste = bulkText !== "" || bulkPreview !== null;
  const dirty = dirtyEdits || pendingPaste;
  const unconfirmedHash = rows.some(row => row.setting && row.setting.imageSha256 !== row.sha256
    && edits[row.imageId].url !== "" && !hashConfirmed[row.imageId]);
  const invalid = [...issues.values()].some(issue => issue.url || issue.alt);
  const pendingStore = loaded !== null && storeDraft.trim().toLowerCase() !== loaded.snapshot.storeKey;
  const localImages = new Map(loaded?.images.map(image => [image.id, image]));

  useEffect(() => {
    function preventLeave(event: BeforeUnloadEvent) {
      if (saveLock.current || dirty) { event.preventDefault(); event.returnValue = ""; }
    }
    window.addEventListener("beforeunload", preventLeave);
    return () => window.removeEventListener("beforeunload", preventLeave);
  }, [dirty]);

  function accept(snapshot: ImageUrlSnapshot, images: ProductImage[], version: number) {
    setLoaded({ snapshot, images, version });
    setEdits(snapshotEdits(snapshot));
    setHashConfirmed({}); setBlocked("");
    setBulkText(""); setBulkPreview(null); setBulkConfirmed(false); setBulkError("");
  }

  async function load(store: string) {
    if (saveLock.current) return;
    const current = ++generation.current;
    setLoading(true); setLoadingStore(store); setError(""); setNotice("");
    try {
      const [images, snapshot] = await Promise.all([
        listProductImages(productId), getShopifyImageUrls({ productId, storeKey: store }),
      ]);
      if (!mounted.current || current !== generation.current) return;
      if (!snapshotMatches(snapshot, images, productId, store)) {
        setBlocked("stale");
        setError("画像とURL設定のUUID・順序・画像情報が一致しません。古い状態のため保存できません。最新状態を再読み込みしてください。");
        return;
      }
      accept(snapshot, images, current);
      setStoreDraft(value => value === store ? snapshot.storeKey : value);
      setStores(value => value.includes(snapshot.storeKey) ? value : [...value, snapshot.storeKey]);
    } catch (cause: unknown) {
      if (mounted.current && current === generation.current) {
        setBlocked("load");
        setError(cause instanceof Error ? cause.message : "画像とURL設定を取得できませんでした。再読み込みしてください。");
      }
    } finally {
      if (mounted.current && current === generation.current) setLoading(false);
    }
  }

  function perform(next: Action) {
    if (next.kind === "back") { generation.current++; onBack(); }
    else void load(next.store);
  }
  function requestAction(next: Action) {
    if (saveLock.current || discardDialog.current?.open) return;
    if (dirty) setAction(next);
    else perform(next);
  }
  function cancelDiscard() {
    if (saveLock.current) return;
    discardDialog.current?.close(); setAction(null);
  }
  function confirmDiscard() {
    if (saveLock.current || !action) return;
    discardDialog.current?.close(); setAction(null); perform(action);
  }
  function change(imageId: string, patch: Partial<Pick<ImageUrlEdit, "url" | "altText">>) {
    if (saveLock.current || loading) return;
    setEdits(value => ({ ...value, [imageId]: { ...value[imageId], ...patch } }));
    setHashConfirmed(value => ({ ...value, [imageId]: false })); setNotice("");
  }

  function checkPaste() {
    if (saveLock.current || loading || !loaded || rows.length === 0) return;
    setBulkPreview(null); setBulkConfirmed(false); setBulkError("");
    const lines = pasteLines(bulkText);
    if (lines.length !== rows.length) {
      setBulkError(`画像${rows.length}枚に対し${lines.length}行あります。空行・末尾の改行も1行として数えます。入力欄は変更していません。`);
      return;
    }
    const proposed = rows.map((row, index) => ({ imageId: row.imageId, url: lines[index], altText: "" }));
    const errors = editIssues(proposed);
    const messages = proposed.flatMap((edit, index) => errors.get(edit.imageId)!.url ? [`${index + 1}行目：${errors.get(edit.imageId)!.url}`] : []);
    if (messages.length) { setBulkError(messages.join(" ")); return; }
    setBulkPreview(lines);
  }
  function applyPaste() {
    if (saveLock.current || loading || !bulkPreview || !bulkConfirmed || bulkPreview.length !== rows.length) return;
    setEdits(value => Object.fromEntries(rows.map((row, index) => [row.imageId, {
      imageId: row.imageId, url: bulkPreview[index], altText: bulkPreview[index] === "" ? "" : value[row.imageId].altText,
    }])));
    setHashConfirmed({}); setBulkText(""); setBulkPreview(null); setBulkConfirmed(false); setBulkError("");
    setNotice("一括貼り付けを入力欄へ適用しました。まだ保存していません。");
  }

  const saveReason = !loaded ? "対象ストアの設定を読み込んでください。"
    : rows.length === 0 ? "画像を追加してから設定してください。"
    : loading ? "読み込みが終わるまで保存できません。"
    : blocked ? "最新状態を再読み込みして確認するまで保存できません。編集内容は保持しています。"
    : pendingStore ? "入力中のストアはまだ読み込んでいません。対象ストアを読み込むか、現在の対象ストアのドメインに戻してください。"
    : pendingPaste ? "一括貼り付けを入力欄へ適用するか、貼り付けをキャンセルしてください。"
    : invalid ? "各画像の入力エラーを修正してください。"
    : unconfirmedHash ? "画像情報が変わった行の対応関係を明示確認してください。"
    : !dirtyEdits ? "保存する変更はありません。" : "";

  async function save() {
    if (saveLock.current || loading || saveReason || !loaded) return;
    saveLock.current = true; setSaving(true); setError(""); setNotice("");
    const current = ++generation.current;
    try {
      const snapshot = await saveShopifyImageUrls({
        productId, storeKey: loaded.snapshot.storeKey, snapshotToken: loaded.snapshot.snapshotToken,
        images: orderedEdits.map(edit => ({ ...edit })),
      });
      if (!mounted.current || current !== generation.current) return;
      if (!snapshotMatches(snapshot, loaded.images, productId, loaded.snapshot.storeKey)) {
        setBlocked("unknown");
        setError("保存結果と表示中の画像情報を照合できません。成功・失敗は断定できません。編集内容を控え、明示的に最新状態を再読み込みして確認してください。");
        return;
      }
      accept(snapshot, loaded.images, current);
      setNotice(`${snapshot.storeKey} の画像URL設定を保存しました。`);
    } catch (cause: unknown) {
      if (!mounted.current || current !== generation.current) return;
      const code = cause instanceof RequestError ? cause.code : "";
      if (code === "stale_data" || code === "invalid_image_url_selection") {
        setBlocked("stale");
        setError("別の操作により設定が変更されました。画像構成やURL設定が古いため保存していません。編集内容を控えたうえで最新状態を再読み込みしてください。自動再保存はしません。");
      } else if (code === "image_url_save_unknown" || !code) {
        setBlocked("unknown");
        setError("URL設定の保存結果を確認できませんでした。成功・失敗は断定できません。自動再送信はしません。編集内容を控え、最新状態を再読み込みして保存結果を確認してください。");
      } else {
        setError(cause instanceof Error ? cause.message : "設定を保存できませんでした。入力内容は保持しています。");
      }
    } finally {
      if (mounted.current && current === generation.current) { saveLock.current = false; setSaving(false); }
    }
  }

  return <section className="panel shopify-image-urls" aria-labelledby="shopify-urls-heading" aria-busy={loading || saving}>
    <div className="section-heading">
      <div><h2 id="shopify-urls-heading" ref={heading} tabIndex={-1}>Shopify画像URLを設定</h2>
        <p className="shopify-url-product"><strong>{productName}</strong> / SKU：{sku}</p></div>
      <button type="button" className="secondary" disabled={saving} onClick={() => requestAction({ kind: "back" })}>商品画像へ戻る</button>
    </div>
    <p className="muted">Shopify Filesからコピーした公開画像URLを、ローカル画像ごとに設定します。URLの実在・画像内容・ストアの所有権は確認しません。画像付きCSV出力は後続機能です。</p>
    <fieldset disabled={saving} className="shopify-store-controls">
      <legend>対象ストア</legend>
      <label htmlFor="shopify-store-domain">ストアのドメイン</label>
      <input id="shopify-store-domain" list="shopify-session-stores" value={storeDraft} autoCapitalize="none" spellCheck={false}
        placeholder="example-store.myshopify.com" onChange={event => { if (!saveLock.current) setStoreDraft(event.target.value); }} />
      <datalist id="shopify-session-stores">{stores.map(store => <option key={store} value={store} />)}</datalist>
      <small>この画面で読み込んだストアだけを候補表示します。ストア一覧や履歴は保存しません。</small>
      <button type="button" disabled={!storeDraft.trim()} onClick={() => requestAction({ kind: "load", store: storeDraft })}>このストアの設定を読み込む</button>
    </fieldset>
    {loading && <p role="status" className="shopify-url-text">{loadingStore} の画像と設定を読み込んでいます…</p>}
    {error && <p role="alert" className="error">{error}</p>}
    {notice && <p role="status" className="success shopify-url-text">{notice}</p>}
    {loaded && <>
      <div className="shopify-url-context">
        <p className="shopify-url-text">現在の対象ストア：<strong>{loaded.snapshot.storeKey}</strong> / {rows.length}枚</p>
        <p className="muted">{dirty ? "未保存の入力があります。" : "保存済みの設定を表示しています。"} 別ストアへ入力内容を引き継ぎません。</p>
        {pendingStore && <p className="warning">ストア入力欄と現在の対象ストアが異なります。「このストアの設定を読み込む」で切り替えるまで保存できません。</p>}
        <button type="button" className="secondary" disabled={saving || loading}
          onClick={() => requestAction({ kind: "load", store: loaded.snapshot.storeKey })}>最新状態を再読み込み</button>
      </div>
      {rows.length === 0 ? <div className="empty"><h3>画像はまだありません</h3><p>商品画像へ戻り、画像を追加してください。空の画像集合は保存しません。</p></div> : <>
        <fieldset disabled={saving || loading}>
          <legend className="sr-only">画像ごとのURLとaltテキスト</legend>
          <ol className="shopify-url-list">{rows.map((row, index) => {
            const image = localImages.get(row.imageId)!;
            const edit = edits[row.imageId];
            const issue = issues.get(row.imageId)!;
            const id = `shopify-image-${row.imageId}`;
            const mismatched = row.setting && row.setting.imageSha256 !== row.sha256 && edit.url !== "";
            return <li key={row.imageId} className="shopify-url-row">
              <div className="shopify-url-identity">
                <LocalPreview key={`${loaded.version}-${row.imageId}`} image={image} />
                <div><h3>{index + 1}枚目{index === 0 ? "・メイン画像" : ""}</h3>
                  <p className="shopify-url-text">{image.original_filename}</p><small>画像UUID：{row.imageId.slice(0, 8)}</small>
                  <p className="muted">保存済み：{row.setting ? "設定済み" : "未設定"} / 入力：{edit.url ? "URLあり" : "未設定・解除"}</p></div>
              </div>
              <label htmlFor={`${id}-url`}>Shopify画像URL</label>
              <textarea id={`${id}-url`} rows={2} value={edit.url} spellCheck={false} autoCapitalize="none"
                aria-invalid={!!issue.url} aria-describedby={`${id}-url-help${issue.url ? ` ${id}-url-error` : ""}`}
                onChange={event => change(row.imageId, { url: event.target.value })} />
              <small id={`${id}-url-help`}>{urlBytes(edit.url).toLocaleString()} / 4,096 UTF-8バイト。https://cdn.shopify.com/ の画像URL（クエリ可）。最終検証は保存時に行います。</small>
              {issue.url && <p id={`${id}-url-error`} className="error">{issue.url}</p>}
              <label htmlFor={`${id}-alt`}>画像altテキスト（任意・単一行）</label>
              <textarea id={`${id}-alt`} rows={1} value={edit.altText}
                aria-invalid={!!issue.alt} aria-describedby={`${id}-alt-help${issue.alt ? ` ${id}-alt-error` : ""}`}
                onChange={event => change(row.imageId, { altText: event.target.value })} />
              <small id={`${id}-alt-help`} className={scalarCount(edit.altText) > 125 ? "warning" : undefined}>
                {scalarCount(edit.altText)} / 512文字。125文字以内を推奨。空欄可・自動生成しません。</small>
              {issue.alt && <p id={`${id}-alt-error`} className="error">{issue.alt}</p>}
              {mismatched && <div className="shopify-url-hash-warning"><p className="warning">保存時と現在の画像情報が異なります。サムネイルとURLの対応を確認するか、設定を解除してください。</p>
                <label><input type="checkbox" checked={!!hashConfirmed[row.imageId]} onChange={event => {
                  if (!saveLock.current && !loading) setHashConfirmed(value => ({ ...value, [row.imageId]: event.target.checked }));
                }} />この画像と入力URLの対応を確認し、現在の画像情報で保存します</label></div>}
              <button type="button" className="secondary" disabled={!edit.url && !edit.altText}
                onClick={() => change(row.imageId, { url: "", altText: "" })}>URL設定を解除</button>
            </li>;
          })}</ol>
          <p className="muted">「URL設定を解除」はURLとaltを空にします。ローカル画像やShopify Filesの画像自体は削除しません。保存するまで設定は変更されません。</p>
          <section className="shopify-url-bulk" aria-labelledby="shopify-bulk-heading">
            <h3 id="shopify-bulk-heading">URLをまとめて貼り付け</h3>
            <label htmlFor="shopify-url-paste">現在の画像順に1行1URL</label>
            <textarea id="shopify-url-paste" rows={5} value={bulkText} spellCheck={false} autoCapitalize="none"
              aria-describedby="shopify-paste-help" onChange={event => {
                if (saveLock.current || loading) return;
                setBulkText(event.target.value); setBulkPreview(null); setBulkConfirmed(false); setBulkError("");
              }} />
            <small id="shopify-paste-help">空行・末尾の改行も1行として扱います。空行は対応画像のURLとaltの解除を意味します。適用だけでは保存しません。</small>
            <div className="detail-actions"><button type="button" className="secondary" onClick={checkPaste}>画像との対応を確認</button>
              <button type="button" className="secondary" onClick={() => {
                if (saveLock.current || loading) return;
                setBulkText(""); setBulkPreview(null); setBulkConfirmed(false); setBulkError("");
              }}>貼り付けをキャンセル</button></div>
            {bulkError && <p role="alert" className="error">{bulkError}</p>}
            {bulkPreview && <div className="shopify-url-paste-preview">
              <p className="shopify-url-text">適用先：<strong>{loaded.snapshot.storeKey}</strong>。サムネイルと各URLの対応を確認してください。</p>
              <ol className="shopify-url-list">{rows.map((row, index) => <li key={row.imageId} className="shopify-url-paste-row">
                <LocalPreview key={`${loaded.version}-${row.imageId}`} image={localImages.get(row.imageId)!} />
                <div><strong>{index + 1}枚目・{localImages.get(row.imageId)!.original_filename}</strong><small>画像UUID：{row.imageId}</small>
                  <p className="shopify-url-text">{bulkPreview[index] || "空行：この画像のURLとaltを解除します"}</p></div>
              </li>)}</ol>
              <label className="shopify-url-check"><input type="checkbox" checked={bulkConfirmed} onChange={event => {
                if (!saveLock.current && !loading) setBulkConfirmed(event.target.checked);
              }} />対象ストア・画像との対応、空行でURLとaltが解除されることを確認しました</label>
              <button type="button" disabled={!bulkConfirmed} onClick={applyPaste}>入力欄へ適用</button>
            </div>}
          </section>
        </fieldset>
        <div className="shopify-url-save">
          <p className="muted" id="shopify-save-reason">{saving ? "保存結果が返るまで、この画面でお待ちください。" : saveReason || "表示中ストアの全画像設定をまとめて保存します。"}</p>
          <button type="button" disabled={saving || !!saveReason} aria-describedby="shopify-save-reason" onClick={() => void save()}>
            {saving ? "保存中…" : "設定を保存"}</button>
        </div>
      </>}
    </>}
    <dialog ref={discardDialog} className="confirm-dialog" aria-labelledby="shopify-discard-heading"
      onCancel={event => { event.preventDefault(); cancelDiscard(); }}>
      <h3 id="shopify-discard-heading">未保存の入力を破棄しますか？</h3>
      <p>個別入力、一括貼り付けの内容、画像対応の確認は保存されていません。必要な内容を控えてから進んでください。</p>
      {action?.kind === "load" && <p className="shopify-url-text">読み込むストア：{action.store}</p>}
      <div className="detail-actions"><button type="button" className="secondary" autoFocus onClick={cancelDiscard}>編集を続ける</button>
        <button type="button" onClick={confirmDiscard}>破棄して進む</button></div>
    </dialog>
  </section>;
}
