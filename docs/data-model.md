# 商品データモデル（Phase 3.5）

商品編集は既存の `updated_at` を楽観ロックのトークンとして使用します。
フォーム開始時の値を `expectedUpdatedAt` として渡し、IDと更新日時を条件に単一SQLで更新します。
一致しなければ `stale_data`、商品自体がなければ `product_not_found` とし、競合時は全フィールド・更新日時を保持します。
更新成功時の日時は現在UTCと「前回日時 + 1ns」の大きい方です。画像操作では商品更新日時を変更しません。
スキーマバージョン2・既存マイグレーション・一意制約は変更しません。

商品削除は、読取トランザクションで取得した商品全フィールド・正規順序の画像IDを確認スナップショットとします。
商品情報全体の一致により、更新日時が変わらないSKU・商品名などの変更も拒否します。
共通permit下で全管理画像の事前検査を行い、単一IMMEDIATE内で商品と画像メタデータを再照合します。
各画像を商品ID・画像IDで1件ずつDELETEし、最後に商品ID指定で1件DELETEします。
関連参照がないことを検査してCOMMITし、途中失敗は全件ロールバックします。ON DELETE RESTRICTは維持します。
DB確定後に各画像を非置換隔離し、対象数＝隔離済み＋欠落＋保留を返します。0枚の商品も削除できます。
COMMIT失敗後はautocommit・商品ID・全画像ID・全相対パスを同一の読取スナップショットで再照会し、
削除確定を確認できたときだけ隔離します。不明・不整合は `product_deletion_unknown` で再起動確認を案内します。
新しいテーブル・列・マイグレーション・削除履歴は追加しません。

画像の1枚削除は `product_id` と `id` で対象を限定し、変更件数1を確認します。
同一IMMEDIATEトランザクションで残存画像を `sort_order, created_at, id` 順に0から連番化し、
商品・他画像のメタデータと `products.updated_at` は保持します。最後の画像も削除できます。
不在・別商品の画像・不正UUIDは `image_not_found` を返します。
スキーマバージョンは2のまま、既存SQL・一意制約を維持し、新規テーブル・削除履歴は追加しません。

DB確定後の管理画像は `media/quarantine/deleted/{商品UUID}/{画像UUID}.{ext}` へ非置換移動します。
返却契約は `deleted: true`、`affected_image_count: 1` と、隔離済み・欠落・隔離保留の各件数です。
隔離失敗でもDB削除は維持し、元の管理画像を保持します。参照を失ったまま終了した画像は次回起動時に
`media/quarantine/{UUID}.recovered` へ回収され得ます。削除履歴を保存しないため必ずdeletedに分類する保証はありません。
COMMIT結果の確認不能時は成功を返さず、管理画像を動かしません。
保護領域はAsset Protocolで公開せず、アプリ内復元や自動消去も行いません。

## products

| 列 | SQLite型 | 制約・意味 |
| --- | --- | --- |
| id | TEXT | PRIMARY KEY / NOT NULL。Rust生成UUID v4。SKU変更でも不変 |
| sku | TEXT | NOT NULL / UNIQUE / COLLATE BINARY。前後空白を除去し空を拒否 |
| name | TEXT | NOT NULL。前後空白を除去し空を拒否 |
| brand | TEXT | 任意。空白のみならNULL |
| price_yen | INTEGER | 任意。0〜9,007,199,254,740,991の整数円 |
| description_note | TEXT | 任意。前後空白を除去し空ならNULL |
| created_at | TEXT | NOT NULL。Rust生成のUTC RFC3339日時（Z）。更新時に保持 |
| updated_at | TEXT | NOT NULL。Rust生成のUTC RFC3339日時（Z）。更新ごとに再設定 |

SKUは大文字小文字を区別します。`ABC` と `abc` は別SKUです。
Unicodeの正規化や全角半角変換はしません。SKU・商品名の入力正規化はRustの `trim()`、
DBではNOT NULL / UNIQUE / CHECKを使い、価格の整数型と範囲も防御します。
価格はJavaScriptの最大安全整数までに制限し、IPCと表示の精度を保ちます。
未入力の価格と0円は区別します。浮動小数点数による価格計算はありません。
UIはIntl APIで円とOSのローカルタイムゾーンの日時を表示します。

DDLの正本は `src-tauri/migrations/0001_create_products.sql` です。
商品UUIDを画像テーブルの外部キーとして利用します。ジョブのテーブルはまだありません。

## product_images

DDLの正本は、ユーザー作成の `src-tauri/migrations/0002_create_product_images.sql` です。Phase 2の実装では変更していません。

