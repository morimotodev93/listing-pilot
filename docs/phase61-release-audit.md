# Phase 6.1 — v0.2.0リリース前監査・Schema v5→v6移行検証

実施日：2026-11-21（日本時間）。基準HEAD：`c271589`。開始時の作業ツリーはクリーン。
公開版はv0.1.1／Schema v5、開発ソースは表示バージョン0.1.1／Schema v6。
今回バージョン変更・配布ビルド・Commit・Push・タグ・Release公開は行っていない。

## 判定と安全境界

**合成データに対する移行・保護・隔離復元は合格。ユーザーのレビュー承認後、Phase 6.2のv0.2.0ビルド準備へ進行可能。配布可否は未確定。**
実ユーザーデータ・実インストーラー更新・Release WebView2の成功を意味しない。
後続の承認条件と配布ゲートは[リリースチェックリスト](release-checklist-v020.md)に分けた。

- 通常の `%LOCALAPPDATA%\com.listingpilot.app` は読取・書込・列挙・コピーしていない。
- 既存SQLite・media・バックアップ・既存`.verification/`成果物は流用・変更・削除していない。
- 新規データはすべて`.verification/phase61/<UUID>/`以下。失敗した初回試験の証跡も残した。
- Tauri GUI、標準設定の`npm run tauri dev`、通常Release EXE、インストーラーは起動していない。
  Rustテストから本番と同じ`startup_service::initialize`を明示的な専用パスで呼んだ。
  起動前にコンパイル時リポジトリルートから専用パスを構築し、全親要素のリンク／reparse pointを拒否する。
  TauriのPathResolverは使用しない。この試験に`app.appDirectoriesOverride`は不要。
  将来GUIを起動する場合は、起動前に同設定の解決先を検証する必要がある。
- Migration 0001〜0006、アプリ実装、依存・lockfile、identifier、CSP、Capabilitiesは変更していない。
  ソース変更は`#[cfg(test)]`のモジュール登録と追加テスト2件のみ。

## 調査対象とv5の根拠

`AGENTS.md`、`README.md`、`docs/architecture.md`、`docs/data-model.md`、`docs/backup-and-restore.md`、
`docs/verification.md`（Phase 5.2.2A／5.2.3／5.2.4／6.0／6.0Aとexport_write履歴）、
ADR 0002〜0005・0011・0014、Tauri設定・Capabilities、Cargo.toml／package.jsonと両lockfile、
DB接続／Migration、startup_service、media_storage、関連Rustテストを確認した。

公開v0.1.1の記録対象コミット`01f0a1b`とHEADのMigration 0001〜0005を`git diff`とGit blob IDで照合し、全5ファイルの一致を確認。
旧`migrations.rs`も`git show 01f0a1b:src-tauri/src/infrastructure/db/migrations.rs`で確認し、上限5・0001〜0005の適用経路を照合した。
合成DBはこの既存SQLを順番に適用し`user_version=5`でコミットしたもので、独自DDLではない。
0006はURL設定テーブル・主キー自動インデックス・検索インデックスの追加だけである。

現行コードの順序は、既存DBの読取専用事前確認 → media OSロック → 書込DB接続とMigration →
ロック下で画像参照を再取得 → reconcile → DBとmediaを公開。
MigrationはIMMEDIATE内でDDL・user_version更新・foreign_key_checkを行い、失敗はロールバックする。
初期化失敗時に既存DBを消去・再作成したり、メモリDBへ切り替えたりする経路はない。
reconcileは未参照originals／stagingを非置換隔離し、`quarantine/deleted/`を再処理しない。

## 新規データセットと結果

最終成功試験のルート（リポジトリ相対、Git除外）：

| 用途 | `.verification/phase61/`直下のUUID |
| --- | --- |
| コマンドログ・Migration由来照合 | `27068d91-44fd-41d6-9acb-afe4d79267de` |
| v5生成・移行・バックアップ・復元・再起動 | `aab4f629-4a7e-4207-a965-4d8950a84084` |
| 失敗時保護5ケース | `4c4a14f1-9015-4fc7-aeaa-b24cad0a804e` |

