# 0011: グループ専用のカスタムフィールドと所属変更時の値保全

- 日付：2026-11-16（日本時間）
- 状態：採用・Phase 4.5.1実装

## 決定と理由

A案（グループ専用方式）を採用する。商品と定義はそれぞれ必ず1グループに所属し、
商品には現在の所属グループの有効な定義だけを適用する。複数所属や共通定義との合成を導入せず、
編集対象・出品候補の判定を一致させる。ADR 0006の全商品共通定義をこの方式へ拡張する。

既定グループは固定UUID `7a8f0c3e-4d5b-4c6a-9e20-1234567890ab` で識別する。
Rustの `DEFAULT_GROUP_ID`、Migration 0005、旧フォームの互換フィルターで同じ値を使用する。
初期名は「既定」。名前変更後も同じUUIDで識別し、アーカイブは禁止する。物理削除APIは作らない。

## Schema Version 5と移行

0001〜0004は変更せず、0005を追加する。既存の単一IMMEDIATE内で0〜4を5へ順次移行し、
5は再適用してもデータ変更なし、6以上は拒否する。最後に `PRAGMA foreign_key_check` を検査し、
不整合または途中失敗ではDDL・データ・user_versionをすべてロールバックする。
本番DBの初期化・再生成・メモリDBへの退避は行わない。

`attribute_groups` はid・name・archived_at・created_at・updated_atを保持する。
名前は全グループでBINARY一意、Rust trim後1〜80 Unicode文字、制御文字・改行を拒否する。
SQLiteにも既存定義と同じtrim/lengthのCHECKを設ける。Unicodeの完全な検証はRustに集約する。

`products` は再構築しない。nullableなREFERENCES列を追加し、全既存行を既定グループへバックフィルする。
その後、INSERTとgroup_id UPDATEのトリガーでNULLを拒否する。外部キーで不在参照を拒否する。
列の `notnull` フラグは0だが、通常のSQL書き込みでNULL所属を作ることはできない。
商品所属件数の検査用にgroup_idのINDEXを追加する。所属先の有効性はRustで検証する。
外部キー有効時のADD COLUMNのDEFAULT制約に合わせた手順である。

定義は新テーブルへ全列と所属をコピーし、一意制約を `UNIQUE(group_id,name)` に変更する。
非表示の定義も制約対象で、別グループの同名だけを許可する。
値も新しい定義テーブルを参照する新テーブルへ全行コピーし、旧値テーブル→旧定義テーブルの順にDROP、
新定義→新値の順にRENAMEする。外部キーを無効にせず、ON DELETE RESTRICTを常に維持する。
商品ID・画像ID/メタデータ・定義ID/全列・値の全組み合わせと文字列を保持する。
画像テーブルと管理原本は変更しない。

SQLiteの制約は [ALTER TABLE](https://www.sqlite.org/lang_altertable.html) と
[Foreign Key Support](https://www.sqlite.org/foreignkeys.html) を参照。

## 保存とグループ管理IPC

- `list_attribute_groups()`：全グループを有効状態、created_at、id順で返す。既定の識別は固定UUID。
- `create_attribute_group({input:{name}})`：UUIDを生成し、有効状態で返す。
- `update_attribute_group({input:{id,name,expectedUpdatedAt}})`：名前変更。
- `set_attribute_group_archived({input:{id,archived,expectedUpdatedAt}})`：アーカイブ・復元。

返却DTOはcamelCase。名前はアーカイブ後も予約する。改名・状態変更はIMMEDIATE内で
取得・楽観ロック照合・更新を行う。updated_atは既存共通関数で前回+1ns以上に進める。
アーカイブでは既定UUIDと所属商品が1件以上あるグループを拒否する。件数確認と更新は同じトランザクション。
復元はarchived_atだけを解除し、定義や値を書き換えない。

商品DTOとProductInputは既存のsnake_caseに従ってgroup_idを追加する。
作成時の省略/nullは既定、更新時の省略/nullは現在の所属を保持する。
明示されたIDは存在と有効性を検証する。所属変更も商品・公開文章・値の既存IMMEDIATE内で行い、
expectedUpdatedAtの楽観ロックを維持する。所属変更自体でグループを新規作成することはない。

定義DTOと作成入力はcamelCaseのgroupIdを使用する。旧作成入力の省略/nullは既定へ割り当てる。
作成・改名・非表示/復元は所属先が有効な場合だけ許可し、確認と書き込みをIMMEDIATEで一体化する。
定義のgroup_idは作成後変更不可。更新入力は未知フィールドを拒否し、DBトリガーでも移動を拒否する。
UUID・型・単位・値を改名や非表示で変更しない。

## 値保全と旧フォーム互換

値の主キーは引き続き(product_id,attribute_id)。group_id列を追加しない。
別グループの値の併存を許可し、所属変更でDELETE・移動・変換しない。
元のグループに戻すと同じ定義UUIDの保存値を再利用できる。

attribute_value_serviceはトランザクション内の保存後商品を取得し、現在の有効グループを確定する。
editableDefinitionsの所属・非表示状態・更新トークン、重複ID・余分な値・型を全検証してから個別更新する。
別グループ項目の更新もクリアも拒否し、所属変更と旧グループ項目が混在したリクエストは商品ごと戻す。
非指定の値を一括削除しない。

編集スナップショットはproduct.group_idと現在グループの有効定義を返し、valuesには保全された全値を含める。
旧商品フォームの新規登録と旧項目設定の表示は固定既定UUIDに絞る。
商品編集はスナップショットの候補だけを送信し、所属を指定せず保持する。
保存失敗・競合時の入力保持と自動再送しない挙動は維持する。

## プレビュー・CSV・削除

Phase 4.4の共通候補生成で現在グループ、有効、値ありの順に絞り、既存の定義順を保つ。
警告・status・内部メモ非公開の契約は変更しない。Phase 4.5も同じ候補を使い、
product.csv / attributes.csv / images.csvの列・文字コード・順序を変更しない。
snapshotTokenには商品所属、全定義の所属・非表示状態、現在グループ全情報が含まれ、所属変更後の旧確認は拒否する。

商品削除スナップショットには所属を含むProductと全グループの値が入る。
既存の完全比較と対象商品限定DELETEで、古い確認を拒否し全値を削除する。
画像permit・OSロック・隔離・出力の非置換確定、CSP・Asset Protocol・Capabilities、依存ロックは維持する。

Phase 4.5.2にはグループ管理・商品選択UIを残す。定義の移動、複数所属、物理削除、一括出力、
販売先別ルールは今回実装しない。移行前は全アプリを終了しDBとmedia全体を一組でバックアップする。
スキーマ4までの旧バイナリで移行後DBを開かない。結果と限界は [検証記録](../verification.md) を参照。