| 列 | SQLite型 | 制約・意味 |
| --- | --- | --- |
| id | TEXT | PRIMARY KEY / NOT NULL。画像UUID v4 |
| product_id | TEXT | NOT NULL、products(id)参照、ON DELETE RESTRICT |
| original_filename | TEXT | NOT NULL。選択元のファイル名のみ |
| relative_path | TEXT | NOT NULL / UNIQUE。`media/originals/{商品UUID}/{画像UUID}.{ext}` |
| mime_type | TEXT | JPEG・PNG・WebPに限定するCHECK |
| file_size_bytes | INTEGER | 正のバイト数 |
| width / height | INTEGER | それぞれ正の画素数 |
| sha256 | TEXT | 長さ64のCHECK。Rustが小文字16進数で生成 |
| sort_order | INTEGER | 非負。商品内の最大値+1（初回0）をINSERT時に決定 |
| created_at | TEXT | NOT NULL。Rust生成UTC RFC3339日時 |

Phase 3.1の並べ替えは既存 `sort_order` のみを単一IMMEDIATEトランザクションで0始まりの連番へ更新します。
一覧の正規順序は `ORDER BY sort_order, created_at, id`、その先頭をメイン画像とします。専用フラグ・列・マイグレーションは追加しません。
操作前の全ID順序との一致を確認し、変更後に不足・余分・重複・別商品のIDを受け付けません。
画像がない商品の空配列同士と、同じ順序への更新は安全に成功します。総画像数を20枚に制限せず、新規取り込みは引き続き最大 `sort_order` + 1へ追加します。
既存のSHA-256・相対パス・画像バイト列・その他メタデータと `products.updated_at` は保持します。

`UNIQUE(product_id, sha256)` により商品内の同一内容を拒否し、別商品への同じ画像は許可します。
`idx_product_images_product_order` は商品ID、表示順、日時、IDの順で一覧を取得するために使います。
SQLのUUID・相対パス・実データハッシュの妥当性はRustでも検証します。SQL自体がUUID形式や相対パスの安全性を保証するわけではありません。
`DuplicateSku` はproducts.skuの一意制約だけに変換し、画像の重複や相対パス競合と区別します。

## 保存先とマイグレーション

`app.path().app_local_data_dir()` / `listing-pilot.sqlite3`。
Windowsでは通常 `%LOCALAPPDATA%\com.listingpilot.app\listing-pilot.sqlite3`。
初回のみ保存先とDBを作成します。既存ファイルは保持し、初期化失敗を通知します。
各接続でforeign_keys=ON、busy_timeout=5秒を設定します。

`PRAGMA user_version` が0なら、IMMEDIATEトランザクション内で0001→0002とバージョン2まで適用します。
1なら0002だけを適用し、2なら変更しません。3以上は初期化エラーにし、商品・画像を変更しません。
失敗時はDDLとバージョン更新をロールバックします。将来の変更は番号付きSQLを追加し、
同じトランザクション内で順に適用する方式を拡張します。適用済みのSQLは変更しません。

## 保存と復旧

保存成功はSQLiteの書き込み完了後に返します。UIはその後一覧を再取得します。
本番DBはGit対象外です。テストはメモリDBまたはUUID付き一時ディレクトリのDBだけを使います。
開発版と配布版は同じアプリ識別子なら保存先を共有するため注意してください。

バックアップはアプリの全プロセスを完全終了した状態で、DBファイルと `media/` 全体を同時点のセットとして別の場所にコピーします。
復旧時は終了状態で現在のDBと `media/` をセットで退避し、同じバックアップセットを元の保存先へコピーしてから起動します。
SQLiteのジャーナル等が残る異常終了直後のコピーは避け、正常終了を確認してください。
DB破損や将来バージョンを検出しても自動削除・上書き・初期化しません。
権限・空き容量などを確認し、必要に応じてバックアップから復旧してください。
自動バックアップ、ダウングレード、暗号化、クラウド同期は未実装です。

## 画像ファイルと障害復旧

`media/.staging/{UUID}.tmp` に上限付きでコピーし、内容・形式・画素数・デコードを検証します。
SHA-256で重複確認後、同一ボリュームの `media/originals/` にrenameし、SQLiteへ1ファイルずつautocommitで登録します。
SQLiteとファイル操作全体を一つのトランザクションにはできないため、DB登録失敗時は相対パス参照を再確認します。
未参照と確認できた新規ファイルだけを削除し、参照ありなら保持、確認不能なら次回起動まで保持します。

起動時は参照一覧を取得できた場合だけ整合性を検査します。未完了のstagingとDB未参照の通常画像を
`media/quarantine/{UUID}.recovered` にrenameし、完全削除しません。危険なリンクや不正なDBパスがあれば整理を停止します。
登録済みの欠落ファイルはDBに残し、一覧APIでプレビューなしを返します。UIは画像がない旨とバックアップ確認を表示します。
隔離画像の自動再登録や元のパスへの自動復元は行いません。元ファイルから再取り込みするか、DBとmediaを揃えたバックアップで復旧してください。