成功データセットは商品2件（SKU、管理名、ブランド、0円／1,280円、内部メモ、公開タイトル、改行・タブ付き公開説明文）、
画像1枚の商品と4枚の商品、既定＋カスタム＋非表示の3グループ、Text／Number／非表示Textの3定義、6保存値を持つ。
Numberは文字列`0012.340000`。非表示グループの旧保存値も含め、現在所属以外の値を保持する。
画像は新規生成3×2 RGB PNGを6枚（各88 bytes）。5枚をUUID原本として登録し、1枚を削除済み隔離画像として配置した。
4枚の画像は挿入順とは異なる`sort_order`を持ち、`ORDER BY product_id,sort_order,created_at,id`も別途保存して照合した。
DBとmediaはテスト専有の同じ準備期間中に作成し、接続・ロックの解放後にバックアップした。

| 確認項目 | 今回観測した結果 |
| --- | --- |
| 起動前バージョン | 専用DBを読取専用で開き5。復元直後のコピーも5 |
| 起動サービスでのMigration | 通常データセットと復元コピーの両方で6 |
| SQLite整合性 | 生成後・復元直後・移行後・再起動後にintegrity_check=ok、foreign_key_check=0行 |
| 商品・画像・グループ・属性・保存値 | 旧5テーブルの全列・全行・SQLite値型を比較して一致（2／5／3／3／6件） |
| SKU・管理名・価格・公開文章・非表示状態 | 全列比較に含め、一致。先頭ゼロ・小数精度・改行・タブも保持 |
| 画像UUIDと表示順 | 全行と正規表示順の比較で一致。先頭画像も保持 |
| 旧スキーマ | sqlite_schemaの旧全行を保持、新規3行のみ追加 |
| image_url_settings | 存在し0行。旧画像へURL設定を自動生成しない |
| 原本・隔離画像 | media全体の相対パス集合・件数・サイズ・SHA-256が移行前後・再起動後とも一致 |
| 生成元画像 | 全6ファイルの相対パス・サイズ・SHA-256が一致。準備時に管理ファイルとのバイト一致も検査 |
| 再適用・再起動 | 両データセットでmigrations::apply再適用後とstartup_service再実行後の旧データ・スキーマが一致 |

保存証跡：`before-v5.json`、`schema-v5.json`、`sources-manifest.json`、`before-files.json`、
`dataset.after-v6.json`、`dataset.files-v6.json`、`restored.after-v6.json`、`restored.files-v6.json`、`result.json`。
全列スナップショットの文字列はRustのSQLite ValueのDebug表現（Text／Integer／Null）で、値型も比較する。
各UUIDルートは再実行ごとに新規作成し、以前の証跡へ上書きしない。

生成元と管理ファイルのSHA-256（0〜4は登録原本、5はdeleted隔離画像。全件88 bytes）：

| 生成元 | SHA-256 |
| --- | --- |
| synthetic-0.png | `f1316a38a44785e33b3dbd77d739a75450e609af81a9e2b38d3e2f9f1712f322` |
| synthetic-1.png | `508d4069b857ba985339cc71afe84b3fbcb9f33cad86ddd4b07f07110c22965d` |
| synthetic-2.png | `2d7203f80ac37ad0d7bbcad24592d95d899ebc63820647d47aa922271d356f7b` |
| synthetic-3.png | `ab0ae48fbab9551147184dac3175d16285c4bd8e6bfeb1e31158159f229e8728` |
| synthetic-4.png | `afad9b68f8c8f7ac9cac7fa8c07e2fd000972746c0af1235511cb25bd8b1ce38` |
| synthetic-5.png | `a9498d474106278b58814cea88040ebb5e40bdd508fdda351549dd08ebaed5f3` |

