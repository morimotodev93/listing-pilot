# Phase 6.2 — v0.2.0 Release候補ビルド

実施日：2026-11-21（日本時間）。基準HEAD：`cc58c3bafa63368d711c651e2726bbd62301ab92`、ブランチ`main`。
開始時の`git status --short`は空。Phase 6.1レビュー承認・Commit・Push済みというユーザー指示を受けて実施した。
対象はWindows x64、アプリv0.2.0／Schema v6。公開済みv0.1.1／Schema v5とは区別する。

## 作業範囲と安全境界

今回の範囲は新規候補の生成・静的検査まで。通常の`%LOCALAPPDATA%\com.listingpilot.app`を読取・書込・列挙・移動・削除していない。
標準設定のTauri dev、通常Release EXE、NSIS／MSIインストーラーは起動していない。実インストール・一般公開は行っていない。
既存SQLite・media・バックアップ・既存`.verification/`成果物は流用・変更・削除していない。
Commit・Push・ブランチ切替・タグ作成・GitHub Release公開は行わず、パッチレビュー待ちとする。

証跡ルート（以下`E`）：

```text
C:\Users\owner\Desktop\sm-dev\listing-pilot\.verification\phase62\70f0c0e5-6327-4108-83c3-6d499f8159ec
```

新規`E/target`を`CARGO_TARGET_DIR`、`E/temp`をプロセス限定の`TEMP`／`TMP`に設定した。
既存`src-tauri/target`の配布成果物はビルド出力先に使わない。
Rustテストは専用一時DB／mediaを使用する。既存Phase 6.1テストだけはコードに固定された`.verification/phase61/<新規UUID>/`へ合成データを作る。
今回のログに出た新規UUIDだけを`E/test-evidence-verified/`へコピーし、元の新規出力も保持する。過去UUIDは開かない。
初回のPowerShell Copy-Itemは長いコピー先で失敗し、部分コピーは`E/test-evidence/`に保全した。
NodeのファイルAPIで別の新規コピー先へ再実行し、4ルート計248ファイルのパス集合・全SHA-256一致を確認した。
元UUID・コピー先・実行ログの対応は`E/test-evidence-copies.json`に記録している。

## バージョンと生成元

`package.json`、`package-lock.json`（ルートと`packages[""]`）、`src-tauri/Cargo.toml`、
`src-tauri/Cargo.lock`（listing-pilot自身）、`src-tauri/tauri.conf.json`の6か所を0.1.1から0.2.0へ置換した。
ビルド対象差分は5ファイル・6行のversion置換のみ。依存・機能コード・Migration 0001〜0006は変更しない。
`E/build-source.patch`と`E/build-source-manifest.json`に未コミットの生成元と追跡ファイルのSHA-256を保存した。
ビルド対象パッチのSHA-256：`CB4A0D20398F01963EDD59AB2F6FEB64C53C6784110FF8A47077A9AF2FF919CB`。
197追跡ファイルを照合し、検証記録文書以外のビルド後のバイト差分がないことも確認した。
ビルド後も対象ソースのバイト一致を照合し、記録文書の追記と区別する。

開始時にpackage.jsonとlockfileの版・dependencies・devDependencies・enginesの一致を確認した。
Windows PowerShell 5の`ConvertFrom-Json`は空文字キーで失敗したためNodeのJSONパーサーで確認した。
Nodeのインライン引数はWindowsの引用符処理で失敗し、専用`.mjs`ファイルへ移して成功した。
静的照合スクリプトからのGit子プロセスもsandbox内ではEPERMとなり、同じ照合を通常権限で再実行して成功した。
これらは検査補助処理の環境上の失敗であり、アプリや依存の修正は行っていない。

## 実行環境と自動検証

Node.js 24.16.0、npm 11.13.0、Rust／Cargo 1.99.1 stable、host `x86_64-pc-windows-msvc`。
Visual Studio Insiders 18.9.12023.133、MSVCコンパイラ19.51.36252.0／リンカー14.51.36252.0。
既存TauriキャッシュにNSIS 3.13、WiX 3.14.1.8722を確認した。
初回PowerShellプロファイルのfnm multishell作成がアクセス拒否となり、以後は`login=false`で実行した。
永続環境設定・プロファイル・OS機能・依存構成は変更していない。

