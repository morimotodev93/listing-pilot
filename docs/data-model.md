# 商品データモデル（Phase 1）

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
画像やジョブのテーブルはまだありません。将来は安定した商品UUIDを外部キーとして関連付けます。

## 保存先とマイグレーション

`app.path().app_local_data_dir()` / `listing-pilot.sqlite3`。
Windowsでは通常 `%LOCALAPPDATA%\com.listingpilot.app\listing-pilot.sqlite3`。
初回のみ保存先とDBを作成します。既存ファイルは保持し、初期化失敗を通知します。
各接続でforeign_keys=ON、busy_timeout=5秒を設定します。

`PRAGMA user_version` が0なら、IMMEDIATEトランザクション内で初期DDLとバージョン1を適用します。
1なら何も変更しません。1より新しいDBは旧アプリで開かず初期化エラーにします。
失敗時はDDLとバージョン更新をロールバックします。将来の変更は番号付きSQLを追加し、
同じトランザクション内で順に適用する方式を拡張します。適用済みのSQLは変更しません。

## 保存と復旧

保存成功はSQLiteの書き込み完了後に返します。UIはその後一覧を再取得します。
本番DBはGit対象外です。テストはメモリDBまたはUUID付き一時ディレクトリのDBだけを使います。
開発版と配布版は同じアプリ識別子なら保存先を共有するため注意してください。

バックアップはアプリを完全終了した状態でDBファイルを別の場所にコピーします。
復旧時は終了状態で現在のDBを退避し、バックアップを元の保存先へコピーしてから起動します。
DB破損や将来バージョンを検出しても自動削除・上書き・初期化しません。
権限・空き容量などを確認し、必要に応じてバックアップから復旧してください。
自動バックアップ、ダウングレード、暗号化、クラウド同期は未実装です。
