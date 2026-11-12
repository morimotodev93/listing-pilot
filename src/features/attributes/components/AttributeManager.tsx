import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  createAttributeDefinition, listAttributeDefinitions, setAttributeArchived, updateAttributeDefinition,
} from "../../../lib/api/attributes";
import { RequestError } from "../../../lib/api/request";
import type { AttributeDataType, AttributeDefinition } from "../types";
import "../attributes.css";

export function AttributeManager() {
  const [attributes, setAttributes] = useState<AttributeDefinition[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [ready, setReady] = useState(false);
  const [showArchived, setShowArchived] = useState(false);
  const [name, setName] = useState("");
  const [dataType, setDataType] = useState<AttributeDataType>("text");
  const [unit, setUnit] = useState("");
  const [editor, setEditor] = useState<AttributeDefinition | null>(null);
  const [editName, setEditName] = useState("");
  const [archiveTarget, setArchiveTarget] = useState<AttributeDefinition | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const operation = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const busy = loading || saving;

  useEffect(() => {
    let cancelled = false;
    listAttributeDefinitions().then(items => {
      if (!cancelled) { setAttributes(items); setReady(true); }
    }).catch((error: unknown) => {
      if (!cancelled) setError(error instanceof Error ? error.message : "項目を取得できませんでした。");
    }).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, []);

  async function refresh() {
    if (operation.current || loading) return;
    operation.current = true;
    setLoading(true); setError(""); setNotice("");
    try {
      setAttributes(await listAttributeDefinitions()); setReady(true); setEditor(null);
    } catch (error: unknown) {
      setReady(false); setError(error instanceof Error ? error.message : "項目を取得できませんでした。");
    } finally { operation.current = false; setLoading(false); }
  }

  async function save(action: () => Promise<AttributeDefinition>, message: string, onSaved?: () => void) {
    if (operation.current || loading || !ready) return;
    operation.current = true;
    setSaving(true); setError(""); setNotice("");
    try {
      await action();
      setNotice(message);
      // Re-read the canonical order; never retry a mutation on refresh failure.
      setReady(false);
      try {
        setAttributes(await listAttributeDefinitions());
        setReady(true);
        onSaved?.();
      } catch {
        setNotice("");
        setError("変更は保存されましたが、一覧の再読み込みに失敗しました。最新の状態を確認するには、再読み込みしてください。");
      }
    } catch (error: unknown) {
      if (error instanceof RequestError && ["stale_data", "attribute_not_found"].includes(error.code)) {
        setReady(false);
        setError("項目は別の操作で変更・削除されています。入力内容は保存されませんでした。必要な入力を控え、「項目一覧を再読み込み」で最新の内容を確認してから編集し直してください。");
      } else {
        setError(error instanceof Error ? error.message : "項目を保存できませんでした。");
      }
    } finally {
      dialog.current?.close(); setArchiveTarget(null);
      operation.current = false; setSaving(false);
    }
  }

  function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void save(() => createAttributeDefinition({ name, dataType, unit: dataType === "number" ? unit : null }), "項目を作成しました。", () => {
      setName(""); setUnit("");
    });
  }

  function cancelArchive() {
    if (operation.current) return;
    dialog.current?.close(); setArchiveTarget(null);
  }

  return <section className="attribute-manager" aria-labelledby="attributes-heading" aria-busy={busy}>
    <div className="section-heading"><div>
      <h2 id="attributes-heading">共通カスタム商品項目</h2>
      <p className="muted">カラーや重量など、共通で使う項目を設定します。商品ごとの値入力は今後対応します。</p>
    </div></div>
    {notice && <p className="success" role="status">{notice}</p>}
    {error && <p className="error" role="alert">{error}</p>}
    <form className="attribute-create" onSubmit={create} aria-labelledby="attribute-create-heading">
      <h3 id="attribute-create-heading">新しい項目</h3>
      <fieldset disabled={busy || !ready}>
        <label htmlFor="attribute-name">項目名 <span>必須</span></label>
        <input id="attribute-name" value={name} onChange={e => setName(e.target.value)} required aria-describedby="attribute-name-help" />
        <small id="attribute-name-help">80文字以内。前後の空白は除去し、改行・制御文字は使えません。</small>
        <label htmlFor="attribute-type">データ型</label>
        <select id="attribute-type" value={dataType} onChange={e => setDataType(e.target.value === "number" ? "number" : "text")}>
          <option value="text">Text（文字列）</option><option value="number">Number（数値）</option>
        </select>
        {dataType === "number" && <><label htmlFor="attribute-unit">単位（任意）</label>
          <input id="attribute-unit" value={unit} onChange={e => setUnit(e.target.value)} aria-describedby="attribute-unit-help" />
          <small id="attribute-unit-help">例：g、kg、mL。20文字以内。</small></>}
        <small>作成後はデータ型と単位を変更できません。</small>
        <div className="actions"><button type="submit">{saving ? "保存中…" : "項目を作成"}</button></div>
      </fieldset>
    </form>
    <div className="attribute-list-heading"><h3>登録済み項目</h3>
      <label className="attribute-filter"><input type="checkbox" checked={showArchived} disabled={busy || !!editor} onChange={e => setShowArchived(e.target.checked)} />非表示の項目も表示する</label>
    </div>
    {loading ? <p role="status">項目を読み込んでいます…</p> : ready || attributes.length > 0 ? <div className="table-scroll"><table>
      <caption className="sr-only">共通カスタム商品項目一覧</caption>
      <thead><tr><th scope="col">項目名</th><th scope="col">型</th><th scope="col">単位</th><th scope="col">状態</th><th scope="col">操作</th></tr></thead>
      <tbody>{attributes.filter(a => showArchived || a.archivedAt === null).map(attribute => <tr key={attribute.id}>
        <td>{attribute.name}</td><td>{attribute.dataType === "text" ? "Text" : "Number"}</td><td>{attribute.unit ?? "—"}</td><td>{attribute.archivedAt ? "非表示" : "表示中"}</td>
        <td><div className="attribute-actions">
          <button type="button" className="secondary" disabled={busy || !ready || !!editor} aria-label={`${attribute.name}の名前を変更`} onClick={() => {
            if (operation.current) return;
            setEditor(attribute); setEditName(attribute.name); setNotice("");
          }}>名前を変更</button>
          <button type="button" className="secondary" disabled={busy || !ready || !!editor} aria-label={`${attribute.name}を${attribute.archivedAt ? "復元する" : "非表示にする"}`} onClick={() => {
            if (operation.current) return;
            if (attribute.archivedAt) {
              void save(() => setAttributeArchived({ id: attribute.id, archived: false, expectedUpdatedAt: attribute.updatedAt }), "項目を復元しました。");
            } else {
              setArchiveTarget(attribute); dialog.current?.showModal();
            }
          }}>{attribute.archivedAt ? "復元する" : "非表示にする"}</button>
        </div></td>
      </tr>)}</tbody>
    </table>{attributes.filter(a => showArchived || a.archivedAt === null).length === 0 && <p className="muted">表示する項目はありません。</p>}</div> : null}
    {editor && <form className="attribute-edit" onSubmit={event => {
      event.preventDefault();
      void save(() => updateAttributeDefinition({ id: editor.id, name: editName, expectedUpdatedAt: editor.updatedAt }), "項目名を変更しました。", () => setEditor(null));
    }}>
      <fieldset disabled={busy}>
        <label htmlFor="attribute-edit-name">「{editor.name}」の新しい名前</label>
        <input id="attribute-edit-name" value={editName} onChange={e => setEditName(e.target.value)} required autoFocus />
        <small>型：{editor.dataType === "text" ? "Text" : "Number"} / 単位：{editor.unit ?? "なし"}（変更不可）</small>
        <div className="actions"><button type="submit" disabled={!ready}>名前を保存</button>
          <button type="button" className="secondary" onClick={() => setEditor(null)}>名前変更をキャンセル</button></div>
      </fieldset>
    </form>}
    <button type="button" className="secondary refresh" disabled={busy} onClick={() => void refresh()}>項目一覧を再読み込み</button>
    <dialog ref={dialog} aria-labelledby="archive-attribute-heading" aria-describedby="archive-attribute-description" onCancel={event => { event.preventDefault(); cancelArchive(); }}>
      <h3 id="archive-attribute-heading">「{archiveTarget?.name}」を非表示にしますか？</h3>
      <p id="archive-attribute-description">項目の定義と登録済みの値は削除されません。後から復元できます。</p>
      <div className="actions"><button type="button" disabled={saving} onClick={() => {
        if (!archiveTarget) return;
        const target = archiveTarget;
        void save(() => setAttributeArchived({ id: target.id, archived: true, expectedUpdatedAt: target.updatedAt }), "項目を非表示にしました。");
      }}>{saving ? "保存中…" : "非表示にする"}</button>
        <button type="button" className="secondary" disabled={saving} onClick={cancelArchive}>キャンセル</button></div>
    </dialog>
  </section>;
}