UUID相対パスとハッシュの対応は`before-files.json`に記録した。移行後のDBファイル自体のハッシュは変わるため、成功移行ではDBバイト一致を要求していない。

## 失敗時の保護

追加テスト`phase61_tests::v5_startup_failures_keep_database_and_every_media_file`は、各ケースに別の新規v5一式を生成する。
故障注入後・起動試行前を基準にして、戻り値・DBを含む全ファイルの相対パス・サイズ・SHA-256を照合した。
SQLiteを読める場合は旧スキーマ・全旧行・user_versionも照合した。

| 故障注入 | 結果と根拠 |
| --- | --- |
| 0006後半のインデックス名衝突 | DatabaseInitialization。テーブル追加もロールバック、v5と全DB／mediaバイト保持 |
| 既存画像の不正外部キー | DatabaseInitialization。foreign_key_check失敗で0006全体をロールバック、注入済みの不整合を含め元のv5状態を保持 |
| user_version=7 | DatabaseInitialization。7を維持、全DB／mediaバイト保持 |
| 別MediaStorageによるOSロック保持 | MediaBusy。v5のまま、Migrationせず全DB／mediaバイト保持 |
| 合成DBのヘッダー破損 | 読取専用事前確認でDatabaseInitialization。破損済みファイルをそのまま保持、再作成なし、media不変。破損前の記録はv5であり、破損後にversionを読めたとは扱わない |

証跡は各ケースの`before-files.json`と`result.json`。これは障害を注入したファイルの保持試験であり、破損DBの修復試験ではない。

既存テストも今回の全件実行で成功：

| テスト名（モジュール接頭辞は省略） | 保護の根拠 |
| --- | --- |
| migration_v5_to_v6_preserves_all_old_columns_data_and_schema | 全列・旧スキーマ・再適用の独立したメモリDB試験 |
| migration_late_ddl_failure_restores_v5_schema_and_data | DDL途中失敗のロールバック |
| migration_foreign_key_failure_rolls_back_v6_addition | 外部キー検査失敗のロールバック |
| migration_failure_rolls_back_schema_and_version | 初期Migration失敗時の既存内容保持 |
| newer_schema_is_rejected_without_modification | 未来スキーマ拒否 |
| initialization_failure_is_reported | 接続初期化失敗と既存DBバイト保持 |
| startup_failure_preserves_database_and_does_not_publish_writable_connection | 破損DBでmediaを作らず停止 |
| dataset_lock_survives_two_processes_and_abnormal_exit | 専用子テストプロセスとOSロック、拒否中のDB不変、ロック解放後の参照保持 |
| initialization_refreshes_references_after_previous_owner_releases_lock | ロック取得後に最新参照を読み直す |
| failed_locked_reference_refresh_preserves_staging_and_originals | 参照不明時の原本・staging保持 |
| failed_database_initialization_never_touches_media | DB初期化失敗時にmediaを整理しない |
| placement_never_replaces_an_existing_file_or_directory_and_cleans_staging | 非置換配置 |
| delete_quarantine_collision_never_overwrites_and_restart_recovers_pending | 隔離先衝突時の非置換と再起動回収 |

メモリDBテストは`image_url_tests`／`tests`、画像・起動テストは`image_tests`にある。
全件テストに既存の専用子プロセス終了試験を含むが、OS強制終了・電源断・実ディスク故障は再現していない。

## バックアップ／隔離復元

成功試験の停止済みv5セットは8ファイル・102,928 bytes：DB 102,400 bytes、原本5枚＋隔離1枚で528 bytes、空のmedia/.lock。
`.staging/`等の空ディレクトリもコピーした。SQLiteの-wal／-shm／-journalはこの正常終了セットには存在しなかった。

