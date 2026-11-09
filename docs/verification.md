# 開発環境の検証結果

## Phase 1：SQLite・商品管理（2026-11-09 日本時間）

確認対象は、商品フォーム → `src/lib/api/products.ts` → Tauri Command → Service → Repository → SQLite → 一覧反映 → 完全終了・再起動後の復元です。
既存の接続確認も引き続き確認しました。以下の初期構築記録は履歴として保持しています。

### 自動検証

| 確認 | 結果 |
| --- | --- |
| `npm run build` | 成功。React・Vite設定の型検査とVite 8.3.6本番ビルド |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功。stable/MSVC |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 成功。15 passed / 0 failed |
| `npm run tauri dev` | 成功。実Tauriデスクトップウィンドウを起動。終了後の再起動も成功 |

ルートにはCargo.tomlがないため、fmtも明示的に `--manifest-path` を付けて実行しています。
Viteはサンドボックス内で `spawn EPERM` になりましたが、同じコマンドを制限外で実行し成功しました。
ユーザーのPATH・PowerShellプロファイルや既存npm依存は変更していません。

Rustの15テストは次を確認します。

1. 商品作成・UUID v4・UTC日時・SKUのtrim
2. 商品取得・存在しないIDの拒否
3. 空一覧・登録済み商品の一覧取得
4. 商品更新・IDと作成日時の保持
5. 登録時と更新時のSKU重複拒否、大小文字を区別する規則
6. 空文字・空白・全角空白の必須項目拒否
7. 負の価格・安全整数上限超過の拒否と上限値の保存
8. 存在しない商品の更新拒否
9. マイグレーション再実行後のデータとバージョン保持
10. ファイルDBの終了・再接続後のデータ保持とforeign_keys設定
11. 任意項目の未入力とNULL保存
12. マイグレーション失敗時のデータ・バージョン保持
13. 新しいスキーマバージョンの拒否
14. 不正なDBファイルの初期化エラーと元ファイル保持
15. DBの必須項目・整数価格・非負価格制約

テストDBはメモリ、またはOS一時ディレクトリ配下のUUID付き専用フォルダーを使用し、
本番のLocal AppDataにはアクセスしません。一時ファイルはテスト終了時に削除します。

### 実デスクトップでの確認

実WebView2にCDPで接続し、Reactフォームへの入力とボタン操作で確認しました。
Tauri IPCやDBはモックではありません。API経由の読み取り結果も画面と照合しました。

| 境界 | 結果・証跡 |
| --- | --- |
| UI描画 | 初期DBの商品0件と商品登録フォームを確認 |
| UI → API | フォームから新規登録と編集に成功 |
| API → SQLite | Rust生成UUID、trim済みSKU、UTC日時、価格とメモを取得 |
| SQLite → 一覧 | 登録・編集後に商品一覧へ反映。編集時の既存メモ読込も確認 |
| エラー → UI | 負の価格に検証メッセージ、SKU重複に日本語エラー表示。入力を保持し再操作可能 |
| 既存の疎通 | `greet` の接続成功メッセージを実画面で確認 |
| 完全終了 → 再起動 | 対象EXEのCloseMainWindowとWaitForExit後に `npm run tauri dev` を再実行し、同一商品の全フィールド一致を確認 |
| 実行時エラー | 初回・再起動後ともJavaScript例外・コンソールエラー0件 |

検証用に新規作成した商品：

- SKU：`PHASE1-VERIFY-1794184464385`
- ID：`08515bc9-13f4-4373-a2bb-591fe8ffdf1d`
- 商品名：`永続化確認用商品（編集済み）`
- 価格：2,500円

検証開始時の商品は0件でした。新しい検証用商品1件は確認後も残しています。
保存ファイルの実在を確認：
`C:\Users\owner\AppData\Local\com.listingpilot.app\listing-pilot.sqlite3`。

Git対象外のローカル証跡：`.verification/products-smoke.mjs`、`products-result.json`、
`products-desktop.png`、`products-restarted.png`。
検証用のアプリ・Vite・デバッグ接続は確認後に停止しました。

### 制限・残課題

- 作業フォルダーに `.git` がなく、`git status --short` は「not a git repository」。変更差分をGitでは比較できません。
- 配布用インストーラーの再生成・別PC検証、OSの強制終了・電源断耐性は今回未検証です。
- 自動バックアップ・自動復旧は未実装。終了中のDBコピーによる手順を `data-model.md` に記載しました。
- 開発版と配布版が同じ識別子ならDB保存先を共有します。
- 次のPhaseでは画像を商品UUIDに関連付ける機能、CSV入出力、バックアップ運用を優先度に応じて検討します。

検証日：2026-11-06（日本時間）

## Rust環境の永続化

初期構築時の一時環境を調査した結果、通常のPATHにRustはなく、標準の
`C:\Users\owner\.cargo` / `.rustup` も存在しませんでした。
ユーザー・システムの `CARGO_HOME` / `RUSTUP_HOME` / `RUSTUP_TOOLCHAIN` は未設定でした。
一時環境にはstable/MSVCのRust / Cargo 1.99.1、rustup 1.29.3、minimal構成と
Cargoの依存キャッシュがあり、Rustの実行中プロセスはありませんでした。

