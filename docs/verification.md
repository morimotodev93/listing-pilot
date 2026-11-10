# 開発環境の検証結果

## Phase 3.1 画像の表示順（2026-11-10 日本時間）

開始時の作業ツリーはクリーン。Commit・Push・ブランチ切り替えは行っていない。
前後移動、先頭のメイン画像表示、全ID順序を送る `reorder_product_images` を追加した。
既存のImportPermit取得後、単一接続のIMMEDIATEトランザクションで検証・順序更新・保存結果取得・COMMITを行う。
スキーマ、既存マイグレーション、依存関係、CSP、Capabilities、識別子、保存先、画像ファイル操作の方式は維持した。

### 自動検証

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 成功、終了コード0。TypeScript検査とVite本番ビルド |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功、終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功、終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功、終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 成功、45件（既存39件＋追加6件）、失敗0件 |
| `git diff --check` | 成功、終了コード0 |

追加6テストは複数ケースを含み、4枚の変更、連番、先頭変更、再取得・接続再開後の維持、同一順序、重複（両配列）・不足・余分・不明・別商品ID、古い順序・追加取り込み後の古いスナップショット、商品不在、途中SQL失敗の全更新ロールバックを検証した。
画像メタデータ・原本と管理画像のバイト列・SHA-256・商品更新日時の保持、ガード解放、DB Mutex取得前の競合拒否、追加取り込みの末尾配置、空・1枚・21枚、sort_order同値時の日時・ID順序も確認した。
すべて専用一時DB・メディアを使い、ローカルtest-fixturesや本番LocalDataにはアクセスしていない。

初回Viteビルドはサンドボックスの `spawn EPERM` で失敗。通常権限で同じ `npm run build` を再実行して成功し、コードや依存関係を変更して回避していない。

### Windows実WebView2

`npm run tauri dev -- --config .verification/phase31/config.json` を実行した。
検証専用 `appDirectoriesOverride` を `.verification/phase31/data` に指定し、今回生成した8×8 PNG画像だけを利用した。
実WebView2のCDPからReact UI・実IPCを操作し、Windows UI Automationでネイティブダイアログから画像を選択した。

- 4枚の取り込みと画像表示、前後移動、先頭・末尾の境界ボタン無効化、メイン画像表示の切り替えを確認。
- 画像一覧再取得・ページ再読み込み後に順序が維持されることを確認。
- UI外の実IPCで順序を変更してから古いUIで移動を実行し、日本語のstale_data通知と最新一覧への復帰を確認。
- 並べ替え後の追加取り込みで5枚目が末尾に入り、先行4枚の順序が保持されることを確認。
- 開発アプリを終了・再起動し、5枚の順序と画像表示が維持されることを確認。
- 成功した各検証区間でアプリのJavaScript例外・CSP違反を検出しなかった。

証跡と検証スクリプトはGit対象外の `.verification/phase31/` に保持し、既存の検証成果物は変更していない。
初回の検証スクリプトは商品一覧読込待機不足とPowerShell文字コードで失敗し、ハーネスを修正して再実行した。
追加の二重送信計数用ハーネスは内部invoke関数の差し替えによる計数ができず、成功根拠には含めていない。実WebView2での保存中全ボタン無効化・二重送信件数の厳密な検証は未完了。同期refガードとRust側の競合拒否はコードレビュー・自動テストで確認した。
ワークスペース内のWebView2プロファイルに対しVite watcherが `EBUSY` を出したが、アプリ動作の確認は完了した。
検証後は起動した開発プロセスを停止した。

今回 `npm run tauri build`、配布版CSP下でのPhase 3.1操作、インストーラー導入、別PC、電源断は未検証。
今回追加した変更は配布設定や権限の変更を含まない。リリース前に配布版の操作も確認する。

## Phase 2 最終レビュー修正（2026-11-09 日本時間）

初回実装の記録は次節に保持した。今回もmain / HEAD `a17ed5f` のまま、Commit・Push・ブランチ切り替えは行っていない。
開始時に `git status --short` と `git diff --check` を確認し、既存の未コミット変更を維持した。

### 修正内容と範囲

今回変更したファイルは以下の6件のみ。