1. `dataset/` → 新規`backup-v5/`へDBとmedia全体をコピーし、全8件の相対パス・サイズ・SHA-256を照合した。
2. バックアップを一切開かず、新規`restored/`へ同じ一式をコピーし、同じmanifestと照合した。
3. 復元コピーを読取専用で開いてv5・旧全行・整合性を検査した。
4. 復元コピーを現行startup_serviceでv6へ移行し、旧全行・画像ハッシュ・再適用・再起動を再検査した。
5. 元バックアップ8件のmanifestが試験終了時も一致することを検査した。

**ファイルコピー・SQLite整合性・Rust起動サービスによる復元は検証済み。GUIの画像表示とネイティブ終了／再起動は今回未検証。**
関連ファイルが残った異常終了セットの復元／WALリカバリーも今回未検証。
現行版向けの停止・バックアップ・照合・旧版へ戻す手順を[バックアップと復元](backup-and-restore.md)へ追記した。
Schema v6からv0.1.1へのダウングレードは保証せず、user_versionだけを5に戻してはならない。
戻す場合は移行前のDB・関連ファイル・media全体の一組を復元し、移行後の一組も別途保全する。

## リリース準備監査表

| 項目 | 現状・判定 | 次工程の確認 |
| --- | --- | --- |
| バージョン | package.json／package-lock.jsonのルートとpackages[空文字]／Cargo.toml／Cargo.lockの自crate／tauri.conf.jsonはすべて0.1.1。今回維持 | 承認後の別工程で0.2.0へ揃える。依存版・identifierは変更しない |
| lockfile整合性 | npm ls --depth=0、cargo metadata --locked --offline --no-depsが終了0。両lockfileに差分なし | version更新後も自アプリ版以外の差分がないか確認 |
| identifier | com.listingpilot.appを維持 | 通常版では同じ保存先を使うため、検証環境側を隔離 |
| 本番CSP | script／styleはself、unsafe-inline／unsafe-evalなし。外部CDN接続許可なし、開発CSPは別設定 | 最新UIをRelease WebView2で確認。生成CSP・実際の違反も検査 |
| Asset Protocol・権限 | `$APPLOCALDATA/media/originals/**/*`のみ。staging／quarantine／DBは範囲外、core:defaultのみ | Releaseで画像表示と範囲外拒否を再確認 |
| WebView2 | Windows配布版にもRuntimeが必要。明示的webviewInstallMode設定なし。固定済みCLI 2.12.4のローカルconfig.schema.jsonでは既定downloadBootstrapper（silent=true）、minimumWebview2Versionは未指定 | Runtimeなし／オフライン環境、実インストーラーの挙動は未検証 |
| NSIS／MSI | `npm run tauri build -- --bundles nsis`／`--bundles msi`。今回未実行 | Phase 6.2以降、版・x64・サイズ・SHA-256・署名状態・生成時刻を記録 |
| 署名 | 署名設定なし。過去v0.1.1 NSIS／EXEはNotSigned | 未署名なら発行元確認やSmartScreen等の警告・組織ポリシーによる実行制限のリスク。現行候補の署名状態と実際の表示は別途確認。今回署名基盤追加なし |
| v0.1.1実績 | Phase 6.0AでNSIS生成（191テスト）、Phase 6.0でユーザー報告によるWindows 11実機1台の導入・終了・削除・再導入とデータ保持成功 | v0.2.0の更新成功として流用しない。MSI実導入や旧→新の更新実績でもない |
| v0.2.0更新 | 合成v5からv6へのサービス移行成功。インストーラー更新は未検証 | 専用ユーザー／VM等でv0.1.1→v0.2.0更新、画像・値・隔離保持、停止バックアップと復元を実施 |
| export_write エラー5 | **未解決**。下記の範囲を維持 | Releaseのリポジトリ内外の専用出力先、単品・一括・Shopify両方式の実出力を切り分け |
| 最新Stickyバー | Phase 5.2.4は専用開発版WebView2で800×600／400×600成功。Release／本番CSPは未検証 | 6画面、スクロール先頭／中間／末尾、キーボード、エラー／競合、モーダル、CSV完成出力、DPI・低い高さを確認 |

