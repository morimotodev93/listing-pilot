# 商品データモデル（Phase 2）

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