- `src-tauri/src/services/image_service.rs`：参照取得の事前確認と、OSロック取得後の再照会・隔離判定を分離。
- `src-tauri/src/infrastructure/media_storage.rs`：正式配置をWindowsの非置換移動へ変更。
- `src-tauri/src/image_tests.rs`：競合・再照会失敗・非置換配置・長いパスの回帰テストを4件追加。
- `docs/architecture.md`：正しい初期化順序と正式配置方式を反映。
- `docs/decisions/0004-product-image-management.md`：既存内容を保持して最終レビューの判断理由を追記。
- `docs/verification.md`：この最終レビュー記録を追加。

React UI、Commands、Repository、DB共有方式、依存関係、CSP、Capabilities、Asset Protocolの設定は変更していない。
0001/0002 SQLと `.gitignore` は開始時からバイト単位で保持した。

原因は、OSロックを取得する前のDB参照一覧で隔離を判断していたことである。
事前照会はDBの利用可否確認だけに使って破棄し、OSロック取得後に最新参照を再照会する。
二度目の照会に失敗した場合は隔離せずエラーを返す。空の参照集合へ置き換えない。
DB初期化・事前照会失敗時はメディアを作成しない。ロック取得後の失敗ではロックファイルや必要ディレクトリが作成され得るが、既存ファイルは動かさない。
ImportPermitとOSロックの保持期間、画像コピー・デコード・ハッシュ中にDB Mutexを保持しない方式は維持した。

正式配置では、事前存在確認と `fs::rename` の間に対象が作成される競合をなくした。
Windowsの `MoveFileExW` にflags=0を指定し、既存対象の置換と別ボリュームへのコピーをOS操作で拒否する。
存在確認のI/Oエラーを「未存在」と扱う分岐も削除した。失敗したstagingは既存のDrop処理で整理する。
長いパスはcanonicalizeのverbatim形式で保持し、Windows専用依存は追加していない。

指摘されたスライス作成だけの `clone()` は開始時点で `std::slice::from_ref()` に置き換え済みだったため、その変更を保持した。
今回追加のテストにも不要なcloneや警告抑制のallowを導入していない。
任意のSQLite UNIQUE分類・タスク結合エラー分類は、動作実績と最小変更を優先して変更しなかった。

### 回帰テスト

| 追加テスト | 検証内容 |
| --- | --- |
| `initialization_refreshes_references_after_previous_owner_releases_lock` | Bの事前参照取得→Aの画像登録→AのOSロック解放→Bのロック取得・再照会を、2接続と明示的な処理順序で再現。登録画像とDBを保持し、quarantineが空であることを確認 |
| `failed_locked_reference_refresh_preserves_staging_and_originals` | 事前照会成功後にテストDBの参照テーブルを利用不能にし、再照会失敗時のstaging・正式ファイル保持、隔離なし、OSロック解放を確認 |
| `placement_never_replaces_an_existing_file_or_directory_and_cleans_staging` | 既存正式ファイルと同名ディレクトリに配置を試み、OS操作が拒否すること、既存画像・元画像・DBを保持し、stagingを整理することを確認 |
| `windows_placement_supports_long_paths_and_unicode_filenames` | 260文字超の管理パスとUnicode・サロゲートペアを含む元ファイル名で正常取り込みとバイト一致を確認 |

実際の複数プロセスのスケジューリングを使う競合テストではなく、同一の初期化後半処理に対し、
プロセスA/B相当のSQLite 2接続とOSファイルロックの取得・解放順を明示した決定的な回帰テストである。sleepやタイミング依存の待機は使っていない。
全テストは専用一時ディレクトリとテストSQLiteを使用し、本番LocalDataは使用していない。

### 最終コマンド検証

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 成功。React・Vite設定の型検査と本番ビルド |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功。警告0件、allow追加なし |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 成功。既存35件＋追加4件、39 passed / 0 failed / 0 ignored |
| `git diff --check` | 成功。既存のCRLF変換予告のみで、空白エラーなし |
| `npm run tauri build`（通常設定、最後のRustソース変更後） | 成功。EXE・MSI・NSISを最新ソースから再生成 |

サンドボックス内のViteは前回同様 `spawn EPERM` で停止した。通常権限で同じbuildを再実行し、成功した。
PowerShellプロファイル・ユーザーPATHは変更していない。

通常設定の配布ビルドは終了コード0で完了した（release最適化は約2分28秒）。
2026-11-09 12:36 日本時間に再生成された以下の3点が、最後に変更したRustソースより新しいことを確認した。

