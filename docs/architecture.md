# アーキテクチャ

モジュラーモノリスを採用します。フロントエンドはFeature単位、Rustは単一crate内の
モジュールで整理し、接続確認、SQLiteによる商品管理、商品画像の取り込み・表示を実装しています。

## React

Phase 3.3の商品削除は `ProductDetail` → `lib/api/products.ts` → `commands/product.rs` →
`services/product_deletion_service.rs` の経路です。プレビューと削除はblocking worker上で処理します。
`get_product_delete_preview({ productId })` は読取トランザクションで商品全体と正規順序の全画像IDを返します。
`delete_product({ input })` のinputは `productId`、`expectedUpdatedAt`、`expectedImageIds`、`expectedProduct` です。
商品全体の一致と重複のないID順序を事前・IMMEDIATE内で検証し、全画像メタデータも事前検査時と照合します。
permit → DBスナップショット → Mutex解放・全ファイル検査 → IMMEDIATE再検証 → 全画像DELETE → 商品DELETE →
参照なし確認 → COMMIT → Mutex解放・全画像隔離、の順を守ります。画像0枚でも共通permitを取得します。
COMMITエラー後はautocommitと全参照を再照会し、曖昧な状態では隔離も成功返却もしません。
各画像の隔離失敗は保留に集計し、残りを継続します。削除済みDBレコードは復活させません。
Reactは最新プレビューの確認dialogを表示し、確定時だけ削除APIを呼びます。失敗時は確認を閉じ、自動再試行しません。
同期refと `ProductImages.onBusyChange` により操作を制御し、削除確認・処理中は編集・遷移・画像操作を停止します。
成功後の一覧再取得・保留警告は `ProductList` が担当します。

Phase 3.2の画像削除は `ProductImages` の確認dialog → `lib/api/images.ts` の
`deleteProductImage({ productId, imageId })` → `delete_product_image` → `image_service::delete` の経路です。
キャンセルはIPCを送信せず、確定中は既存の同期refガードで取り込み・並べ替え・再削除を防ぎます。
成功後は一覧を再取得し、隔離保留は通常エラーとは別の警告で通知します。

Rustは共通ImportPermit取得 → UUID・商品と画像の関連確認 → ファイル検査 → IMMEDIATE内の最新画像再取得 →
商品IDと画像IDで1件削除 → 正規順序に従い残存画像を連番化 → COMMIT → ファイル隔離の順に処理します。
ファイル検査・移動時にはDB Mutexを解放し、permitは隔離の終了まで保持します。
COMMITの失敗だけはautocommit状態と画像ID・相対パスの双方の最新参照を再確認します。
参照が残れば移動せずエラー、参照なしなら隔離、DB状態不明ならファイルを保持して再起動確認を案内します。

Windowsの隔離は既存 `place_new` の `MoveFileExW(flags=0)` を再利用し、置換・別ボリュームコピーを禁止します。
パスの全要素のリンク・reparse pointを拒否し、NotFoundだけを欠落として扱います。
Windows以外の削除隔離は原子的な非置換移動が未実装のため保留を返します。取り込み経路は変更しません。
起動時のOSロック下DB再照会と `originals/`・`.staging/` の検査は維持し、`quarantine/deleted/` を再処理しません。
詳細は [ADR 0005](decisions/0005-safe-product-and-image-deletion.md) を参照してください。

- `src/app/App.tsx`：アプリ全体の組み立てと基本レイアウト。
- `src/features/connection/`：接続確認UIとローカルな表示状態。
- `src/lib/api/`：UIが利用する非同期APIの窓口。`request.ts` がIPCと日本語エラー変換、`images.ts` が検証済み画像パスのURL変換を担当。
- UI、フォーム、画面遷移、進捗表示を担当します。Rustが扱うファイルやDBを直接操作しません。
- 共通UIが必要になった段階で `src/components/` を作成します。

Reactコンポーネントは `getGreeting(): Promise<string>` を利用し、Tauri APIを直接
importしません。将来Webデモが必要になった際は、この窓口の実装を同じ契約のWeb用Adapterに
差し替えます。現時点ではAdapterの仕組みやモックは追加しません。

## Tauri Commands

- `src-tauri/src/commands/mod.rs`：フロントエンドとRustの境界。
- `src-tauri/src/lib.rs`：アプリの初期化とCommand登録。
- `src-tauri/src/main.rs`：デスクトップのエントリーポイント。
- `src-tauri/capabilities/default.json`：メインウィンドウの基本権限。追加プラグイン権限なし。