### 実施した設定

- 一時環境のCargoホーム10,793ファイル、rustupホーム152ファイルを標準パスへコピーし、
  コピー元・先の全ファイルのSHA-256一致を確認。
- 保存済みチェックサムと一致する公式 `rustup-init.exe` を
  `--profile minimal --default-host x86_64-pc-windows-msvc --default-toolchain stable-x86_64-pc-windows-msvc --no-update-default-toolchain --no-modify-path`
  で実行。既存のRust 1.99.1を維持。コピーで通常ファイルになった3つのプロキシも、
  rustup本体と同一ハッシュであることを確認して標準のハードリンクに修復。
- Rustホーム：`C:\Users\owner\.rustup`。
  Cargoホーム：`C:\Users\owner\.cargo`。
- ユーザーPATH末尾に `C:\Users\owner\.cargo\bin` を1回だけ追加。
  それ以外のユーザーPATHとシステムPATHは変更なし。Windowsへ環境変更を通知。
- `CARGO_HOME` / `RUSTUP_HOME` は未設定のまま標準値を使用。
  一時フォルダーへの環境変数・リンカー設定は追加していない。
- ユーザー既定：`stable-x86_64-pc-windows-msvc`、minimal。
  プロジェクトの `rust-toolchain.toml` も同じ選択であり、変更なし。
- 一時環境は削除せず保持。Node.js / npm、PowerShellプロファイル、Visual Studioの設定は変更なし。

### 新規PowerShellとMSVC

保存されたシステムPATHとユーザーPATHから環境を組み立て、新しい `powershell.exe` を起動。
プロファイルあり・`-NoProfile` の両方でRustを認識しました。
`VsDevCmd.bat` は使用せず、`LIB` / `INCLUDE` / `WindowsSdkDir` なども未設定で検証しました。
Codexからこの新規プロセスを起動してビルドを実行しています。

| 確認 | 結果 |
| --- | --- |
| `rustup show` | 標準Rustホーム、stable/MSVC、対象プロジェクトのtoolchain設定を認識 |
| プロジェクト外の `rustup show active-toolchain` | `stable-x86_64-pc-windows-msvc (default)` |
| `rustup --version` | `1.29.3 (edbc831ce 2026-11-02)` |
| `rustc --version` / `rustc -vV` | `1.99.1 (58cf8f9c3 2026-10-13)`、host `x86_64-pc-windows-msvc` |
| `cargo --version` | `1.99.1 (5f94df478 2026-08-27)` |
| `where.exe rustc` | `C:\Users\owner\.cargo\bin\rustc.exe` のみ |
| `where.exe cargo` | `C:\Users\owner\.cargo\bin\cargo.exe` のみ |
| `rustup target list --installed` | `x86_64-pc-windows-msvc` |
| MSVCツール | 既存のVisual Studio Community 2026 Insiders、ツールフォルダー `14.51.36231`、`cl.exe` 19.51.36252 / `link.exe` 14.51.36252.0 |
| Windows SDK | 既存の10.0.26100.0。kernel32.lib、ucrt.lib、rc.exe、mt.exeの存在を確認 |
| Rustの単体リンク確認 | `rustc` で検証用EXEの生成・実行に成功。開発者シェルの事前設定なし |

プロファイルなしのNode.js / npmは24.16.0 / 11.13.0、通常プロファイルでは既存のfnmが
24.19.0 / 11.17.0を選択しました。今回Node.js / npmのインストール・更新は行っていません。
`npm run build` と `cargo check --manifest-path src-tauri/Cargo.toml` は、プロファイルなしの
新規PowerShellで終了コード0を確認しました。
新規PowerShellのログはGit対象外の `.verification/rust-new-powershell.log` と
`rust-profile.log`、変更前のPATHとコードのハッシュは `rust-environment-before.json` に保存しています。

PATHを既に読み込んでいるPowerShell、VS Code、Codexは、親アプリケーションごと完全に終了して
起動し直してください。アプリケーション自体の再起動やWindowsの再ログインは今回実行していません。
Visual Studio / Build Tools / SDKの追加インストールは行っていません。

### Listing Pilotのビルド結果

すべて、新しいPowerShellで永続PATHを読み込み、開発者シェルを使わず実行しました。

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 成功。型検査とVite本番ビルド、終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功。終了コード0 |
| `npm run tauri build` | 成功。MSVCでreleaseビルド（4分22秒）後、MSI / NSISを生成、終了コード0 |

生成物：

- `src-tauri/target/release/listing-pilot.exe`
- `src-tauri/target/release/bundle/msi/Listing Pilot_0.1.0_x64_en-US.msi`（1.97 MiB）
- `src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.0_x64-setup.exe`（1.35 MiB）