| 配布物 | SHA-256 |
| --- | --- |
| `src-tauri/target/release/listing-pilot.exe` | `D0742B0E724A9F576FF96D1F9600547C7F3F559FC06D17A160DA12BE06744ED9` |
| `src-tauri/target/release/bundle/msi/Listing Pilot_0.1.0_x64_en-US.msi` | `89669D0575D02CFA96DF97DADABF50143077E2AC416BE9DD55B19A3DE43CCC99` |
| `src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.0_x64-setup.exe` | `A96ECD19B90AC7833A10A3398CF9200CFE3A893C20AA1954E47210784E5CF568` |

通常EXE内に標準identifier `com.listingpilot.app` が含まれ、初回・今回のテスト用絶対保存先が含まれないことも確認した。
今回の配布物を過去の検証用バイナリで置き換える操作は行っていない。
CLIの既存identifier末尾 `.app` に関するmacOS向け推奨警告は継続しているが、Windowsビルドは成功している。

### 関連するWindows実WebView2検証

今回専用の `.verification/final-review/tauri.config.json` に `appDirectoriesOverride` を指定し、新しいUUID付き一時フォルダーへDB・画像・WebViewデータを隔離した。
通常設定ファイルにはテスト用保存先を追加していない。

- 新規商品をフォームから登録し、詳細の空状態からRustネイティブダイアログでPNG・JPEG・不正ファイルを選択。
- 新しい配置方式で成功2枚・失敗1枚となり、実imgのnaturalWidth=8を2枚とも確認。
- 同じ画像の再選択は重複1枚、取り込み中の二重Commandは `media_busy`、キャンセルは正常結果。
- 正常終了後に再起動し、商品情報と画像メタデータが一致、画像2枚とも再表示。
- 正式画像と選択元のSHA-256が一致し、quarantineが空であることを確認。
- 通常表示・操作のCSP違反0件、JavaScript例外0件。

再起動試行で旧Viteの終了直前に新Viteが起動し、一度ポート1420の競合が発生した。
旧検証プロセスの終了と1420/9334の解放を確認して再実行し、Vite・Tauri起動と再表示検証が成功した。アプリコードの変更は不要だった。
証跡は `.verification/final-review/` の `images-result.json`、`images-desktop.png`、`images-restarted.png` と検証スクリプト。

CSP・Asset Protocolの設定は変えていないため、今回release版の画面・範囲外403の再検証は行っていない。
初回実装時の開発/release版検証と範囲外HTTP 403の記録は次節に保持した。
インストーラー経由の導入・別PC・実ディスク容量不足・電源断・実運用バックアップ復旧は引き続き手動確認対象である。

検証アプリ・Vite・CDPは終了し、1420/9334に検証サーバーが残っていないことを確認した。
最終 `git status --short` は開始時と同じパス集合で、今回の内容変更は上記6ファイルに限定した。意図しない変更・新規Git対象ファイルはない。
0001/0002 SQLおよび `.gitignore` のSHA-256は次節の初回記録と一致し、ユーザーの変更を保持している。

## Phase 2 初回実装：商品画像（2026-11-09 日本時間）

開始時HEADは `a17ed5f`（main）。開始時の未コミット変更は `.gitignore`、ユーザー作成の0002 SQL、ADR 0004。
`.gitignore` と0001/0002 SQLはバイト単位で保持し、ADR 0004は元の内容に実装確定事項を追記した。
Commit・Push・ブランチ切り替えは実施していない。

確認対象は、商品一覧 → 詳細 → Rustネイティブダイアログ → 検証・コピー → SQLite登録 → Asset Protocol画像表示 → 終了・再起動後の再表示。
React Best PracticesとVerificationスキルを適用し、UIから直接invokeせずAPI層を通す境界も確認した。

### 自動検証

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 成功。React・Vite設定の型検査とVite本番ビルド |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功。stable/MSVC |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 成功。既存15件＋追加20件、計35 passed / 0 failed |
| `npm run tauri dev -- --config <一時検証設定>` | 成功。実WebView2で以下の経路を確認。通常終了後の再起動も成功 |
| `npm run tauri build` | 成功。Windows x64のMSI・NSISインストーラーを生成（詳細は下記） |
| `npm run tauri build -- --config <一時検証設定> --no-bundle` | 成功。保存先だけを隔離したrelease版で本番CSPの実動作を確認 |
| `git diff --check` | 成功 |

Viteはサンドボックス内で子プロセス起動が `spawn EPERM` となったため、通常権限で再実行して成功した。
PowerShellプロファイル・ユーザーPATH・npm依存は変更していない。