接続確認用の `greet` は引数を持たず `String` を返します。商品用の6つのCommandを公開し、
Commandsは薄い入口にし、検証・エラー変換とRust内部モジュールへの委譲を担います。

## Rust

ファイル操作、画像検証、SQLiteアクセスを担当します。バッチ処理・AI API通信は将来の対象です。
単一crateを維持し、機能が必要になった時点で内部モジュールを追加します。
永続化はSQLite（rusqlite / bundled）を使用し、DBアクセスはRust側に集約します。

## 商品管理の責務と通信

```text
ProductList / ProductForm → lib/api/products.ts → Tauri invoke
  → commands/product.rs → services/product_service.rs
  → infrastructure/db/product_repository.rs → SQLite
```

- `features/products/types.ts`：IPCで受け渡す商品型。`hooks/useProducts.ts`：一覧取得と再読み込み。古い応答の反映を防止。
- `commands/product.rs`：6つのIPC入口。`spawn_blocking` でDB処理をワーカースレッドへ委譲。
- `services/product_service.rs`：入力検証、UUID生成、UTC日時、商品登録・更新の調整。
- `domain/product.rs`：商品型、入力型、必須項目・価格の検証と空白正規化。
- `infrastructure/db/connection.rs`：保存先作成・接続・foreign_keys・5秒busy timeout。
- `infrastructure/db/migrations.rs`：トランザクションと `user_version` によるスキーマ管理。
- `infrastructure/db/product_repository.rs`：パラメータ化SQL、行の変換、件数による存在確認。
- `error.rs`：必須不足、SKU重複、価格不正、商品不在、DB操作、DB初期化のエラーコード。

アプリ起動時にPath APIで保存先を解決し、接続を1本作成します。接続はMutexで直列化します。
初期化に失敗した場合もUIは起動し、商品APIが `database_initialization` を返すため日本語で通知できます。
接続確認は独立して使えます。内部DBエラーは標準エラー出力に記録し、UIには固定メッセージだけを返します。
メモリDBへのフォールバックはありません。復旧後はアプリを再起動します。

DB操作をRustに限定することで、入力検証・一意制約・ファイルアクセスを一か所に保ちます。
UIから任意SQLは受け付けません。ORM、コネクションプール、Repository trait、DIは導入しません。
商品APIの関数を将来のWeb Adapterへの境界とし、今回はTauri実装のみとします。

## 現在の呼び出し経路

### 商品画像（Phase 2）

```text
ProductList → ProductDetail / ProductImages → lib/api/images.ts
  → commands/image.rs → services/image_service.rs
    → infrastructure/media_storage.rs（ファイル・デコード・ハッシュ）
    → infrastructure/db/image_repository.rs（SQLite）
  → 検証済みプレビューパス → convertFileSrc → Asset Protocol → img
```

- `domain/image.rs`：画像メタデータ、上限、取り込み結果の契約。
- `commands/image.rs`：商品UUIDのみを受け取り、Rustのdialog builderでファイルを選択。ダイアログと処理はasync Commandから `spawn_blocking` へ委譲。
- `services/image_service.rs`：個別取り込み、重複確認、DB登録と補償処理、起動時整合性検査の調整。
- `infrastructure/media_storage.rs`：サイズ制限付きコピー、内容検証、画素数・デコードメモリ制限、SHA-256、UUID相対パス、原子的rename、隔離。
- `infrastructure/db/image_repository.rs`：既存0002 SQLに対応。表示順はINSERT内のサブクエリで割り当て、同一商品内重複はDB制約でも拒否。
- `infrastructure/db/mod.rs`：共有 `Database` と短い `with()` 操作を共通化。画像コピー・ハッシュ・検証・プレビュー用ファイル確認の間はDB Mutexを保持しない。

起動順はDB接続・トランザクション内マイグレーション → 画像参照取得の事前確認 → メディアのOSファイルロック → ロック下で最新の画像参照を再取得 → 整合性検査 → Command公開です。
事前確認の参照一覧は破棄し、ロック取得前後に別プロセスが登録した画像を孤立ファイルと誤認しないようにします。
DB初期化・事前確認に失敗した場合は画像ディレクトリを作成・整理しません。ロック後の再取得に失敗した場合も隔離せずエラーを返します（ロックファイルや必要なディレクトリは作成され得ます）。画像初期化だけが失敗した場合も商品管理は利用可能です。
取り込み中はプロセス内のpermitで二重操作と整合性検査を拒否します。OSファイルロックは他プロセスの取り込み・隔離を防ぎ、終了・異常終了時に解放されます。

