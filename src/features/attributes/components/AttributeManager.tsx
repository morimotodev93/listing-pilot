import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  createAttributeDefinition, listAttributeDefinitions, setAttributeArchived, updateAttributeDefinition,
} from "../../../lib/api/attributes";
import { createAttributeGroup, listAttributeGroups, setAttributeGroupArchived, updateAttributeGroup } from "../../../lib/api/attributeGroups";
import { ConfirmDialog } from "../../../components/ConfirmDialog";
import { RequestError } from "../../../lib/api/request";
import type { AttributeDataType, AttributeDefinition, AttributeGroup } from "../types";
import { DEFAULT_GROUP_ID } from "../types";
import "../attributes.css";

export function AttributeManager({ onChanged }: { onChanged: () => void }) {
  const [groups, setGroups] = useState<AttributeGroup[]>([]);
  const [selectedGroupId, setSelectedGroupId] = useState(DEFAULT_GROUP_ID);
  const [showArchivedGroups, setShowArchivedGroups] = useState(false);
  const [newGroupName, setNewGroupName] = useState("");
  const [groupName, setGroupName] = useState("");
  const [confirmation, setConfirmation] = useState<{ kind: "switch"; id: string } | { kind: "archive"; group: AttributeGroup } | { kind: "refresh" } | { kind: "create"; name: string } | null>(null);
  const mounted = useRef(true);
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
  const selectedGroup = groups.find(g => g.id === selectedGroupId);
  const fieldsDisabled = busy || !ready || !selectedGroup || !!selectedGroup.archivedAt;
  const dirty = !!(name || unit || dataType !== "text" || newGroupName || (selectedGroup && groupName !== selectedGroup.name) || (editor && editName !== editor.name));
  function createGroup(value: string) {
    void save(() => createAttributeGroup({ name: value }), "グループを作成しました。", result => {
      resetForms(); setSelectedGroupId(result.id); setGroupName(result.name);
    });
  }
  function validGroupName(value: string) {
    if (!value.trim() || [...value.trim()].length > 80 || /[\x00-\x1f\x7f-\x9f]/.test(value)) {
      setError("グループ名は1〜80文字で入力し、改行・制御文字は使わないでください。"); return false;
    }
    return true;
  }
  function resetForms() { setName(""); setUnit(""); setDataType("text"); setNewGroupName(""); setEditor(null); }
  function selectGroup(id: string) {
    resetForms(); setSelectedGroupId(id); setGroupName(groups.find(g => g.id === id)?.name ?? "");
  }

  useEffect(() => {
    let cancelled = false;
    mounted.current = true;
    Promise.all([listAttributeGroups(), listAttributeDefinitions()]).then(([allGroups, items]) => {
      if (!cancelled) {
        setGroups(allGroups); setAttributes(items); setReady(true);
        setGroupName(allGroups.find(g => g.id === DEFAULT_GROUP_ID && !g.archivedAt)?.name ?? "");
      }
    }).catch((error: unknown) => {
      if (!cancelled) setError(error instanceof Error ? error.message : "項目を取得できませんでした。");
    }).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; mounted.current = false; };
  }, []);

  async function refresh() {
    if (operation.current || loading) return;
    operation.current = true;
    setLoading(true); setError(""); setNotice("");
    try {
      const [allGroups, items] = await Promise.all([listAttributeGroups(), listAttributeDefinitions()]);
      if (!mounted.current) return;
      setGroups(allGroups); setAttributes(items); setReady(true); resetForms();
      setGroupName(allGroups.find(g => g.id === selectedGroupId)?.name ?? "");
      onChanged();
    } catch (error: unknown) {
      if (!mounted.current) return;
      setReady(false); setError(error instanceof Error ? error.message : "項目を取得できませんでした。");
    } finally { operation.current = false; if (mounted.current) setLoading(false); }
  }

  async function save(action: () => Promise<AttributeDefinition | AttributeGroup>, message: string, onSaved?: (result: AttributeDefinition | AttributeGroup) => void) {
    if (operation.current || loading || !ready) return;
    operation.current = true;
    setSaving(true); setError(""); setNotice("");
    try {
      const result = await action();
      if (!mounted.current) return;
      onChanged();
      setNotice(message);
      // Re-read the canonical order; never retry a mutation on refresh failure.
      setReady(false);
      try {
        const [allGroups, items] = await Promise.all([listAttributeGroups(), listAttributeDefinitions()]);
        if (!mounted.current) return;
        setGroups(allGroups); setAttributes(items); setReady(true);
        // Compare with the name shown before this operation, not the refreshed name.
        if (groupName === selectedGroup?.name) {
          setGroupName(allGroups.find(g => g.id === selectedGroupId)?.name ?? groupName);
        }
        onSaved?.(result);
      } catch {
        if (!mounted.current) return;
        setNotice("");
        setError("変更は保存されましたが、一覧の再読み込みに失敗しました。最新の状態を確認するには、再読み込みしてください。");
      }
    } catch (error: unknown) {
      if (!mounted.current) return;
      if (error instanceof RequestError && ["stale_data", "attribute_not_found", "attribute_group_not_found", "attribute_group_archived"].includes(error.code)) {
        setReady(false);
        setError("項目は別の操作で変更・削除されています。入力内容は保存されませんでした。必要な入力を控え、「項目一覧を再読み込み」で最新の内容を確認してから編集し直してください。");
      } else {
        setError((error instanceof Error ? error.message : "項目を保存できませんでした。") + " 入力は保持しています。必要に応じて項目一覧を再読み込みしてください。");
      }
    } finally {
      operation.current = false;
      if (mounted.current) { dialog.current?.close(); setArchiveTarget(null); setSaving(false); }
    }
  }

  function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void save(() => createAttributeDefinition({ groupId: selectedGroupId, name, dataType, unit: dataType === "number" ? unit : null }), "項目を作成しました。", () => {
      setName(""); setUnit("");
    });
  }

  function cancelArchive() {
    if (operation.current) return;
    dialog.current?.close(); setArchiveTarget(null);
  }

  return <section className="attribute-manager" aria-labelledby="attributes-heading" aria-busy={busy}>
    <div className="section-heading"><div>
      <h2 id="attributes-heading">グループ別カスタム商品項目</h2>
      <p className="muted">商品ごとに1つのグループを選び、そのグループ専用の項目を使います。</p>
    </div></div>
    {notice && <p className="success" role="status">{notice}</p>}
    {error && <p className="error" role="alert">{error}</p>}
    <div className="group-settings">
      <label htmlFor="attribute-group">管理するグループ</label>
      <select id="attribute-group" value={selectedGroupId} disabled={busy || !ready || !!editor || !!archiveTarget || !!confirmation} onChange={e => {
        if (operation.current) return;
        const id = e.target.value;
        if (dirty) setConfirmation({ kind: "switch", id }); else selectGroup(id);
      }}>
        {groups.filter(g => showArchivedGroups || !g.archivedAt || g.id === selectedGroupId).map(g => <option key={g.id} value={g.id}>{g.name}{g.archivedAt ? "（非表示）" : ""}</option>)}
      </select>
      <label className="attribute-filter"><input type="checkbox" checked={showArchivedGroups} disabled={busy || !!editor || !!archiveTarget || !!confirmation} onChange={e => setShowArchivedGroups(e.target.checked)} />非表示のグループも表示する</label>
      <form onSubmit={event => {
        event.preventDefault();
        if (!validGroupName(newGroupName)) return;
        if (name || unit || dataType !== "text" || (selectedGroup && groupName !== selectedGroup.name)) setConfirmation({ kind: "create", name: newGroupName });
        else createGroup(newGroupName);
      }}><fieldset disabled={busy || !ready || !!editor || !!archiveTarget || !!confirmation}>
        <label htmlFor="new-group-name">新しいグループ名</label>
        <input id="new-group-name" value={newGroupName} onChange={e => setNewGroupName(e.target.value)} required />
        <small>1〜80文字。改行・制御文字は使えません。</small>
        <button type="submit">グループを作成</button>
      </fieldset></form>
      {selectedGroup && <form onSubmit={event => {
        event.preventDefault();
        if (!validGroupName(groupName)) return;
        void save(() => updateAttributeGroup({ id: selectedGroup.id, name: groupName, expectedUpdatedAt: selectedGroup.updatedAt }), "グループ名を変更しました。", result => setGroupName(result.name));
      }}><fieldset disabled={busy || !ready || !!editor || !!archiveTarget || !!confirmation}>
        <label htmlFor="group-name">選択中グループの名前</label>
        <input id="group-name" value={groupName} onChange={e => setGroupName(e.target.value)} required />
        <div className="actions"><button type="submit" disabled={groupName === selectedGroup.name}>グループ名を保存</button>
          <button type="button" className="secondary" disabled={selectedGroup.id === DEFAULT_GROUP_ID} onClick={() => {
            if (selectedGroup.archivedAt) void save(() => setAttributeGroupArchived({ id: selectedGroup.id, archived: false, expectedUpdatedAt: selectedGroup.updatedAt }), "グループを復元しました。");
            else setConfirmation({ kind: "archive", group: selectedGroup });
          }}>{selectedGroup.archivedAt ? "グループを復元" : "グループを非表示"}</button></div>
      </fieldset></form>}
      {selectedGroup?.archivedAt && <p className="muted">このグループは非表示です。項目を編集するにはグループを復元してください。</p>}
    </div>
    <form className="attribute-create" onSubmit={create} aria-labelledby="attribute-create-heading">
      <h3 id="attribute-create-heading">新しい項目</h3>
      <fieldset disabled={fieldsDisabled || !!editor || !!confirmation}>
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
      <caption className="sr-only">グループ別カスタム商品項目一覧</caption>
      <thead><tr><th scope="col">項目名</th><th scope="col">型</th><th scope="col">単位</th><th scope="col">状態</th><th scope="col">操作</th></tr></thead>
      <tbody>{attributes.filter(a => a.groupId === selectedGroupId && (showArchived || a.archivedAt === null)).map(attribute => <tr key={attribute.id}>
        <td>{attribute.name}</td><td>{attribute.dataType === "text" ? "Text" : "Number"}</td><td>{attribute.unit ?? "—"}</td><td>{attribute.archivedAt ? "非表示" : "表示中"}</td>
        <td><div className="attribute-actions">
          <button type="button" className="secondary" disabled={fieldsDisabled || !!editor || !!confirmation} aria-label={`${attribute.name}の名前を変更`} onClick={() => {
            if (operation.current) return;
            setEditor(attribute); setEditName(attribute.name); setNotice("");
          }}>名前を変更</button>
          <button type="button" className="secondary" disabled={fieldsDisabled || !!editor || !!confirmation} aria-label={`${attribute.name}を${attribute.archivedAt ? "復元する" : "非表示にする"}`} onClick={() => {
            if (operation.current) return;
            if (attribute.archivedAt) {
              void save(() => setAttributeArchived({ id: attribute.id, archived: false, expectedUpdatedAt: attribute.updatedAt }), "項目を復元しました。");
            } else {
              setArchiveTarget(attribute); dialog.current?.showModal();
            }
          }}>{attribute.archivedAt ? "復元する" : "非表示にする"}</button>
        </div></td>
      </tr>)}</tbody>
    </table>{attributes.filter(a => a.groupId === selectedGroupId && (showArchived || a.archivedAt === null)).length === 0 && <p className="muted">表示する項目はありません。</p>}</div> : null}
    {editor && <form className="attribute-edit" onSubmit={event => {
      event.preventDefault();
      void save(() => updateAttributeDefinition({ id: editor.id, name: editName, expectedUpdatedAt: editor.updatedAt }), "項目名を変更しました。", () => setEditor(null));
    }}>
      <fieldset disabled={busy || !!selectedGroup?.archivedAt}>
        <label htmlFor="attribute-edit-name">「{editor.name}」の新しい名前</label>
        <input id="attribute-edit-name" value={editName} onChange={e => setEditName(e.target.value)} required autoFocus />
        <small>型：{editor.dataType === "text" ? "Text" : "Number"} / 単位：{editor.unit ?? "なし"}（変更不可）</small>
        <div className="actions"><button type="submit" disabled={!ready}>名前を保存</button>
          <button type="button" className="secondary" onClick={() => setEditor(null)}>名前変更をキャンセル</button></div>
      </fieldset>
    </form>}
    <button type="button" className="secondary refresh" disabled={busy || !!archiveTarget || !!confirmation} onClick={() => { if (dirty) setConfirmation({ kind: "refresh" }); else void refresh(); }}>項目一覧を再読み込み</button>
    {confirmation && <ConfirmDialog title={confirmation.kind === "archive" ? `「${confirmation.group.name}」を非表示にしますか？` : "未保存の入力を破棄しますか？"}
      confirmLabel={confirmation.kind === "archive" ? "グループを非表示にする" : "未保存の入力を破棄して続ける"}
      onCancel={() => setConfirmation(null)} onConfirm={() => {
        const target = confirmation; setConfirmation(null);
        if (target.kind === "switch") selectGroup(target.id);
        else if (target.kind === "refresh") void refresh();
        else if (target.kind === "create") createGroup(target.name);
        else void save(() => setAttributeGroupArchived({ id: target.group.id, archived: true, expectedUpdatedAt: target.group.updatedAt }), "グループを非表示にしました。");
      }}>
      <p>{confirmation.kind === "archive" ? "グループを非表示にしても、所属するフィールド定義や保存済みの値は削除されません。"
        : confirmation.kind === "create" ? "グループの作成に成功すると、ほかの未保存入力は破棄され、作成したグループに切り替わります。作成に失敗した場合は入力を保持します。保存済みのデータは変更しません。"
        : "グループ名や項目フォームの未保存入力を破棄します。保存済みのデータは変更しません。"}</p>
    </ConfirmDialog>}
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