追加Rustテスト（`src-tauri/src/image_tests.rs`）は以下を確認する。

1. Version 0/1/2のマイグレーションと既存商品の保持
2. 0002の途中失敗時にテーブル作成をロールバックし、バージョン1を保持
3. JPEG・PNG・WebPの内容判定、メタデータ、表示順、元ファイルと保存ファイルのバイト列・SHA-256一致
4. ファイル名に依存しない同一商品内の重複と別商品への登録許可
5. 画像のハッシュ・相対パス一意制約をDuplicateSkuと区別
6. 商品不在のService拒否と外部キーによる直接登録拒否
7. 非対応・偽装・破損・読み取り不能・ディレクトリと部分成功
8. 20MiB超過、21枚拒否、20枚受け付け
9. 4,000万画素境界と巨大PNGヘッダーのデコード前拒否
10. staging・正式配置の失敗時の元ファイル保持と一時ファイル整理
11. DB INSERT失敗後の補償と登録済み画像の保持
12. SQLite AFTER INSERT / RAISE(FAIL)でエラーと書き込みが共存する場合の参照確認・画像保持
13. 異常終了を模したstaging残存・正式配置後未登録の孤立画像の隔離と再実行の安全性
14. 二重openのOSロック、取り込み中の二重操作・隔離拒否
15. 欠落画像と改変相対パス・トラバーサルの拒否
16. SQLiteとファイルの再open後の永続性
17. キャンセルの正常な空結果
18. Windows directory junction拒否とリンク先の保持
19. DB初期化失敗時に画像ディレクトリを作成・整理しない
20. 画像参照クエリ失敗を空の参照集合と誤認しない

単体テストはOS一時ディレクトリと専用SQLiteのみを使い、終了時に破棄した。

### Windows実アプリ検証

公式の `app.appDirectoriesOverride` を一時設定 `.verification/phase2.config.json` に指定した。
DB・画像・WebViewデータはUUID付きの一時フォルダーに隔離し、本番の `%LOCALAPPDATA%/com.listingpilot.app` は使用しなかった。
検証用設定はGit対象外であり、通常の `tauri.conf.json` には保存先の変更を含めていない。
CDPは検証用WebView2のローカルポート9334を使用し、ネイティブダイアログはUI AutomationとWin32メッセージで操作した。
Tauri IPC、ネイティブダイアログ、SQLite、Asset Protocolはモックではない。

| 境界 | 結果・証跡 |
| --- | --- |
| 一覧 → 商品登録 → 詳細 | 検証商品をフォームから登録し、商品名クリックで詳細へ移動。ブランド・価格などの既存情報と空状態を確認 |
| 詳細 → Rustダイアログ | 取り込みボタンから実ダイアログを表示。処理中表示と画面移動の無効化を確認 |
| 複数選択 → 個別処理 | PNG・JPEG・不正PNGを選択し、成功2枚・失敗1枚を通知。成功画像を保持 |
| SQLite → API → img | 画像メタデータとAPI層のURL変換を確認。実imgのnaturalWidth=8を2枚とも確認 |
| 同一画像の再選択 | 成功0枚・重複スキップ1枚・失敗0枚。画像数は2枚を維持 |
| 二重Command | ダイアログ表示中の追加importを `media_busy` で拒否 |
| 選択キャンセル | 正常結果の日本語通知と既存画像2枚の保持 |
| 画像公開範囲 | 実在するstaging画像・quarantine画像・SQLite・ダイアログで選択した元PNGをAsset URLで指定し、全てHTTP 403で読み込み拒否。通常の登録画像だけ表示 |
| CSP・実行時例外 | 通常画像表示と操作中のCSP違反0件、JavaScript例外0件。意図した範囲外拒否のネットワークエラーは別扱い |
| 終了 → 再起動 | 正常終了・再起動後、商品情報と画像メタデータが完全一致し、2枚とも再表示 |
| 起動時整合性 | 範囲外拒否試験で残したstaging1ファイルを隔離。登録済み画像を保持 |
| release版・本番CSP | 保存先だけを同じ一時領域に変更したrelease EXEを起動。`http://tauri.localhost/` の実画面で画像2枚・商品詳細・Rustダイアログ・重複通知を確認。CSP違反・JS例外0件 |
| release版・欠落と復元 | 検証専用の登録画像を一時退避して再読込し、プレースホルダーと案内を確認。元のパスへ戻して2枚の再表示を確認 |
| release版・公開範囲 | ダイアログで選択した元PNGも含め、同じ範囲外4種類全てのHTTP 403をCDPのNetwork応答で確認 |