各実行のコマンド・開始終了時刻・終了コードは`E/*.result.json`、出力は対応する`*.log`に保存する。
Rustは`CARGO_NET_OFFLINE=true`、check／clippy／testには`--locked`を付ける。
次の表はすべて最終アプリソースに対する結果であり、以後の編集は検証記録文書だけとする。

| コマンド | 終了コード・結果 |
| --- | --- |
| `npm ci --dry-run --offline --ignore-scripts --no-audit --no-fund` | 0。ただしdry-runはtarballの全キャッシュ存在を保証しなかった |
| `npm ci`（offline、audit/fund無効） | 1。Vite 8.3.6のtarballがENOTCACHED。通常npmキャッシュのログ書込拒否も記録 |
| `npm ci`（再実行、専用npmキャッシュ、audit/fund無効） | 0。承認されたネットワーク／通常権限で29 packages追加。固定lockfileの依存を取得。npm更新通知のみでnpm自体は更新しない |
| `npm run build` | 初回1、Vite設定読込でspawn EPERM。同じソース・依存を通常権限で再実行し0、TypeScript両設定とVite 45 modules成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 0 |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | 0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 0。警告0 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked -- --nocapture` | 初回101：249 passed／1 failed／0 ignored。再実行0：**250 passed／0 failed／0 ignored**、main／doc-tests各0件 |
| `cargo metadata --manifest-path src-tauri/Cargo.toml --locked --offline --format-version 1 --no-deps` | 0。自crate 0.2.0、専用targetを確認 |
| `git diff --check` | 0。最終文書・レビュー成果物作成後に確認。改行変換のGit通知は空白エラーと区別 |

初回テストの失敗は`product_export_tests::batch::nested_junction_is_never_written_through_or_cleaned_recursively`の
`cmd /d /c mklink /J`準備処理（status.successのassert）であり、保護処理のassertへ到達していない。
長い専用TEMPによるパス長制限を疑い、Windows `GetShortPathNameW`で同じ新規`E/temp`を指す
`C:\Users\owner\Desktop\sm-dev\LISTIN~1\VERIFI~1\phase62\70F0C0~1\temp`へプロセス内TEMP／TMPだけを変更した。
同じsandbox・同じ最終ソースで全250件成功したため、テストやアプリのコード・保護条件は変更しなかった。
mklinkの内部stderrは既存テストが出力しないので、厳密なOSエラー原因は断定しない。初回ログも保持する。

## 候補生成・成果物

**NSIS・MSIとも新規生成成功。Phase 6.2はレビュー可能。配布可否・実動作の判定はPhase 6.3に残す。**

| コマンド | 終了コード | 開始〜終了（2026-11-21 JST） |
| --- | --- | --- |
| `npm run tauri build -- --bundles nsis` | 0 | 12:31:46.2581199〜12:37:58.7358981 |
| `npm run tauri build -- --bundles msi` | 0 | 12:38:36.6693135〜12:41:36.0022963 |

両コマンドともbeforeBuildCommandのTypeScript／Viteも成功。通常権限で実行したがアプリは起動していない。
専用targetの初回Releaseコンパイルは6分00秒、MSI時の再コンパイルは2分44秒。
両ログにidentifier末尾`.app`のmacOS向け警告あり。Windows生成は成功し、指定どおりidentifierは変更していない。
Rustはoffline。NSIS／WiXは既存キャッシュを利用し、両ビルドログに追加ダウンロードの報告はない。
依存取得のネットワーク実行は前記の`npm ci`再試行として記録した。

実際の絶対パス（順にNSIS、MSI、MSIビルド終了後のRelease EXE、NSISビルド終了後EXEの保存コピー）：

```text
C:\Users\owner\Desktop\sm-dev\listing-pilot\.verification\phase62\70f0c0e5-6327-4108-83c3-6d499f8159ec\target\release\bundle\nsis\Listing Pilot_0.2.0_x64-setup.exe
C:\Users\owner\Desktop\sm-dev\listing-pilot\.verification\phase62\70f0c0e5-6327-4108-83c3-6d499f8159ec\target\release\bundle\msi\Listing Pilot_0.2.0_x64_en-US.msi
C:\Users\owner\Desktop\sm-dev\listing-pilot\.verification\phase62\70f0c0e5-6327-4108-83c3-6d499f8159ec\target\release\listing-pilot.exe
C:\Users\owner\Desktop\sm-dev\listing-pilot\.verification\phase62\70f0c0e5-6327-4108-83c3-6d499f8159ec\nsis-release\listing-pilot.exe
```

| 成果物 | サイズ（bytes） | 版 | 対象・PE／MSI内部情報 | Authenticode |
| --- | ---: | --- | --- | --- |
| NSIS `Listing Pilot_0.2.0_x64-setup.exe` | 2,791,795 | FileVersion／ProductVersionとも0.2.0 | 対象x64。NSIS起動スタブ自体のPE Machineは0x014C（x86）、生成設定ARCH=x64、アプリ本体は0x8664 | NotSigned |
| MSI `Listing Pilot_0.2.0_x64_en-US.msi` | 3,915,776 | 内部ProductVersion=0.2.0 | SummaryTemplate=x64;0、ProductName=Listing Pilot、ProductLanguage=1033、ALLUSERS=1 | NotSigned |
| Release EXE（MSIビルド終了後） | 7,895,040 | FileVersion／ProductVersionとも0.2.0 | PE Machine=0x8664（AMD64） | NotSigned |
| Release EXE（NSISビルド終了後の保存コピー） | 7,895,040 | FileVersion／ProductVersionとも0.2.0 | PE Machine=0x8664（AMD64） | NotSigned |

全件とも署名者・タイムスタンプ証明書なし。NSISの32bitスタブをx64のアプリ本体と混同しない。
MSIの版はFileVersionInfoに頼らず内部Propertyテーブルから取得した。
Windows Installer COMの読取専用`OpenDatabase(..., 0)`とSELECT、SummaryInformationの読取だけを行い、導入・修復メソッドは呼んでいない。
初回のPowerShellリフレクション呼出しはDISP_E_TYPEMISMATCHで失敗し、明示的なstring／int引数の直接呼出しへ修正して終了0。
この修正はGit除外の検査スクリプトだけで、MSIの変更ではない。

| 成果物 | SHA-256 |
| --- | --- |
| NSIS | `11A76A1BE037BA3171CDAF4D62743F45A0FDB4733CE1E29A5AD40C1434454C9C` |
| MSI | `39A630F695B7A370745ED4F9804947FB294F812BD28B90DF4C90E59D21C39DBB` |
| Release EXE（MSIビルド終了後） | `097E89BF56F43ADA3F60E13688E1EDF3385328BA5352C1B954667B6C61E57CB4` |
| Release EXE（NSISビルド終了後の保存コピー） | `EB24F50172A67E478C4EEBD2DD87CC4FD1D216A8F683CAABCAE7DD7054BFE7E7` |

| 成果物 | ファイル作成日時（2026-11-21 JST） | 最終書込日時（同日JST） |
| --- | --- | --- |
| NSIS | 12:37:55.5025511 | 12:37:58.6525508 |
| MSI | 12:41:29.1215199 | 12:41:29.4010000 |
| Release EXE（MSIビルド終了後） | 12:41:25.2657490 | 12:41:35.9685147 |
| NSISビルド終了後EXEの保存コピー | 12:37:58.8243182（コピー作成） | 12:37:58.6862537 |

両bundleログにEXEの配布形式情報パッチが記録されている。表のEXEは各コマンド終了後のスナップショットであり、
2回のビルドでハッシュは異なる。アーカイブからのペイロード展開・そのEXEとのハッシュ一致検査は今回行っていない。
`E/artifacts.json`と`E/msi-properties.json`に実測値を保存した。通常Release EXEは実行していない。
旧targetの配布関連4ファイルは開始時とサイズ・SHA-256が一致した（`E/old-artifacts-before.json`／`old-artifacts-after.json`）。

## 本番設定の静的監査

| 項目 | 確認結果 |
| --- | --- |
| identifier／保存先 | `com.listingpilot.app`を維持。Rustの`app_local_data_dir()`も維持。v0.1.1と通常保存先を共有するため、初回起動だけでもv5→v6移行が起こり得る |
| 本番CSP | script-src／style-srcはself、unsafe-eval／本番unsafe-inlineなし。外部CDN許可なし。devCspは別設定。ハッシュ／nonce自動付加の無効化なし |
| Asset Protocol | `$APPLOCALDATA/media/originals/**/*`のみ。staging／quarantine／DBは対象外 |
| Capabilities | `default.json`の`core:default`のみ。fs／dialogのJS権限追加なし |
| Schema | migrations::applyの上限6、0001〜0006は基準HEADと同一。IMMEDIATE／foreign_key_check／失敗時rollbackを維持 |
| 起動時保護 | DB読取専用事前確認→media OSロック→書込接続・Migration→ロック下で参照再読込→reconcileの契約を維持 |
| WebView2 | 明示設定なし。固定CLI 2.12.4のローカルschemaでdownloadBootstrapper、silent=trueが既定。minimumWebview2Version未指定。実導入・Runtimeなし環境は未検証 |
| NSIS／MSI | bundle.active=true、targets=allを維持。生成NSISはINSTALLMODE=currentUser、RequestExecutionLevel=user、ARCH=x64。MSIは生成WiXのInstallScope=perMachine、内部ALLUSERS=1。実際の権限要求は未検証 |
| ダウングレード | v6 DBをv0.1.1で扱える保証はない。user_versionだけを5へ戻さない。[バックアップと復元](backup-and-restore.md)の停止済みv5一式復元手順を維持。インストーラーの版判定はDB互換性保証ではない |
| Stickyアクションバー | CSSのposition:sticky／bottom:0と6画面のpage-action-barを確認。ProductForm／ProductDetail／ListingPreview／ShopifyExport／ShopifyImageUrls／ShopifyImageExportが候補ソースに含まれる |

生成NSISでは`INSTALLWEBVIEW2MODE=downloadBootstrapper`、`WEBVIEW2INSTALLERARGS=/silent`、
minimum版は空で、Runtimeを検出できない場合のダウンロード・導入処理を含む。
`DeleteAppDataCheckboxState=1`かつ更新モードでない場合にAppData／LocalAppDataのidentifier領域を削除する分岐がある。
チェックボックスの実表示・初期状態と、未選択時のデータ保持は実インストール／アンインストール未実施のため今回の成功範囲に含めない。
`ALLOWDOWNGRADES=true`も既定のまま維持した。これはインストーラーの版選択であり、v6 DBを旧版で扱えるという意味ではない。
静的根拠は`E/target/release/nsis/x64/installer.nsi`（抽出メモ`E/nsis-static-audit.txt`）。
生成WiXの`E/target/release/wix/x64/main.wxs`にもdownload bootstrapperと`/silent /install`、
`MajorUpgrade AllowDowngrades=yes`を確認した（抽出メモ`E/msi-static-audit.txt`）。実行はしていない。

## 最終差分とレビュー成果物

変更は次の8ファイル。自アプリ版更新5ファイルと記録文書3ファイルだけで、READMEは変更しない。

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `docs/verification.md`
- `docs/release-checklist-v020.md`
- `docs/phase62-release-build.md`（新規）

`.review/phase62/phase62.patch`には未追跡の新規文書も含め、`.review/phase62/changed-files.txt`に8ファイルを列挙する。
`git diff --check`とパッチの`git apply --reverse --check`は終了0。
最終HEADは基準のまま、main、7追跡ファイル変更＋新規文書1ファイル、ステージ変更なし。
`.review/`と`.verification/`はGit除外を維持し、インストーラー・EXE・ログ・合成SQLite／mediaは追加しない。

## 引き継ぐ未検証・未解決事項

- Windows `export_write` エラー5は未解決。今回のビルドやViteのEPERMとは別件であり、単品・一括・Shopify両方式／リポジトリ内外の出力をPhase 6.3で切り分ける。
- 最新Release WebView2・本番CSP・Asset範囲外拒否・Stickyバーの実GUI操作は未検証。
- NSIS／MSIの実インストール・削除、v0.1.1→v0.2.0更新、GUIでのバックアップ復元は未検証。
- WebView2 Runtime未導入・オフライン、別PC／VMは未検証。
- 未署名配布に伴うSmartScreen・発行元警告・組織ポリシー等の実表示／実行可否は未検証。今回署名基盤・Updater・CIは導入しない。
- 長期的な画像・DB保全、大規模データ、電源断・実ディスク故障、残存WAL等の復旧は保証しない。Phase 6.1の合成移行・サービス復元成功と区別する。

ユーザーのパッチレビュー後、隔離環境でのPhase 6.3へ進む判断を行う。ビルド成功だけで配布可能とは判定しない。
Phase 6.3でoverrideを使う場合は、通常候補と別の専用ビルドを作り、起動前に`app.appDirectoriesOverride`の解決先を検証する。
