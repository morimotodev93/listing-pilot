# Listing Pilot

**写真から、出品準備完了まで。**

写真を起点とした出品準備を支援するWindowsデスクトップアプリです。Phase 1として、SQLiteによる商品登録・一覧・取得・編集に対応しています。
公式 `create-tauri-app` のTauri 2 / React / TypeScriptテンプレートから作成しています。

## 採用技術

- Tauri 2（単一Rust crate）
- Rust stable / `x86_64-pc-windows-msvc`
- React 19 / Vite 8 / TypeScript 7.0.2
- SQLite / rusqlite（bundled）。DBアクセスはRustに集約
- npm（依存関係の再現には `package-lock.json` と `src-tauri/Cargo.lock` を使用）

## 必要な開発環境

- Windows 10 / 11 x64
- Node.js 24 LTSとnpm 10以降（Node.js 22.12以降の22系も利用可）
- Visual Studioまたは[Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  の「C++によるデスクトップ開発」、MSVC x64/x86ビルドツール、Windows SDK。
  既存のVisual Studioに揃っていれば、Build Toolsの追加インストールは不要
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
- [Rust / rustup](https://www.rust-lang.org/tools/install) のstable/MSVCツールチェーン

Rustが未導入の場合は、公式の `rustup-init.exe` でユーザー単位に導入します。
ホストは `x86_64-pc-windows-msvc`、ツールチェーンはstableを選びます。
標準の保存先は `%USERPROFILE%\.rustup` と `%USERPROFILE%\.cargo`、
ユーザーPATHへの登録先は `%USERPROFILE%\.cargo\bin` です。
`RUSTUP_HOME` / `CARGO_HOME` の明示設定は不要です。
既存のRustがある場合は、`where.exe rustup` と `rustup show` で確認してから不足分だけ導入してください。

Build Toolsの追加で管理者権限を求められる場合は、開発者自身でインストールしてください。
PATH変更後はPowerShell、VS Code、Codexの親アプリケーションも完全に終了して開き直してください。
起動中のアプリから開いたターミナルには、変更前のPATHが引き継がれる場合があります。
新しいPowerShellで次を確認できます。

```powershell
node --version
npm --version
rustup show
rustc -Vv
cargo --version
where.exe rustc
where.exe cargo
```

このプロジェクトの `rust-toolchain.toml` はstable/MSVC、minimalプロファイルを選択します。
ユーザー全体の既定ツールチェーンとは別のプロジェクト設定です。
ツールチェーンが未導入の場合だけ、次で追加できます。

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal
```

このPCでは標準パスに配置したRust / Cargo 1.99.1、rustup 1.29.3と、既存のVisual Studio
Community 2026 Insiders（MSVC 14.51）/ Windows SDK 10.0.26100.0を使用します。
新規PowerShellで、開発者シェルを事前に起動せずMSVCのリンク、`cargo check`、
`npm run tauri build`（MSI / NSIS生成）に成功しています。
一時フォルダーのRustを有効化する操作は不要です。

## インストールと起動

```powershell
cd C:\Users\owner\Desktop\sm-dev\listing-pilot
npm install
npm run tauri dev
```

Tauri CLIがViteを起動し、Listing Pilotのデスクトップウィンドウを開きます。
商品管理から商品の登録・編集ができます。「Rustとの接続確認」を開いて
「接続を確認」を押すと、従来のRust Commandが返した文字列も確認できます。

フロントエンドだけを確認する場合：

```powershell
npm run dev
```

`http://localhost:1420` で画面を確認できます。ブラウザー単体ではRustに接続できないため、
接続確認にはデスクトップアプリの利用を案内します。Web用Adapterは未実装です。

## ビルドと検証

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

`npm run build` はReact側とVite設定の型検査、およびフロントエンドのビルドを行います。
Windows配布用パッケージの作成は `npm run tauri build` です。

## 現時点の実装範囲

- アプリ名・キャッチコピーの表示
- 接続確認ボタン、処理中・成功・エラーの表示
- `src/lib/api` を介したTauri Command呼び出し
- Rustの `greet` Commandからの文字列応答
- 日本語の商品一覧・登録・編集フォーム、必須項目・価格検証、保存中の二重送信防止
- UUIDによる商品識別、SKU一意制約、SQLiteマイグレーション、終了・再起動後の永続化

削除、CSV、画像処理、バッチ、AI API、Shopify対応、Webデモ、多通貨、検索・ページネーション、クラウド同期は未実装です。

## 商品データの保存

Tauri Path APIの `app_local_data_dir()` で取得したアプリ専用ディレクトリに
`listing-pilot.sqlite3` を保存します。Windowsでは通常
`%LOCALAPPDATA%\com.listingpilot.app\listing-pilot.sqlite3` です。
プロジェクトや実行ファイルの隣には保存しません。開発版も同じ識別子のDBを利用します。
SKUは前後空白を除去し、大文字小文字を区別します（`ABC` と `abc` は別SKU）。
価格は任意の0以上の整数円です。JavaScriptで精度を失わない上限を適用します。

アプリを完全終了してからDBを別の場所へコピーするとバックアップできます。
復旧時もアプリを終了し、現在のDBを退避してからバックアップを同じ保存先へ戻します。
初期化失敗時にDBを削除・再作成したり、メモリDBへ切り替えたりはしません。
自動バックアップ・自動修復は未実装です。詳細は [データモデル](docs/data-model.md) を参照してください。

責務分担は [docs/architecture.md](docs/architecture.md)、初期選定の理由は
[docs/decisions/0001-initial-stack.md](docs/decisions/0001-initial-stack.md) を参照してください。
実行したコマンドと結果は [docs/verification.md](docs/verification.md) に記録しています。

## 公式資料

- [Tauriのプロジェクト作成](https://v2.tauri.app/start/create-project/)
- [Windows向け前提条件](https://v2.tauri.app/start/prerequisites/#windows)
- [ReactからRustを呼び出す](https://v2.tauri.app/develop/calling-rust/)
- [Viteの動作環境](https://vite.dev/guide/)
- [TypeScript 7.0の正式リリース](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