### 画像表示順（Phase 3.1）

`ProductImages` → `lib/api/images.ts` → `reorder_product_images` → `image_service::reorder` → `image_repository::update_order` の経路で更新します。
IPC引数は `productId`、`expectedImageIds`（操作前の全順序）、`orderedImageIds`（変更後の全順序）です。
既存一覧と同じ `ImagePreview` を返し、API層が検証済みプレビューパスをURLへ変換します。

`spawn_blocking` 内で既存の `begin_import()` / `ImportPermit` を共通の画像変更ガードとして取得し、その後DBへアクセスします。
OSロックの方式は維持します。単一接続の `Database::with_mut()` 内でIMMEDIATEトランザクションを開始し、商品存在・重複ID・現在順序との完全一致・変更後のID集合を検証します。
`invalid_image_order` と `stale_data` を区別し、不正入力・SQLエラーでは全更新をロールバックします。
保存結果の一覧はトランザクション内で読み、COMMIT成功後に返します。プレビュー用ファイル確認はDB Mutex解放後です。

UIは画像UUIDで対象を特定し、保存成功後にのみ順序と先頭の「メイン画像」表示を更新します。
同期的なrefガードで二重送信を防ぎ、保存中は取り込み・再読み込み・移動・詳細からの遷移を無効化します。
失敗時は日本語エラーを保持して最新一覧を再取得し、再取得失敗中は移動を停止します。
画像ファイル・メタデータ・商品更新日時・スキーマは変更しません。

Windowsの正式配置には `MoveFileExW`（flags=0）を使用し、配置先の存在確認と配置を一つのOS操作で行います。
既存ファイルの置換・別ボリュームへのコピーを許可せず、ステージングからの同一ボリューム移動を維持します。
事前の存在確認を削除したため、確認時のI/Oエラーを「存在しない」と扱うこともありません。失敗時のstagingは従来のDrop処理で整理します。

DBには相対パスを保存し、Rustで商品UUID・画像UUID・MIMEから期待するパスと完全一致することを確認します。
絶対パスはプレビュー可能な通常ファイルだけを返します。パスの各要素のシンボリックリンク・Windows reparse point（junctionを含む）を拒否します。
ファイルシステムを同一ユーザーの別プログラムが同時に改変する攻撃全般を防ぐ仕組みではありません。

Asset Protocolは `$APPLOCALDATA/media/originals/**/*` のみ、CSPは `img-src` に `asset:` と `http://asset.localhost` のみ追加します。
Capabilitiesは `core:default` のままで、フロントエンドにdialog・fsプラグイン権限を与えません。
Rustのdialog builderは、JS用dialog Commandが行う選択元ファイルの動的スコープ追加を呼びません。
詳細は [ADR 0004](decisions/0004-product-image-management.md) を参照してください。

### 接続確認

```text
接続確認ボタン
  → features/connection/ConnectionCheck.tsx
  → lib/api/index.ts の getGreeting()
  → invoke<string>("greet")
  → Rust commands::greet()
  → 返却文字列をReactのstateに保存
  → 画面のステータス領域に表示
```

通信失敗も同じステータス領域に表示します。ブラウザー単体ではTauriを呼び出さず、
デスクトップアプリでの確認を案内します。

## Phase 1の主要ディレクトリ

```text
src/
  app/ App.tsx, App.css
  features/
    connection/ ConnectionCheck.tsx
    products/
      components/ ProductList.tsx, ProductForm.tsx
      hooks/ useProducts.ts
      types.ts, products.css
  lib/api/ index.ts, products.ts
  main.tsx
src-tauri/
  migrations/ 0001_create_products.sql
  src/
    commands/ mod.rs, product.rs
    domain/ mod.rs, product.rs
    services/ mod.rs, product_service.rs
    infrastructure/
      mod.rs
      db/ mod.rs, connection.rs, migrations.rs, product_repository.rs
    lib.rs, main.rs, error.rs, tests.rs
  Cargo.toml, Cargo.lock, tauri.conf.json
docs/
  architecture.md, data-model.md, verification.md
  decisions/ 0001-initial-stack.md, 0002-sqlite-product-management.md
```
