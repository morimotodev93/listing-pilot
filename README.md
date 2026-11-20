# Listing Pilot

**写真から、出品準備完了まで。**

写真を起点とした出品準備を支援するWindowsデスクトップアプリです。SQLiteによる商品管理と、商品画像の取り込み・プレビュー・表示順変更・1枚ずつの安全な削除、商品全体の安全な削除（Phase 3.3）に対応しています。
公式 `create-tauri-app` のTauri 2 / React / TypeScriptテンプレートから作成しています。

Phase 4.1では「カスタム項目設定」から共通の商品項目を作成し、名前変更・非表示・復元できます。
Text / Numberと、Numberの任意単位に対応します。Phase 4.2では商品登録・編集で値を一括保存し、詳細で表示できます。
非表示の値は保持し、復元後に再利用できます。商品削除時はその商品の値も削除します。
Textは2,000文字以内、Numberは整数15桁・小数6桁以内の文字列で、先頭ゼロや精度を保持します。
作成後の型・単位変更、定義の物理削除は未実装です。詳しくは [ADR 0007](docs/decisions/0007-product-attribute-values.md) を参照してください。
非表示でも定義は保持され、同名項目は新規作成できません。
Phase 4.3では「EC掲載用の文章」で公開用タイトル（任意・200文字）と公開用説明文（任意・10,000文字）を
登録・編集し、詳細表示できます。管理用の商品名・内部メモとは独立し、自動転記しません。
未設定は未設定のまま保持します。Phase 4.4では商品詳細の「出品プレビュー」から、
公開用文章・画像・設定済みの有効なカスタム値と準備状況を確認できます。
公開用タイトル未設定時は管理名を候補として表示し、警告します。候補は自動保存しません。
プレビューは読み取り専用の取得時点の情報です。変更後は再読み込みしてください。
Phase 4.5ではプレビューから商品1件の汎用CSVとオリジナル画像をエクスポートできます。
警告がある場合は内容への明示確認が必要で、出力不可の商品や確認後に変更された候補は出力しません。
選択した親フォルダーに `ListingPilot_{商品UUID}_{日付}/` を作り、product.csv・attributes.csv・images.csvとimages/を保存します。
CSVはUTF-8 BOM付きです。数式として扱われる文字列は対象列を表示して拒否し、値は自動修正しません。
同じ商品を同じ日に同じ親へ出力すると衝突エラーになります。別の親フォルダーを選んでください。
アプリ保存領域やリンク先には出力できません。途中で清掃に失敗した場合は表示された一時フォルダーを確認してください。
自動公開は未実装です。単品出力の契約は [ADR 0010](docs/decisions/0010-single-product-export.md) を参照してください。
Phase 4.6では一覧のチェックボックスで商品を選択し、一括エクスポートできます。
所属グループで絞り込み、「表示中をすべて選択」「表示中の選択を解除」「すべての選択を解除」を利用できます。
グループを切り替えても選択は保持され、一覧再読み込みでは削除済み商品の選択を除きます。
「選択したN件を一括エクスポート」で全対象の最新内容を確認し、警告があればチェックボックスで承認してください。
出力不可の商品が1件でもあれば全体を停止します。上限は20商品・画像200枚・登録済み画像合計1GiBです。
選択した親の `ListingPilot_Batch_{日付}_{バッチUUID}/{商品UUID}/` に、単品と同じ3つのCSVとimages/を出力します。
現在グループの有効で保存済みのカスタム値だけを使い、元商品・原本画像・旧グループの保存値は変更しません。
全件が成功した場合だけ完成フォルダーを1回確定し、途中失敗・確認後の変更では完成させません。
出力開始後のキャンセルには対応しません。清掃失敗・確定結果不明では表示された場所を確認してください。
開発監視中のリポジトリ内では最終移動がアクセス拒否になる環境があります。開発時はリポジトリ外の専用フォルダーを選んでください。
詳しくは [ADR 0012](docs/decisions/0012-batch-product-export.md) と [検証記録](docs/verification.md) を参照してください。
Phase 5.1では同じ一覧選択から「選択商品をShopify用CSVで出力」を利用できます（1〜20商品）。
Shopify専用の10列・UTF-8 BOMなし・LF形式で、新規商品の下書き・非公開用CSVを1つ生成します。
画像・カスタム属性・管理メモは含みません。画像未登録・原本欠落でもCSVだけを出力できます。
公開用タイトル未設定は管理名を候補とし、タイトル／説明文未設定の警告は明示確認が必要です。
価格未設定は出力不可、0円は有効です。Vendor未設定は空欄となり、Shopify側で既定値が付く場合があります。
対象ストアの通貨がJPYであることをShopify管理画面で確認し、確認欄をチェックしてください。
アプリはストア通貨を検出・変換しません。インポート時は一致Handleの上書きをオフにしてください。
既存Handleの衝突検出・既存商品の更新・Shopifyへの直接送信は行いません。
出力先は `ListingPilot_Shopify_{日付}_{出力UUID}/shopify-products.csv`、Handleは商品UUIDから安定生成します。
Shopify Dev Storeでの実インポートと再エクスポートはユーザーによる受入確認待ちです。
詳しくは [ADR 0013](docs/decisions/0013-shopify-csv-export.md) を参照してください。
Phase 5.2.2Bでは商品詳細の「商品画像」から「Shopify画像URLを設定」を開けます。
対象の`example-store.myshopify.com`を入力して読み込み、ローカル画像のサムネイル・画像UUIDを確認してURLと任意のaltを設定します。
altは最大512 Unicode文字・125文字以内を推奨。一括貼り付けは1行1URLで、空行・末尾改行も画像1枚分として数えます。
対応確認後の「入力欄へ適用」は未保存です。「設定を保存」で対象ストアの全画像設定をまとめて保存してください。
「URL設定を解除」はURLとaltを空にします。ローカル画像やShopify Filesの画像自体は削除しません。
未保存時の戻る・ストア切替・再読み込みには破棄確認があります。競合・保存結果不明では自動再送信せず、入力を保持して明示的な再取得を案内します。
保存時と現在の画像情報が異なる場合は、画像とURLの対応を再確認してください。外部URLの画像をアプリ内では取得・表示しません。
画像付き13列CSVはPhase 5.2.2Cで実装予定です。既存の画像なし10列CSVは従来どおり利用できます。
Phase 4.5.1ではRust側でカスタム項目グループに対応し、既存商品・項目を「既定」へ移行します。
商品には現在グループの項目だけを適用し、グループ変更前の値は保持します。
Phase 4.5.2では「カスタム項目設定」でグループの作成・改名・非表示／復元とグループ別項目管理ができます。
商品登録・編集で所属を選択し、一覧・詳細で所属名を表示します。未保存カスタム値がある切替では確認し、
破棄するのは未保存のカスタム入力だけです。既存商品の所属変更保存にも確認が必要です。
旧グループへ戻ると保存済みの値を再利用でき、プレビュー・CSVには現在グループの有効な設定済み値だけを使います。詳細は [ADR 0011](docs/decisions/0011-attribute-groups.md) を参照してください。
現在の開発版は起動時にDBをスキーマ6へ移行します。**起動前に全アプリを終了し、DBとmedia全体を一組でバックアップしてください。**
公開済みv0.1.1（スキーマ5）では移行後のDBを開けません。コードレビュー待ちのPhase 5.2.2A／Bは通常データで起動せず、専用の隔離データで検証してください。