WebView2既定値の根拠は今回インストール済み`node_modules/@tauri-apps/cli/config.schema.json`であり、未実行インストーラーの実測ではない。
署名・Updater・CI・新しい配布基盤は導入していない。

### export_write エラー5の未解決範囲

過去Phase 4.5.1／4.5.2／4.6の記録では、監視中のリポジトリ配下で最終非置換移動が
`Export stage=final_move kind=PermissionDenied os=Some(5)`。staging生成・CSV書込／syncは完了し、清掃後にOS専用一時フォルダーで成功した。
単品の画像0枚と、一括の画像2枚コピーを含む出力で観測されている。
監視プログラム等のハンドル競合は仮説で、直接原因・具体的なロック元は未確定。
Phase 6.0のv0.1.1 CSV成功報告は出力先・方式の詳細がなく、解決の根拠にならない。
最新10列／13列とRelease環境での再現範囲は未確認。今回は実出力を行わず、保護・移動フラグ・権限を緩和していない。
今回のVite spawn EPERMやテスト補助処理の.lock読取エラー33は、別の観測として記録する。

## 自動検証結果（今回実行）

Windows x64、Node.js 24.16.0、npm 11.13.0、Rust／Cargo 1.99.1 stable／MSVC。
初回PowerShellプロファイルがfnm multishell作成のアクセス拒否を出したため、以後はlogin=falseで実行。永続環境は変更していない。
最終ソース編集後の結果：

| コマンド | 終了コード・結果 |
| --- | --- |
| npm run build | 0。TypeScript両設定＋Vite本番ビルド、45 modules。sandboxの初回は1（spawn EPERM）、コード・依存を変えず承認された通常権限で再実行して成功 |
| cargo fmt --manifest-path src-tauri/Cargo.toml --check | 0 |
| cargo check --manifest-path src-tauri/Cargo.toml | 0 |
| cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings | 0。警告0 |
| cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture | 0。**250 passed / 0 failed / 0 ignored**。既存248＋新規2、main／doc-tests各0 |
| cargo metadata --manifest-path src-tauri/Cargo.toml --locked --offline --format-version 1 --no-deps | 0 |
| npm ls --depth=0 | 0 |
| git diff --check | 0。最終文書・レビュー成果物作成後に確認 |

新規2テストの初回絞込実行は終了101、1 passed／1 failed。
Windows OSロック保持中にテスト用manifestが.lockの全バイトを読もうとしてエラー33になったため、
拒否確認後にテスト側の所有ロックを解放して全ファイル照合するよう修正した。アプリ実装は変更していない。
最終全件実行で両テスト成功。初回ルート`5719fdc6-8411-447b-95b8-06e190cd16dd`／`94e4f659-7564-4cd8-9bdc-1f818e45b3e6`とログを保全している。
`npm run tauri build`、GUI・実WebView2・実インストールは今回は未実行。過去の成功と区別する。

## 問題の重要度と残課題

| 重要度 | 項目 | 判定 |
| --- | --- | --- |
| 重大な移行不具合 | 今回の合成試験範囲 | 検出なし。実データ全パターンや実障害の保証ではない |
| 高（配布判定ゲート） | v0.2.0実更新、最新Release WebView2／CSP／Stickyバー | 未検証。ビルド準備を妨げないが、配布可と判定しない |
| 中（既知・未解決） | export_writeのWindowsエラー5 | 原因とReleaseへの影響範囲が未確定。解決か配布上の扱いを次工程で判断 |
| 中（運用上の制約） | 未署名、WebView2未導入／オフライン、別PC／MSI | 現行配布候補での実測が必要 |
| 制限 | WAL等が残った復元、GUI画像表示、電源断・実故障、大規模データ | 今回未検証。破壊的な試験や実データ利用はしていない |

実データのコピーを使う追加試験にはユーザーの別途明示承認が必要。
レビュー成果物は`.review/phase61/phase61.patch`と`changed-files.txt`。DB・media・ログ・build成果物は含めない。