TauriがWiX / NSISと補助DLLを取得し、ハッシュ検証後にユーザーキャッシュへ展開しました。
Visual StudioやシステムPATHの変更はありません。生成したインストーラーの実行、署名、
配布先PCでの検証は行っていません。
既存の識別子 `com.listingpilot.app` の末尾 `.app` に関するmacOS向けの警告がありましたが、
Windowsビルドは成功しています。今回の目的に合わせ、識別子やアプリコードは変更していません。

実行記録は `.verification/rust-build.log`、生成物のSHA-256と最終設定確認は
`.verification/rust-final-audit.json` に保存しています。
アプリコード、依存定義、ロックファイル、`rust-toolchain.toml` の計17ファイルを作業前の
SHA-256と比較し、変更がないことを確認しました。
検証スクリプト・リンク確認用EXE・ログと元の一時環境は、削除せず保持しています。

公式資料：[rustupの標準インストール先](https://rust-lang.github.io/rustup/installation/index.html)、
[Rustホームの環境変数](https://rust-lang.github.io/rustup/environment-variables.html)。

## 初期構築時の記録

以下は永続化前の検証記録です。一時環境の有効化は今後の開発には不要です。

### 使用環境

- Windows x64 / Node.js 24.16.0 / npm 11.13.0
- Rust / Cargo 1.99.1 stable、host `x86_64-pc-windows-msvc`
- 既存のVisual Studio Community 2026 Insiders / MSVC 14.51.36231
- 既存のWindows SDK 10.0.26100.0 / WebView2 Runtime 156.0.4314.58

Rustが通常のPATHと既定のインストール先に存在しなかったため、公式rustup-initの
SHA-256を確認し、`%TEMP%\listing-pilot-rust` 内の `CARGO_HOME` / `RUSTUP_HOME` へ
`--no-modify-path --profile minimal --default-toolchain stable-x86_64-pc-windows-msvc` で導入しました。
検証プロセス内でのみPATHと既存の `VsDevCmd.bat -arch=x64 -host_arch=x64` の環境を適用しました。
管理者権限によるシステム依存関係の追加や永続PATHの変更は行っていません。

### 実行した主なコマンド

| コマンド | 結果 |
| --- | --- |
| `npm create tauri-app@4.7.5 listing-pilot -- --template react-ts --manager npm --identifier com.listingpilot.app --tauri-version 2 --yes` | 成功。既存ディレクトリがないことを確認して生成 |
| `npm install` | 成功。29パッケージ追加、監査30パッケージ、脆弱性0件 |
| `npm run build` | 最終状態で成功。React側とVite設定の型検査、およびVite本番ビルド |
| `cargo check`（`src-tauri` 内） | 成功。終了コード0 |
| `npm run tauri dev` | 成功。Vite起動、Rust実行ファイル生成、ネイティブウィンドウ起動 |
| `npm ls --depth=0` | 成功。主要npm依存の実バージョンを確認 |
| `node .verification/smoke.mjs` | 最終状態で成功。実WebView2への入力からRust応答の画面表示まで確認 |

### 実際の動作

「接続を確認」を押すと `src/lib/api` の `getGreeting()` がRustの `greet` Commandを呼び、
次の文字列をステータス領域に表示しました。Tauriのモックは使用していません。

```text
Listing Pilot の準備ができました。Rustとの接続に成功しました。
```

- アプリ名とキャッチコピーが一致することを確認。
- ネイティブプロセスの `MainWindowTitle` は `Listing Pilot`、ウィンドウハンドルは非0、応答あり。
- 実WebView2のCDPマウス入力でボタンを操作し、`data-state="success"` と返却文字列を確認。
- 完了後にボタンが再度有効になることを確認。
- 最終検証でJavaScript例外・コンソール/ネットワークエラーなし。
- 画面と結果JSONはGit対象外の `.verification/desktop-success.png`、`smoke-result.json` に保存。

### 発生した問題と対処

- 初回のPowerShell起動時に既存のfnmプロファイルが権限エラーを出したため、以降は
  プロファイルを読み込まず実行。既存のプロファイルは変更していません。
- npmメタデータ取得：サンドボックスのオフライン設定で `ENOTCACHED`、通信制限で
  `ECONNREFUSED`。実行プロセス内の設定とネットワーク実行権限で解消。
- 初回のビルド：`noEmit` とproject referencesの組み合わせで `TS6310`。
  型検査対象を独立した設定に整理し、両方をbuildスクリプトで検査することで解消。
- Vite実行：サンドボックスの子プロセス起動制限で `spawn EPERM`。
  プロジェクト内のビルドをサンドボックス外の通常権限で実行して成功。
- 初回の画面検証：Rust疎通は成功したが、favicon未指定により `/favicon.ico` が404。
  `public/favicon.svg` を指定し、再検証でエラーがないことを確認。

### 初期構築時の未実施事項

- `npm run tauri build` による配布用インストーラー作成、署名、配布環境での動作確認。
- ブラウザー単体のUI操作検証（Reactの起動と操作は実Tauri WebView2で検証）。
- 将来機能の検証（SQLite、画像、CSV、AIなどは未実装）。

検証用の開発プロセスとデバッグ接続は確認後に停止しました。
継続開発時のRust環境と起動手順は [README](../README.md) を参照してください。