画像カードの「削除」から対象画像を確認し、「削除する」で登録情報を削除できます。
残った画像は0始まりの表示順に整えられ、先頭がメイン画像になります。
管理画像は `media/quarantine/deleted/{商品UUID}/{画像UUID}.{ext}` に保持し、取り込み元は変更しません。
商品詳細の「商品を削除」から最新の商品名・SKU・画像枚数・カスタム値件数を確認し、商品と全画像・カスタム値の登録情報をまとめて削除できます。
確認後に商品・画像・カスタム値が変わった場合は中止し、内容の再確認を案内します。
アプリ内復元・完全消去は未実装です。隔離だけが失敗した場合は警告を表示し、次回起動時に未参照画像を再確認します。
DBと `media/` 全体を、アプリ終了中に一組でバックアップしてください。
対象ファイル・終了確認・復元前の退避は [バックアップと復元](docs/backup-and-restore.md) を参照してください。

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

Phase 3.4（2026-11-10）では、既存Rustテスト71件、TypeScript・Vite・Rustの品質チェックと
Windows Release / MSI / NSIS生成に成功しました。専用保存先に隔離した開発版の実IPC、
本番CSP下の専用Releaseで商品登録・編集・画像表示・並べ替え・削除・商品削除・再起動、
DBとメディアの整合性、バックアップから別領域への復元を確認しています。
通常設定の配布EXEそのものの操作、インストーラー導入・アンインストール・上書き更新、
別PC、電源断・実ディスク故障は未検証です。詳細と制約は [検証記録](docs/verification.md) を参照してください。

