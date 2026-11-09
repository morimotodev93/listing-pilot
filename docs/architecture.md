# アーキテクチャ

モジュラーモノリスを採用します。フロントエンドはFeature単位、Rustは単一crate内の
モジュールで整理し、接続確認とSQLiteによる商品管理を実装しています。

## React

- `src/app/App.tsx`：アプリ全体の組み立てと基本レイアウト。
- `src/features/connection/`：接続確認UIとローカルな表示状態。
- `src/lib/api/index.ts`：UIが利用する非同期APIの窓口。Tauriの `invoke` をここに集約。
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

接続確認用の `greet` は引数を持たず `String` を返します。商品用の4つのCommandを追加し、
Commandsは薄い入口にし、検証・エラー変換とRust内部モジュールへの委譲を担います。

## Rust

今後、ファイル操作、画像処理、バッチ処理、SQLiteアクセス、AI API通信を担当します。
単一crateを維持し、機能が必要になった時点で内部モジュールを追加します。
永続化はSQLite（rusqlite / bundled）を使用し、DBアクセスはRust側に集約します。

## 商品管理の責務と通信

```text
ProductList / ProductForm → lib/api/products.ts → Tauri invoke
  → commands/product.rs → services/product_service.rs
  → infrastructure/db/product_repository.rs → SQLite
```

- `features/products/types.ts`：IPCで受け渡す商品型。`hooks/useProducts.ts`：一覧取得と再読み込み。古い応答の反映を防止。
- `commands/product.rs`：4つのIPC入口。`spawn_blocking` でDB処理をワーカースレッドへ委譲。
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