Git対象外のローカル証跡：`.verification/images-smoke.mjs`、`pick-images.ps1`、`images-product.json`、
`images-result.json`、`images-scope.json`、`images-desktop.png`、`images-restarted.png`、`images-production.png`、`images-missing.png`。
開発・releaseの検証アプリとViteは終了し、CDP接続も終了した。一時データと証跡はレビューのため残している。

### 既存SQLの保持

作業前後のSHA-256が一致した。

| ファイル | SHA-256 |
| --- | --- |
| 0001_create_products.sql | `49DCDDA90C3ECB1A9487AB05DD4500C2CB866FC8ECA818A13C4A77F39E8AA0FB` |
| 0002_create_product_images.sql | `FF7487E4898B5E651BF90D101413447FCC9ACB88E05B1FABB48CA4EAC3BF26B5` |

0002には必要な商品UUIDの外部キー、相対パスの一意制約、商品ごとのSHA-256一意制約、MIME・サイズ・寸法・表示順の制約があり、修正は不要だった。

### 作成・変更ファイル

- 新規Rust：`commands/image.rs`、`domain/image.rs`、`services/image_service.rs`、`infrastructure/db/image_repository.rs`、`infrastructure/media_storage.rs`、`image_tests.rs`。
- 変更Rust：`lib.rs`、`error.rs`、既存各mod、`commands/product.rs`、`infrastructure/db/migrations.rs`、`tests.rs`。
- 設定：`src-tauri/Cargo.toml`、`Cargo.lock`、`tauri.conf.json`。Capabilitiesとnpm依存は変更なし。
- 新規React/API：`ProductDetail.tsx`、`ProductImages.tsx`、`imageTypes.ts`、`src/lib/api/images.ts`、`request.ts`。
- 変更React/API：`ProductList.tsx`、`products.css`、`src/lib/api/products.ts`。ProductFormとアプリのブランド・アイコンは変更なし。
- 文書：`README.md`、`docs/architecture.md`、`data-model.md`、`verification.md`、既存ADR 0004。
- ユーザーの `.gitignore` と0002 SQLは開始時の変更状態を保持した。

### 未実施・Windows実機での残確認

- 生成したインストーラー経由の導入・アンインストール、別PCでのWebView2互換性。release EXE自体の本番CSP・画像表示・範囲外拒否は隔離設定で確認済み。
- 実写真を使ったJPEG・PNG・WebPの長時間利用、極端なビット深度・アニメーション画像の表示互換性。
- 実際のディスク容量不足、権限変更、ネットワークドライブ、OS強制終了・電源断。失敗・残存ファイルは単体テストで模擬した。
- 実運用で欠落が発生した場合のバックアップ復旧。プレースホルダーとファイル復元後の再表示はrelease版の実画面で確認済み。
- 実運用バックアップを別ディレクトリへ復元し、DBとmediaを揃えた復旧を確認すること。
- メディアOSロックは二重openの単体テストで確認。実アプリ二重起動後の案内と再起動は手動確認対象。

画像の削除・変換・リサイズ・並べ替え・ジョブ処理は今回の対象外。既知のテスト失敗はない。

### 配布ビルド記録

`npm run tauri build` が終了コード0で成功し、以下を生成した。インストーラーの実行・公開は行っていない。

- `src-tauri/target/release/listing-pilot.exe`
- `src-tauri/target/release/bundle/msi/Listing Pilot_0.1.0_x64_en-US.msi`（約3.40MiB）
- `src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.0_x64-setup.exe`（約2.44MiB）

CLIは既存identifier `com.listingpilot.app` の末尾 `.app` に対してmacOS向けの推奨警告を出した。
Windowsビルドは成功しており、ユーザー指定の既存identifierとブランドは維持した。

本番CSPの実行時確認には、保存先だけを公式一時設定で変更した追加の `--no-bundle` releaseビルドを使用した。
通常配布用EXEを退避し、検証用EXEを `.verification/listing-pilot-isolated-release.exe` にコピーした後、通常EXEをバイト単位で復元した。
復元した `src-tauri/target/release/listing-pilot.exe` のSHA-256は
`DEC5E6BC4BB406F5C575170621E7E8981BA968997211D239A53FBCFA527FB5D0`。
MSI・NSISは通常設定で生成したまま保持した。検証用の保存先設定は通常配布物には含めていない。

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