## 現時点の実装範囲

- アプリ名・キャッチコピーの表示
- グループ別カスタム商品項目の定義管理（Text / Number、任意単位、名前変更、非表示・復元、重複と競合の拒否）
- 接続確認ボタン、処理中・成功・エラーの表示
- `src/lib/api` を介したTauri Command呼び出し
- Rustの `greet` Commandからの文字列応答
- 日本語の商品一覧・登録・編集フォーム、必須項目・価格検証、保存中の二重送信防止
- UUIDによる商品識別、SKU一意制約、SQLiteマイグレーション、終了・再起動後の永続化
- 一覧の商品名から詳細表示、Rustのネイティブダイアログによる複数画像選択
- 元画像を変更しないコピー保存、画像メタデータ、同一商品内のSHA-256重複スキップ
- 商品画像のプレビュー、欠落時のプレースホルダー、個別の成功・失敗・キャンセル通知
- 起動時の未完了一時ファイル・孤立画像の隔離（完全削除は行わない）

画像リサイズ・形式変換、CSVインポート、AI API、Shopify直接連携、Webデモ、多通貨、検索・ページネーション、クラウド同期は未実装です。

画像はJPEG・PNG・WebPに対応し、1枚20MiB・4,000万画素、1回20枚まで選択できます。
実際のバイト列をデコードして検証します。デコーダーの割り当て上限は256MiBで、画像のビット深度などによって画素数の上限以内でも拒否される場合があります。
保存先は既存のメディアOSロックで保護し、取得できない起動プロセスでは商品・画像APIを利用できません。
別プロセスを閉じた後は、利用するアプリを再起動してください。
商品編集は画面を開いたときの更新日時と一致する場合だけ保存し、競合時は入力を保持して案内します。
Phase 3.5の監査判断・再現結果は [品質改善記録](docs/qa-hardening-phase35.md) を参照してください。

## 商品データの保存

商品詳細の画像カードから「前へ移動」「後ろへ移動」で表示順を保存できます。
先頭がメイン画像になり、再読み込み・再起動後も順序を維持します。保存中は画像操作と画面遷移を停止します。
保存失敗時はエラーを表示して最新一覧を再取得します。再取得にも失敗した場合は「画像を再読み込み」後に移動を再試行してください。
画像変換・ドラッグ＆ドロップは未実装です。

Tauri Path APIの `app_local_data_dir()` で取得したアプリ専用ディレクトリに
`listing-pilot.sqlite3` を保存します。Windowsでは通常
`%LOCALAPPDATA%\com.listingpilot.app\listing-pilot.sqlite3` です。
プロジェクトや実行ファイルの隣には保存しません。開発版も同じ識別子のDBを利用します。
SKUは前後空白を除去し、大文字小文字を区別します（`ABC` と `abc` は別SKU）。
価格は任意の0以上の整数円です。JavaScriptで精度を失わない上限を適用します。

画像本体は同じ保存先の `media/originals/{商品UUID}/{画像UUID}.{ext}` に保持し、DBには相対パスを保存します。
プレビューはTauri Asset Protocolで `media/originals/` だけを公開します。
アプリの全プロセスを完全終了してから、DBと `media/` をセットで別の場所へコピーするとバックアップできます。
復旧時も全プロセスを終了し、現在のDBと `media/` を退避してから同一時点のバックアップを同じ保存先へ戻します。
初期化失敗時にDBを削除・再作成したり、メモリDBへ切り替えたりはしません。
隔離されたファイルは `media/quarantine/` に保持します。確認・復旧は手動で行い、ファイルだけを戻してもDBの関連付けは復元されません。
自動バックアップ・欠落画像の自動修復は未実装です。
詳細は [バックアップと復元](docs/backup-and-restore.md)、[データモデル](docs/data-model.md) を参照してください。

責務分担は [docs/architecture.md](docs/architecture.md)、初期選定の理由は
[docs/decisions/0001-initial-stack.md](docs/decisions/0001-initial-stack.md) を参照してください。
実行したコマンドと結果は [docs/verification.md](docs/verification.md) に記録しています。

## 公式資料

- [Tauriのプロジェクト作成](https://v2.tauri.app/start/create-project/)
- [Windows向け前提条件](https://v2.tauri.app/start/prerequisites/#windows)
- [ReactからRustを呼び出す](https://v2.tauri.app/develop/calling-rust/)
- [Viteの動作環境](https://vite.dev/guide/)
- [TypeScript 7.0の正式リリース](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
