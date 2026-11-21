# 開発環境の検証結果

## Phase 6.2 v0.2.0 Release候補ビルド（2026-11-21 日本時間）

基準HEAD：`cc58c3bafa63368d711c651e2726bbd62301ab92`、main、開始時クリーン。
Phase 6.1のユーザーレビュー承認後、5ファイル・6か所の自アプリ版だけを0.2.0へ更新した。
依存・Migration 0001〜0006・Schema v6・identifier・CSP・Asset Protocol・Capabilities・保存先は維持した。
生成元の差分・197追跡ファイルのSHA-256、全実行ログは新規
`.verification/phase62/70f0c0e5-6327-4108-83c3-6d499f8159ec/`に保管する。
詳細は[Phase 6.2候補ビルド記録](phase62-release-build.md)と[判定チェックリスト](release-checklist-v020.md)を参照。

`npm ci`はofflineでENOTCACHED（終了1）の後、専用キャッシュと承認されたネットワークで再実行し終了0。
`npm run build`はsandbox内でspawn EPERM（終了1）の後、同じソースを通常権限で再実行し終了0、45 modules。
fmt／check／clippy（-D warnings）は終了0。Cargoの依存取得はoffline、check／clippy／testは--locked。
Rustテスト初回は終了101、249成功／1失敗。長い専用TEMP配下でジャンクション作成準備のassertが失敗した。
同じ専用ディレクトリを指すWindows短縮パスへプロセス内TEMP／TMPを変更し、同じsandbox・同じソースで再実行した。
最終結果は終了0、**250 passed／0 failed／0 ignored**、main／doc-tests各0件。テストコードの変更なし。
既存Phase 6.1テストの今回新規4UUID出力（初回分を含む）はコピー後の248ファイルの全SHA-256一致も確認した。

NSIS／MSI生成はともに終了0。新規NSISは2,791,795 bytes、MSIは3,915,776 bytes、最終Release EXEは7,895,040 bytes。
版はすべて0.2.0、アプリ対象x64、全件NotSigned。MSI内部ProductVersion=0.2.0、SummaryTemplate=x64;0を読取専用COMで確認。
絶対パス・作成／書込時刻・全SHA-256・NSIS終了時EXEの別保存は上記候補ビルド記録を参照。
NSISはcurrentUser、MSIはperMachine（ALLUSERS=1）、WebView2はdownloadBootstrapperのまま。旧配布関連4ファイルのハッシュは不変。
最終git diff --checkと、新規文書を含むレビューパッチのgit apply --reverse --checkは終了0。
通常LocalDataは読取・書込・列挙・移動・削除していない。既存データや過去検証成果物は変更していない。
通常Tauri dev・通常Release EXE・インストーラーは起動せず、実インストール・一般公開も行っていない。
`export_write`エラー5は未解決。Release WebView2／本番CSP、実NSIS／MSI導入、v0.1.1からの更新、GUI復元、
Runtime未導入／オフライン、別PC／VM、SmartScreen等の実表示、長期保全・電源断は未検証のまま引き継ぐ。
Commit・Push・タグ・Release公開はせず、パッチレビュー後に隔離環境でのPhase 6.3へ進む判断を行う。

## Phase 6.1 リリース前監査・Schema v5→v6移行（2026-11-21 日本時間）

基準HEADは`c271589`、開始時の作業ツリーはクリーン。公開版v0.1.1／Schema v5と現行開発版Schema v6を監査した。
詳細・証跡・試験名は[Phase 6.1監査記録](phase61-release-audit.md)、後続ゲートは[v0.2.0リリースチェックリスト](release-checklist-v020.md)を参照。
バージョンは0.1.1のまま。Migration 0001〜0006、アプリ実装、依存・lockfile、identifier、CSP・権限は変更していない。

公開v0.1.1記録対象コミット`01f0a1b`とMigration 0001〜0005のGit blob一致を確認し、そのSQLから完全新規のv5 DBを生成した。
商品2件、画像1枚／4枚、公開文章、3グループ（非表示含む）、Text／Number等3定義・6値、PNG原本5枚と削除済み隔離PNG1枚を用意した。
専用パスで本番のstartup_serviceを呼び、起動前5→移行後6、integrity_check=ok、foreign_key_check=0行、
旧5テーブル全列・全行・旧スキーマ・UUID／画像順序・生成元とmedia全体のパス／サイズ／SHA-256保持、URL設定テーブル追加、再適用・サービス再起動を確認した。
追加2件のRustテストは`src-tauri/src/phase61_tests.rs`。毎回新しい`.verification/phase61/<UUID>/`を作り証跡を保持する。

DDL後半失敗・既存FK不整合・未来版7・media OSロック・破損DBの5ケースで起動を拒否し、
故障注入後を基準にDBを含む全ファイルのパス・サイズ・SHA-256一致を確認した。DB再作成や画像整理は行われなかった。
既存のMigration／初期化／2プロセスロック／参照再取得／非置換保護テストも今回全件成功した。

正常終了済みv5のDBとmedia一式（8ファイル・102,928 bytes）を新規バックアップ先へコピーし、
別の新規復元先へコピーして全manifest照合後、v5整合性→v6移行→サービス再起動・全旧データ／原本／隔離保持を確認した。
バックアップ自体は開かず、終了時も元のmanifestと一致。SQLite関連ファイルは当該セットには存在せず、WAL等の復旧は未検証。
バックアップ文書に現行版の手順を追記し、過去Phase履歴を保持した。v6→v0.1.1は保証せず、user_versionだけを5に戻すことは禁止。
戻す場合は移行後セットを退避し、更新前v5のDB・関連ファイル・media全体の一組から復元する。

最終ソース編集後の今回の実行結果：

| コマンド | 終了コード・結果 |
| --- | --- |
| `npm run build` | 0。TypeScript両設定＋Vite、45 modules。初回は1（sandboxのspawn EPERM）、同じコード・依存のまま承認された通常権限で成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 0、警告0 |
| `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture` | 0。250 passed／0 failed／0 ignored（既存248＋新規2）、main／doc-tests各0 |
| `cargo metadata --manifest-path src-tauri/Cargo.toml --locked --offline --format-version 1 --no-deps` | 0 |
| `npm ls --depth=0` | 0 |
| `git diff --check` | 最終文書・レビュー成果物作成後に0 |

新規2テストの初回絞込実行は終了101（1成功・1失敗）。Windowsロック中の.lockをテスト補助処理が読みエラー33になったため、
拒否観測後にテスト側ロックを解放してから全ファイルを照合するよう修正した。アプリ実装変更なし、最終全件実行で成功。
初回PowerShellプロファイルもfnmの権限エラーを出したため以後は読み込まず、永続環境は変更していない。

最終証跡は`.verification/phase61/`の`27068d91-44fd-41d6-9acb-afe4d79267de`（ログ・SQL由来照合）、
`aab4f629-4a7e-4207-a965-4d8950a84084`（成功移行・復元）、`4c4a14f1-9015-4fc7-aeaa-b24cad0a804e`（失敗保護）。初回失敗の証跡も保全した。
通常LocalDataは読み書きせず、既存DB・media・バックアップ・検証成果物は流用・変更・削除していない。
GUI・通常tauri dev・Release EXE・インストーラーは起動せず、tauri buildも今回未実行。

合成試験で重大な移行不具合は検出していない。**レビュー承認後にPhase 6.2のビルド準備へ進行可能、配布可否は未確定。**
v0.1.1の過去NSIS実機成功はv0.2.0更新の成功ではない。実更新・最新Release CSP／Stickyバー・GUI復元・別PC／MSI・WebView2未導入／オフライン・残存WAL等は未検証。
**Windows export_writeエラー5は未解決**。監視中リポジトリ内の最終非置換移動での過去記録を維持し、最新Release／10列／13列への影響は次工程へ残す。
OS強制終了・電源断・実ディスク故障は再現していない。実データコピー試験には別途明示承認が必要。
レビュー用`.review/phase61/phase61.patch`／`changed-files.txt`に検証DB・media・ログ・ビルド成果物は含めない。
Commit・Push・タグ・Release公開は行わず、Phase 6.2へ進まずレビュー待ちとする。

## Phase 5.2.4 操作ボタンの常時表示（2026-11-21 日本時間）

基準HEAD：`28b1cd9`。商品登録・編集、商品詳細、Shopify画像URL設定、Shopify CSV（10列／13列）、出品プレビューの操作を、各パネル／フォーム末尾の下部バーへ移動した。`products.css` の `position: sticky; bottom: 0; z-index: 1`、白背景・上境界線・折り返しボタンを使用し、通常フローの末尾余白と、対象画面表示時だけのdocumentの `scroll-padding-bottom` を確保した。グローバルなfixed配置は使用していない。

CSV方式選択も同じパネル内に置き、狭い表示幅の先頭でstickyの親領域がボタンを画面外へ押し下げないようにした。方式切替で再描画されるラジオへフォーカスを戻す。商品フォームの保存エラーはバー内、URL設定の保存不可理由・未保存状態はバー内に表示し、URL操作結果とCSV確認欄にはリンクで移動できる。

保存・戻る・出力の既存ハンドラ、refによる二重操作ガード、競合拒否、破棄／グループ確認、URLの保存可能条件、JPY確認・警告トークン・確認失効・ネイティブダイアログのキャンセル処理は変更していない。バーへ移した出力ボタンには、元の条件に加え、プレビュー未取得／読み込み中のdisabledを明示した（従来は未取得時にボタン自体を表示していなかった）。Rust、SQLite、Migration、IPC契約、CSP、Capabilities、依存・lockfile、App.cssは変更していない。

### 自動チェック

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 最終ソースで成功・終了コード0。TypeScriptとVite本番ビルド。sandbox内ではViteの子プロセス起動が `spawn EPERM` となったため、通常権限で再実行して成功。依存変更なし |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功・終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功・終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功・終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 成功・248 passed、0 failed。main／doc-testsは各0件 |
| `git diff --check` | 成功・終了コード0 |

### Windows実WebView2での確認

`npm run tauri dev -- --config .verification/phase524-20261121/dev-config.json` で起動。新規の専用 `app.appDirectoriesOverride` を `.verification/phase524-20261121/data-v3` に設定し、Path IPCの返却先との完全一致を検証してから操作した。通常LocalAppDataは使用していない。起動前にこの専用領域だけへ生成したSQLite・20枚のPNGを配置し、`integrity_check = ok`、`foreign_key_check = 0` を確認した。既存の検証データは流用していない。

既定800×600のネイティブウィンドウで起動し、実WebView2のCDPで表示領域を800×600／400×600に設定して検証した。IPCはモックではなく既存Rust実装を呼び出した。

| 項目 | 観測結果 |
| --- | --- |
| 6画面の表示・長いページ | 商品詳細・登録編集・URL設定・出品プレビュー・10列CSV・13列CSVの各画面で、両表示幅の先頭／中間／末尾にバーと全ボタンが表示領域内にあり、横はみ出しがない。商品詳細／URL／プレビュー／13列では20画像を使用 |
| 最終入力欄・確認欄 | 通常フローのバー前に末尾内容を表示できる。13列の確認欄リンクで、JPY・警告確認欄がバーより上に収まることを座標で確認 |
| 商品保存・戻る・競合 | バーのマウス操作で保存・一覧への戻りに成功。実IPCで更新内容を取得して永続化を確認。価格不正のエラーをバー内に表示。別IPC更新後の `stale_data` で入力を保持し再保存を無効化。競合エラー表示時も両表示幅・3スクロール位置でバーが可視 |
| URL設定 | 未保存状態を表示。バーの戻るで破棄ダイアログ、編集継続で入力保持、保存でaltを実IPCから取得して一致確認。未保存内容の破棄で商品画像へ戻る。モーダルがバーより上のtop layerに表示されることを確認 |
| 10列／13列の確認条件 | JPY確認と警告承認の片方だけでは出力不可、両方で有効。13列のストア変更でプレビュー・確認が失効し、再読込後は未承認に戻る |
| ネイティブ出力キャンセル | 10列・13列とも実フォルダー選択ダイアログを開き、表示中は出力・戻る・方式切替が無効。検証アプリのプロセスに属するダイアログだけをWM_CLOSEで閉じ、キャンセル通知とJPY・警告確認の解除を確認 |
| キーボード・フォーカス | 必須項目不足でSKUへフォーカスしバーの上に表示。フォーカスした保存ボタンのEnter、テキスト入力中のEnterの両方で保存成功。ラジオのArrowRightで13列へ切替し、選択ラジオへのフォーカスを維持 |

最終のUI確認結果は `ui-result.json`（16項目）、`native-result.json`（3項目）、`keyboard-result.json`（3項目）とスクリーンショットを、Git対象外の `.verification/phase524-20261121/` に保存した。最終実行のJavaScript例外・コンソールエラーは0件。途中の検証スクリプト修正履歴／失敗結果も同領域に残しており、最終成功結果とは区別する。開発サーバーから専用EBWebViewのCookiesへのwatchでEBUSY通知があったが、上記の操作検証は完了した。

確認後に開発サーバーを停止し、検証用 `listing-pilot` プロセスが残っていないことを確認した。

### 今回の未検証・UX上の制限

- 長時間の実保存中に連打するケース、URLの全競合・保存結果不明障害、グループ変更確認、一括貼り付け全ケースは今回の実WebView2では再検証していない。既存ガード／ハンドラを変更していないことは差分確認、Rust側の保護は既存248テストの結果による。
- CSV完成ファイルの生成・Shopify再インポート、再起動保持、Release版／本番CSP、インストーラー、別PCは今回未検証。出力操作の実機検証はネイティブダイアログのキャンセルまで。
- 警告・JPY確認は本文を読み、バー内リンクで確認欄へ移動する。商品編集・削除・画像操作・再読み込みの全ボタンを常時表示する変更ではない。
- バーは対象パネル／フォームの範囲に追従する。狭幅ではボタンが折り返し表示される。400px未満の表示幅、OSの拡大率変更、極端に低い表示高さ、スクリーンリーダーは今回未検証。

## Phase 5.2.3 最終受け入れ記録（記録日：2026-11-21 日本時間）

基準HEADは`c0bfcc2`（Phase 5.2.2C実装済み）。以下は**ユーザーによるWindows実WebView2およびShopify Dev Storeの手動確認結果**として記録する。Codex自身が今回実行・観測した結果ではなく、DB・CSV・Shopifyを再調査して追認したものでもない。基本的な実機・Shopify受け入れは完了とし、以下の確認範囲に限定する。

| 項目 | ユーザー確認結果 |
| --- | --- |
| 環境隔離・DB整合性 | PASS。Tauriの`app.appDirectoriesOverride`を使用し、専用LocalDataにSQLite・media・EBWebViewを作成。通常LocalAppDataは使用していない。SQLite Schema Version 6、`integrity_check = ok`、`foreign_key_check = 0` |
| 実WebView2での登録・保存・保持 | PASS。画像1枚／4枚の商品登録、Shopify画像URL・altの保存、再起動後の設定保持、13列CSV出力に成功 |
| 13列CSV | PASS。2商品・画像5行、UTF-8 BOMなし・LF。Handle・画像位置・altの対応を確認し、`青い小皿の正面写真`のalt出力に成功 |
| 10列CSVの実機回帰 | PASS。10列・商品2行、BOMなし・LFのみ。画像URL列を含まないことを確認 |
| Shopify Dev Store | PASS。画像4枚／1枚の商品登録に成功。4枚の画像順1〜4・メイン画像、altテキストが一致。両商品とも下書き・非公開。Shopifyから再エクスポートし、画像枚数・順序・altの対応を確認 |
| ストア分離・不正入力 | PASS。ストアA／Bの設定分離、不正URLの入力エラーと保存防止を確認 |
| 未保存変更の保護 | PASS。未保存変更時の破棄確認、キャンセル時の入力保持、破棄時の保存済みデータ保持を確認 |
| CSV出力先選択のキャンセル | PASS。キャンセルでき、完成CSVフォルダーを作成しないことを確認 |

Phase 5.2.2CでCodexが実行したRust自動テスト248件成功（既存221件＋新規27件）および隔離ブラウザーのIPCモック19シナリオ成功は、後掲の過去の自動検証記録である。今回のユーザーによる実機受け入れとは別の証拠として扱い、今回テストを再実行したとは記載しない。過去Phaseの「未検証」「今回未実施」は各Phase当時の状態であり、現在の確認範囲はこの節を参照する。

### 未検証事項・将来のリリース判定に残す項目

- COMMIT結果不明の実障害、実電源断、ディスク容量不足、OS強制終了中の保存。
- 別PC、MSI／NSISによる実インストール、最新機能の隔離Release版・本番CSPでの操作。
- 実WebView2における全競合ケース。上表を超える並び替え・画像削除・失敗時保護などの実機検証は、今回の報告から完了と扱わない。
- Shopify管理画面のAPI連携。手動インポート・再エクスポートの成功をAPI連携の検証とは扱わない。

自動テストの障害注入結果を実障害の検証に置き換えず、基本的な受け入れ完了を配布版・障害耐性全般の保証に拡大しない。既存の大量画像性能・URLの長期的な有効性など、今回確認範囲外の制限も維持する。

今回のCodex作業は記録確定の文書編集のみ。アプリ起動・再起動、通常LocalAppDataへのアクセス、DB・media・画像原本や検証データの変更・削除、Shopifyの商品・設定変更は行っていない。Rust・React・Migration・依存・セキュリティ設定も変更していない。
`git diff --check`は終了コード0。レビュー用パッチと変更ファイル一覧は`.review/phase523/`に置き、DB・media・検証成果物は含めない。Commit・Push・ブランチ・タグ・Release操作は行わない。

## Phase 5.2.2C Shopify画像付き13列CSV（2026-11-20〜21 日本時間）

基準HEADは`d8328ec`。開始時の作業ツリーはクリーン。画像付き専用のプレビュー・出力IPC、DTO、検証・CSV生成サービス、方式選択UIを実装した。
既存10列は従来のIPC・トークン・CSV内容・ファイル名・確定処理を維持し、商品候補を作る小さな関数と引用符処理などだけを共有する。
Schema v6、Migration 0001〜0006、依存／ロック、CSP・Capabilities・Asset Protocol、既存Workspace・汎用出力は変更していない。

最終ソースの検証結果（11月21日の継続作業では文書とレビュー成果物を仕上げ、チェックを再確認）：

| コマンド | 終了コード・結果 |
| --- | --- |
| `npm run build` | 0。TypeScript検査・Vite本番ビルド成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 0。既存221件＋新規27件＝248件成功、失敗0・無視0。main／doc testsは各0件 |
| `git diff --check` | 0。空白エラーなし |

Rustの追加27件は専用の一時SQLite・media・出力領域で実行した。13列用の画像は主に登録メタデータだけの架空画像で、原本が存在しなくても画像を読み取り・コピーせずCSVを生成することを確認した。

- 13列の正確な見出し、1／4画像、複数商品、先頭行と追加行の空欄位置、Handle反復、位置1始まり、商品UUID順、画像順とUUIDタイブレークを確認。
- 公開説明文の改行・カンマ・引用符、alt空欄・日本語・引用符・125／512スカラー値・補助平面／結合文字、BOMなしUTF-8・LFを確認。
- 15,000,000バイト未満の実レンダリング境界、URLの4,096／4,097バイト、保存値の再検証（不正URL・制御文字・数式・SHA不一致／不正）を確認。
- 画像0枚、URL未設定・他ストアのみ、商品内重複と正規化後の重複、他商品・他ストアとの分離、価格0／NULL、250／251画像、20／21商品、1商品blockedでの全件停止を確認。
- 商品編集、画像追加・削除・順序、URL・alt・設定解除・保存SHA・更新日時・選択商品集合・ストア変更でのトークン失効、10列／URL設定保存用トークンの流用拒否、警告承認の失効を確認。
- WALの別接続から商品とURLを同時更新しても、商品読取とURL読取が異なる時点へ混ざらないことを確認。
- ステージング後に別接続でURLを更新し、Mutexが解放されていることと確定前照合での拒否を確認。商品・alt・画像順・設定有無も最終照合の対象であることを確認。
- 既存完成出力の保持、非置換衝突、清掃失敗時の未知ファイル保持、移動結果不明時のCSV保持、アプリ保存領域への出力拒否を確認。
- 10列CSVの期待バイト列と従来のトークン構築を独立に比較し、画像URL設定を削除しても10列の内容・トークンを変えず出力できることを確認。既存の汎用単品・一括テストも全件成功。
- プレビューJSONにURL・altの原文、管理原本パス、危険な画像IDを含めず、商品UUID・安全な画像UUID・位置・理由を返すことを確認。

画像上限は[Shopify公式CSV資料](https://help.shopify.com/en/manual/products/import-export/using-csv#adding-multiple-product-images-in-a-csv-file)の1商品最大250画像と照合した。
251枚以上はプレビューのimage_limitと実行時のshopify_product_image_limitで拒否する。原本バイト数や汎用200画像／1GiB上限は使用しない。

TypeScriptの既存テスト基盤はないため依存・基盤は追加せず、`.verification/phase522c-ui-20261120-8d46/`に新規の一時HTML・メモリ内IPCモック・Node/CDPスクリプトを作成した。
Viteのみを`127.0.0.1:1437`で起動し、新規の一時user-data-dirを指定したheadless Chromeで実コンポーネントをStrictMode下で確認した。
`node .verification/phase522c-ui-20261120-8d46/run.mjs`は終了コード0、12シナリオ成功、未処理例外0・外部HTTPリクエスト0。
これはReact／APIラッパーのUI確認であり、実WebView2・Rust IPC・SQLiteへ接続した試験ではない。結果JSONと800px画面は同領域に保持し、レビュー用パッチへ含めない。

| UIモックで確認した項目 | 結果 |
| --- | --- |
| 10列の初期選択、ストア入力不要、既存IPCの利用 | 成功 |
| 13列でストア未入力なら取得／出力しない、登録枚数・不足・メイン・画像順・alt状態・blocked表示 | 成功 |
| JPY確認と警告承認の独立した制御、画像付き専用トークン・正規化ストアの送信、成功時件数／CSV場所表示 | 成功 |
| 方式・ストア変更でプレビュー／警告承認／JPY確認を失効、旧方式・旧ストアの遅延応答を破棄 | 成功 |
| 出力中の二重送信・方式／ストア切替・戻るを停止 | 成功 |
| キャンセル・stale_data・結果不明後の承認解除、エラー後に明示的な再読込を要求し自動再送信しない | 成功 |
| 不正ストアの日本語エラー、800px／600pxで横はみ出しなし | 成功 |

ReactのHooks・非同期結果の失効・入力ラベル・disabled／同期ref・status／alertについて既読のreact-best-practicesスキルの項目で確認した。
途中の検証ではサンドボックス内Viteがspawn EPERMで終了コード1となり、コード・依存を変えず承認された通常権限で再実行して成功した。
初回Rustテストは241件中238成功・3失敗（終了コード1）。テスト用の非正規URLをSQL制約の下で投入したこと、追加画像のSHA重複、警告なしデータへ警告承認エラーを期待したことが原因で、テスト準備を修正した。
境界テスト追加後の最終状態は上表の248件全件成功である。

### Phase 5.2.2Cレビュー修正：出力前のローカルサムネイル（2026-11-21 日本時間）

既存の未コミット変更を保持し、`ShopifyImageExport.tsx`と`products.css`へ表示を追加した。既存`listProductImages(productId)`の結果を商品ID・画像UUIDで照合し、CSV側の位置・メイン画像・URL／alt設定状態と併記する。配列インデックスで結合せず、`loading="lazy"`と`decoding="async"`を使用する。
取得失敗・preview_url欠落・画像読込失敗はプレースホルダーにし、CSVのblocked判定・出力可否は変更しない。CDN URLをimg要素に渡さず、URL設定・原本の更新APIも呼ばない。Rust・Migration・10列CSV・セキュリティ設定・依存はこの修正では変更していない。

新規の隔離領域`.verification/phase522c-thumbnails-20261121/`で、実コンポーネントとメモリ内IPCモックをheadless Chromeから確認した。
`node .verification/phase522c-thumbnails-20261121/run.mjs`は終了コード0、17シナリオ成功、未処理例外0・外部HTTPリクエスト0。既存12項目に加え、画像APIの逆順応答でもUUIDが一致すること、CSV順変更・メイン表示、欠落／取得失敗／描画失敗でも出力可能なこと、旧ストア・旧方式・旧商品選択の遅延応答を破棄することを確認した。
最終ソースで上表の6コマンドを再実行し、すべて終了コード0。Rustは248件成功、失敗0・無視0で維持した。UI試験用Viteの再起動は既存試験サーバーのポート使用中で一度終了コード1となり、起動済みの同サーバーを利用して試験を完了した。
実WebView2でのAsset Protocolによるローカル画像表示、実際の大量画像での性能は未検証。通常LocalAppData・既存SQLite・管理原本は使用せず、実Tauriアプリも起動していない。UIモック成果物はレビュー用パッチに含めない。

### 最終レビュー修正：サムネイル取得の待機分離（2026-11-21 日本時間）

`ShopifyImageExport.tsx`のCSVプレビュー取得とサムネイル取得の待機を分離した。loadingはCSV取得だけで解除し、画像一覧は商品ごとに独立取得して商品ID・画像UUIDで追加反映する。未完了・失敗ではプレースホルダーを維持し、CSV確認・出力を妨げない。世代番号・マウント状態による旧応答の拒否を維持した。Rust・Migration・依存・セキュリティ設定・10列CSVはこの修正では変更していない。

既存検証成果物を保持し、`.verification/phase522c-independent-thumbnails-20261121/`に新しい隔離UIモックを作成した。
`node .verification/phase522c-independent-thumbnails-20261121/run.mjs`は終了コード0、19シナリオ成功、未処理例外0・外部HTTPリクエスト0。従来17項目に加え、サムネイル応答を未解決のままCSVプレビュー表示・承認・出力成功まで進めること、現在の遅延応答はUUIDで正しく反映され承認を維持することを確認した。商品・ストア・方式変更後の旧応答破棄、取得失敗時のプレースホルダー、10列への影響なしも再確認した。
最終ソースで`npm run build`、`cargo fmt --manifest-path src-tauri/Cargo.toml --check`、`cargo check --manifest-path src-tauri/Cargo.toml`、`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`、`cargo test --manifest-path src-tauri/Cargo.toml`、`git diff --check`を実行し、すべて終了コード0。Rustは248件成功・失敗0・無視0。
実WebView2・大量画像性能は引き続き未検証。通常LocalAppData・実データ・管理原本は使わず、Tauriアプリは起動していない。全変更のレビュー用パッチ・変更一覧を更新し、UI検証成果物は含めない。

### Phase 5.2.3の手動受け入れチェックリスト（今回未実施）

通常LocalAppDataを共有しない専用ユーザー／VM／保存先を明確に隔離した専用ビルドで実施する。今回、通常のnpm run tauri devや実アプリは起動しておらず、既存SQLite・管理原本は使用していない。

- [ ] 実WebView2→Rust→隔離SQLiteで、10列／13列の方式切替とストア別のURL設定状態・blocked表示を確認する。
- [ ] JPY確認・警告承認・ネイティブ出力先選択のキャンセル、出力中の二重操作／方式切替／離脱を実操作する。
- [ ] 画像1枚／4枚／複数商品の13列CSVを実アプリで出力し、列・行・位置・alt・文字コードと原本非コピーを確認する。
- [ ] 同じ隔離環境でURL・alt・画像構成変更後の競合、清掃失敗・結果不明時の通知と承認失効を確認する。
- [ ] Shopify Dev Storeで既存Handle上書きオフの新規商品としてインポートし、画像順・メイン・altを再エクスポートで比較する。
- [ ] 配布版・別PC・実電源断、OSウィンドウ終了／beforeunloadの挙動を確認する。状態を構成した単体試験を実電源断試験としない。

URL形式の妥当性はCDN上の実在・画像内容の一致・ストア所有権・インポート成功を保証しない。Shopify API・HEAD確認・自動アップロードは追加していない。
レビュー成果物は`.review/phase522c/`の全差分・変更一覧・READMEのみとし、DB・media・秘密・ビルド出力・UI検証成果物を含めない。Commit・Push・ブランチ・タグ・Release操作は行わない。

## Phase 5.2.2B Shopify画像URL管理専用UI（2026-11-20 日本時間）

基準HEADは`0f6f637`、開始時の作業ツリーはクリーン。専用Reactパネル、APIラッパー、エラー表示、入力補助検証と必要なCSSを実装した。
Rust・Migration・Schema v6・依存／ロック・CSP・Capabilities・Asset Protocol・画像なし10列CSVは変更していない。13列CSVはPhase 5.2.2Cの対象。

最終ソースで次を実行した。

| コマンド | 終了コード・結果 |
| --- | --- |
| `npm run build` | 0。TypeScript検査・Vite本番ビルド成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 0。既存221件すべて成功、失敗0・無視0。Rustテスト追加なし |
| `git diff --check` | 0。空白エラーなし |

通常サンドボックス内の最初の`npm run build`はVite設定読み込み時の`spawn EPERM`で終了コード1。
TypeScript検査は通過しており、コードや依存を変更せず必要な権限で再実行して成功した。

TypeScriptのテスト基盤は存在せず、npm依存・テスト基盤は導入していない。
補助的なUI検証は新規の`.verification/phase522b-ui-20261120-6e39/`内の一時HTML・IPCモック・Node/CDPスクリプトで実施した。
Viteのみを`127.0.0.1:1436`で起動し、別の新規user-data-dirを指定したheadless Chromeで、実際のProductDetail／ProductImages／ShopifyImageUrlsをStrictMode下で表示した。
`get_shopify_image_urls`等の応答はメモリ内の偽物であり、Tauri・Rust IPC・SQLiteには接続しない。ストア・商品・画像は架空データ、プレビューはローカルSVGのdata URLである。
`node .verification/phase522b-ui-20261120-6e39/run.mjs`の最終結果は終了コード0、22シナリオ成功、未処理例外0、外部HTTP画像リクエスト0。
スクリプト初回の引用符誤りと、遅延画像が画面内へ入るのを待つ検証手順を修正して再実行した。アプリの画像遅延読み込みを変更して解消したものではない。
結果JSONと800pxの画面は同領域に保持し、Gitやレビュー用パッチには含めない。
検証用ブラウザープロファイルを作業領域内に置いたため、Viteの監視ログにはそのキャッシュファイルのロックによるEBUSYが出た。
アプリの監視設定は変更していない。UI検証完了後に専用ブラウザーとViteを終了し、検証用ポート1436／9346の待ち受けが0件であることを確認した。

| UI受け入れ項目 | ブラウザー＋IPCモックで観測した結果 |
| --- | --- |
| 0・1・4画像、商品名・SKU、UUID情報、メイン画像、ローカルプレビュー | 成功。0枚は保存操作なし。StrictModeでもクリック1回につき設定取得1回 |
| 個別入力、クエリ、4,096／4,097バイト、URL異常・alt空欄／125／126／512／513文字・補助平面文字／改行／数式 | 成功。入力を勝手に変えずエラーを表示し保存を停止 |
| 全画像保存、成功時の返却値採用、URL＋alt解除 | 成功。変更なし画像も送信し、画像の追加・削除・並べ替えIPCは呼ばない |
| 一括貼り付け4行、不足・超過・末尾改行・空行・重複・不正URL | 成功。検査エラーで個別入力を変えず、空行のURL／alt解除を確認表で明示 |
| 一括確認・適用・適用前キャンセル・適用後の戻るキャンセル・保存エラー | 成功。適用だけでは保存IPCなし。エラー時に入力保持 |
| A／Bストア分離、未保存時の切替確認、セッション内候補 | 成功。入力中の別ストアが未読込なら保存を停止し、Aの編集をBへ引き継がない |
| 取得中のストア切替、旧A応答、強制アンマウント後の取得／保存応答 | 成功。新しい画面・ストアへ古い結果を反映しない |
| 保存中の連続クリック・ストア変更・戻る、beforeunload | 成功。送信1回、操作停止、合成beforeunloadイベントをキャンセル |
| stale_data、image_url_save_unknown、通常の入力エラー、画像取得エラー | 成功。編集保持、自動再送信なし。競合／不明では再保存を停止し、明示的な再取得と破棄確認を要求 |
| 設定SHA不一致 | 成功。全対象行の明示確認なしには保存不可。URL編集で確認が失効 |
| 画像順序・追加／削除・SHAに相当するAPI応答の不整合 | 成功。UUID／順序／SHA不一致では保存せず、自動整列・補正しない |
| 欠落・読み込み失敗サムネイル | 成功。プレースホルダーと案内を表示し、外部CDN画像を代用しない |
| キーボード、入力ラベル、800px／600px、長いURL | dialogのキャンセル初期フォーカスとEscape、各入力のラベル対応、狭幅の縦積みと横はみ出しなしを確認 |

Reactの構造・Hooks・アクセシビリティについてreact-best-practicesスキルの項目で確認した。
取得・保存はEffectへ置かず、非同期結果の世代確認、同期保存ref、UUIDのkey、入力ラベルとエラー／成功のroleを使用している。

### Phase 5.2.3で行う手動受け入れチェックリスト（今回未実施）

通常LocalAppDataを共有しない専用Windowsユーザー／VM／明確に保存先を隔離した専用ビルドで行う。
今回、通常の`npm run tauri dev`や実アプリの起動は行っておらず、既存SQLite・管理原本を使用していない。

- [ ] 実WebView2→Rust IPC→専用SQLiteで0・1・4枚の取得、個別保存・再取得・再起動、altとクエリの保持を確認する。
- [ ] A／Bストアで設定を分離し、未保存時のストア切替・戻る・再取得の破棄確認を実操作する。
- [ ] 一括4URL、件数不足／超過、末尾改行、空行、重複・不正URL、適用前後のキャンセルを確認する。
- [ ] 設定解除がURLとaltだけに作用し、ローカル原本・Shopify Filesを変更しないことを確認する。
- [ ] 専用環境での画像順序変更・追加・削除・別操作によるURL変更、設定SHA不一致に対する競合・明示確認を確認する。
- [ ] 遅延応答、保存中の二重操作・離脱、stale_data／image_url_save_unknown、画像取得失敗時の入力保持を実IPCで確認する。
- [ ] 実WebView2でキーボード操作・スクリーンリーダー・OSウィンドウ終了／再読込とbeforeunloadの挙動を確認する。合成イベントの結果をネイティブ終了防止の保証としない。
- [ ] 本番CSP／Asset Protocol下のサムネイル、配布版・別PCを確認する。13列CSVと実ShopifyインポートはPhase 5.2.2C以降で行う。

レビュー用差分と変更一覧は`.review/phase522b/`へ置く。既存`.verification/`の他の成果物は変更・削除していない。
Commit・Push・タグ・Release・ブランチ操作は行わず、コードレビュー待ちとする。

## Phase 5.2.2A Shopify画像URL管理バックエンド（2026-11-20 日本時間）

基準HEADは `ea08363`（ADR 0014設計確定）。開始時の作業ツリーはクリーン。
Migration 0006／Schema v6、Rustの検証・Repository・Service、取得／全件保存の2つのTauri Commands、削除サービスとの整合性を実装した。
専用React UI・一括貼付け画面・13列CSV・画像付き出力トークン・Shopify APIは未実装。コードレビュー待ちであり、Phase 5.2.2全体の完了ではない。

最終ソースでの検証結果：

| コマンド | 終了コード・結果 |
| --- | --- |
| `npm run build` | 0。TypeScript検査とVite本番ビルド成功。Viteの子プロセス制約を避けるため承認された通常権限で実行 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 0。既存191件＋新規30件＝221件成功、失敗0・無視0。main／doc testsは各0件 |
| `git diff --check` | 0。空白エラーなし |

追加30件は次を含む。SQLiteはメモリDBまたはUUID付きの専用一時ディレクトリ、mediaは同じ一時領域だけを使用した。

- v5→v6移行、既存商品・画像・グループ・属性・属性値の全列／旧スキーマ保持、再適用、DDL途中失敗とforeign_key_check失敗時のロールバック。
- Shopify CDN URLの許可・拒否、scheme／host／明示443の正規化、クエリ保持、入力値と正規化後の4,096バイト境界、重複の適用範囲、ストア正規化・境界・不正形式。
- altの空欄・125／126／512／513文字、補助平面文字・結合文字、制御文字・改行・CSV数式拒否、入力不正時の全ストア保持。
- URL保存・更新・解除・再接続後の永続化、登録SHA、原本・取り込み元・商品情報の不変、複数ストア／商品／platformの分離、トークンの別商品・別ストア流用拒否、変更なし保存。
- 並べ替え後のUUID対応、画像追加・削除・順序・SHA・設定URL／alt／SHA／更新日時・未設定変更による失効、不完全／重複／不正画像集合の拒否。
- 2行目のSQL失敗、更新の無視・予期しないトリガー変更、遅延外部キーによるCOMMIT失敗での全件保護、共通permitの競合拒否。
- 画像／商品削除の全ストアCASCADE、他画像・他商品設定と他商品の管理原本保持、SQL／COMMIT失敗での画像・URL設定・原本の保持。
- COMMIT結果確認時のURLだけの不整合・孤立参照・参照テーブル不明・未終了トランザクションの検出と原本保持、削除確定・URL参照なしの場合の判定。
- camelCase、未知フィールド拒否、パス／ファイル名を返さないDTO、SQLの主キー・外部キー・長さ制約と既存RESTRICT。

途中の検証では、通常サンドボックス内の `npm run build` が終了コード1（Vite設定読み込み時の `spawn EPERM`）。
コード・依存を変えずに通常権限で再実行して成功した。初回Rustテストは214成功・2失敗（終了コード1、旧Schema v5期待値の更新漏れ）。
またclippyでテストの不要なformat!が1件検出された（終了コード1）。修正後、最終ソースに対して上表の全チェックを実行し成功した。

通常の `%LOCALAPPDATA%\com.listingpilot.app`、既存SQLite、管理原本にはアクセスしていない。アプリを起動していないため、公開済みv0.1.1のデータも移行していない。
Schema v6はv0.1.1では開けない。実WebView2から新IPCを呼ぶ操作、専用UI、実Shopifyの画像実在・所有権・インポート、
配布パッケージの作成・インストール、別PC、電源断・実ディスク故障は今回未検証。
COMMITの曖昧な結果は状態を構成したテストで確認しており、実電源断による再現ではない。URLの形式検証は画像内容の一致を保証しない。
Git差分で既存Migration 0001〜0005、依存／ロック、10列CSVと既存出力確定処理、CSP・Capabilitiesに変更がないことを確認した。
レビュー用パッチは `.review/phase522a/` に置き、検証成果物・DB・media・build出力を含めない。Commit・Push・タグ・Release・ブランチ操作は行わない。

## Phase 6.0 Windows実インストール・アンインストール検証（2026-11-20 日本時間）

対象は **v0.1.1**、コミット `01f0a1baa8d6622a984fe57aa34b0b0b39b2df0b`（`01f0a1b`）。
検証日は2026-11-20、環境はWindows 11 x64の実機1台。
**以下はユーザーが実施し、成功を報告した検証結果であり、Codexが自動実行・直接確認した結果ではない。**

| ユーザー実施の検証項目 | 報告結果 |
|---|---|
| NSISインストール、スタートメニュー登録、初回起動 | 成功 |
| 商品登録、画像追加・プレビュー・並び替え | 成功 |
| CSVエクスポート | 成功 |
| アプリ再起動後の商品・画像保持 | 成功 |
| Windows標準画面からのアンインストール | 成功 |
| アンインストール後のアプリ登録・スタートメニュー項目の削除 | 成功 |
| アンインストール後のSQLite、`media/originals`、出力済みCSVの保持 | 成功 |
| アンインストール後の再インストールとデータ再利用 | 成功 |
| NSISのアプリデータ削除オプションの初期状態 | オフ |

上記の範囲について、従来の未実施記録をユーザー報告による実機検証成功として補足する。
既存のPhase 6.0Aおよび以前の記録は、当時の検証状況として削除・改変せず保持する。
今回の成功報告は当該実機1台と上記操作の範囲に限り、Phase 6.0のすべての検証完了とは扱わない。

別PC／VM、MSIの実インストール・アンインストール、旧版からのアップグレード時のデータ保持、
アプリデータ削除オプションをオンにした場合の削除範囲、電源断・障害復旧は引き続き未検証。
未署名配布時のOS警告の具体的な内容、WebView2／CSPの詳細検査についても、今回の報告から検証済みとはしない。
CSVの形式・出力先や詳細な内容照合は今回の報告では特定されていない。
**Windowsの `export_write` エラー5は未解決のまま保持する。** 今回のCSV出力成功から解決済みとは判断しない。

今回の編集は本ファイルへの追記のみ。ビルド・自動テスト・インストーラー実行は行っていない。
CodexによるSQLite・管理画像・既存検証データの変更・削除、Commit・Push・タグ変更・Release変更は行っていない。

## Phase 6.0A テスト版 v0.1.1 の準備（2026-11-20 日本時間）

開始時の `git status --short` は空、`git log -1 --oneline` は
`27d29b4 docs: propose Shopify image URL mapping`。基準HEADは
`27d29b4456995998cb7c07aa673c9f63b10a3c28` と一致した。
このHEADに自アプリのversion更新だけを加えた未コミットソースから、Windows x64向けNSISを生成した。
既存 `v0.1.0` タグのオブジェクトは作業前後とも
`aca527839e6b5c95fe0d0365bc954fd6bdbdd63d`、参照コミットは
`ee30fafbb8f188f31f20a42567a3186bf311ef00`。タグ操作・Commit・Push・Release公開は行っていない。

`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` を0.1.1へ更新。
`npm version 0.1.1 --no-git-tag-version --ignore-scripts`（終了0）でpackage.jsonとpackage-lock.jsonを同期し、
`cargo metadata --manifest-path src-tauri/Cargo.toml --offline --format-version 1 --no-deps`（終了0）でCargo.lockを同期した。
ロックファイルの差分は自アプリのversionのみ。ビルド対象の差分は5ファイル・6行のversion置換で、
依存・機能コード・データ構造・identifier・CSP・capabilitiesに変更なし。

環境はWindows x64、Node.js 24.16.0、npm 11.13.0、Rust／Cargo 1.99.1、
host `x86_64-pc-windows-msvc`。PowerShellは既存fnmプロファイルの権限エラーを避けるため、
初回確認後はプロファイルを読み込まず実行し、プロファイルや永続環境設定は変更していない。

| コマンド | 結果 |
|---|---|
| `npm run build` | 初回終了1、Vite設定読込のspawn EPERM。同じコマンドを制限外の通常権限で再実行し終了0、41 modules |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了0、Clippy警告なし |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了0、191 passed / 0 failed / 0 ignored、main／doc-test各0件 |
| `npm run tauri build -- --bundles nsis` | 終了0、09:31:45.962〜09:34:55.028 JST。beforeBuildCommandのTypeScript／Viteも成功、NSISのみ1件生成 |
| `git diff --check`（本記録追記後） | 終了0、空白エラーなし。Cargo.lockのLF→CRLF通知あり |

ビルドは既知のVite子プロセス制限を避けて制限外の通常権限で実行した。
既存identifier末尾 `.app` に関するmacOS向け警告は残るが、Windowsビルドは成功した。
Rustテストは既存のメモリDB／UUID付き専用一時ディレクトリを使用する。
通常のSQLite・LocalAppData・管理画像・既存検証用データは変更・削除せず、生成アプリも起動していない。

新しい配布候補は次のNSISのみ。**旧0.1.0 NSIS／MSIは0.1.1の配布物として扱わない。**

| 項目 | 実測値 |
|---|---|
| パス | `src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.1_x64-setup.exe` |
| サイズ | 2,736,769 bytes |
| SHA-256 | `CA0B59360669DA5F7D6BDAD25796031785937C57834AFF82AD7971754F95A9D6` |
| Authenticode | NotSigned、署名者／タイムスタンプ証明書なし |
| FileVersion／ProductVersion | ともに `0.1.1` |
| ProductName／FileDescription | ともに `Listing Pilot` |
| 最終書込時刻 | 2026-11-20 09:34:54.9594993 JST、今回のビルド開始後 |
| 対象アーキテクチャ | Tauri出力はx64、生成アプリのPE Machineは `0x8664`（AMD64） |

生成アプリ `src-tauri/target/release/listing-pilot.exe` は7,710,208 bytes、NotSigned、
FileVersion／ProductVersionとも0.1.1、SHA-256は
`97BE841BAB6389CE621EEB8EFD385638428D17E786251F72F8824770EC1A72E2`。
旧0.1.0 NSIS（2,737,933 bytes、SHA-256 `13BE8AEB025EA8635EC086322125AA75B512F6A15D4A30D4CD046C3AF272868C`）と
旧0.1.0 MSI（3,833,856 bytes、SHA-256 `462FCAA3577523D51E94ED0BD384C02B52237E6076067FA9ED26CA421A7792EB`）は
作業前後でハッシュが一致した。MSIの再ビルドは実施していない。

証跡はGit管理外の `.verification/phase60a-20261120-v011/`。
各コマンドのログ・終了コード、ビルド時刻、成果物メタデータ、ビルド対象差分と追跡179ファイルのSHA-256を保存した。
ビルド対象差分 `build-source.patch` のSHA-256は
`282A42A817DC08D0DBADB3B99BF3D5C738091A3661463385A267C56396E8029B`。
ビルド後・本記録追記前に追跡179ファイルがビルド開始時のハッシュと全件一致した。
最終変更は上記5ファイルと本記録の計6ファイル、未追跡ファイルなし、HEADと既存タグは維持。

NSISの実インストール・アンインストール、生成アプリの起動、実WebView2、別PC／VM、
アップグレード時のデータ保持、未署名配布時のOS警告、電源断・障害復旧は今回未検証。
**Windowsの `export_write` エラー5は未解決のまま保持する。** ビルド成功を解決の根拠とはしない。
後続の導入検証はユーザーの判断で別途実施する必要がある。

## Phase 6.0 Windowsインストーラービルド・検証準備（2026-11-19 日本時間）

対象HEADは `c01cc1d19587badb151e39d6ede6b7ef9cae9d07`（main、HEADを指すタグなし）。
開始時は未追跡の `docs/decisions/0014-shopify-image-url-mapping.md` のみで、追跡済みファイルに差分なし。
ADR 0014を変更せず保持し、現行設定のままNSISとMSIを個別に生成した。
**今回の範囲はビルドと検証準備まで。実インストール・アンインストール・生成アプリの起動は行っていない。**

環境はWindows 11 Home 25H2 x64（10.0.26200.9974）、Rust／Cargo 1.99.1 stable/MSVC、
Visual Studio Insiders 18.9.12023.133、MSVC 14.51.36231（link.exe 14.51.36252.0）、SDK 10.0.26100.0。
プロファイルなしPowerShellでNode.js 24.16.0／npm 11.13.0、Tauri CLI 2.12.4、TypeScript 7.0.2、Vite 8.3.6を使用。
既存fnmプロファイルの権限エラーを避けるためプロファイルを読み込まず、永続環境設定は変更していない。

| コマンド | 結果 |
|---|---|
| `npm run build` | 初回終了1、Vite設定読込のspawn EPERM。制限外の通常権限で同じコマンドを再実行し終了0、41 modules |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了0、警告なし |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了0、191 passed / 0 failed / 0 ignored、main／doc-test各0件 |
| `npm run tauri build -- --bundles nsis` | 終了0、11:57:40〜12:00:42 JST、181.07秒 |
| `npm run tauri build -- --bundles msi` | 終了0、12:01:08〜12:03:59 JST、171.21秒 |

既存のTypeScript専用テストスクリプト／テストファイルはなく、両TypeScript設定の型検査とViteビルドを確認した。
RustテストのDB・画像はメモリ／UUID付き専用一時領域を使用する。通常LocalDataは対象にしていない。
両インストーラービルドは既知のVite子プロセス制限を避け、制限外の通常権限で実行。
identifier末尾 `.app` のmacOS向け警告あり。依存・version・identifier・CSP・capabilitiesの変更や、
VBSCRIPT有効化・管理者権限取得・署名設定・OS設定変更は行わず、両ビルドが成功した。

| 成果物 | サイズ（bytes） | SHA-256 | Authenticode |
|---|---:|---|---|
| `Listing Pilot_0.1.0_x64-setup.exe` | 2,737,933 | `13BE8AEB025EA8635EC086322125AA75B512F6A15D4A30D4CD046C3AF272868C` | NotSigned |
| `Listing Pilot_0.1.0_x64_en-US.msi` | 3,833,856 | `462FCAA3577523D51E94ED0BD384C02B52237E6076067FA9ED26CA421A7792EB` | NotSigned |

通常出力先はそれぞれ `src-tauri/target/release/bundle/nsis/` と `src-tauri/target/release/bundle/msi/`。
NSISの作成／最終書込は12:00:38.7238402／12:00:41.9587976 JST、
MSIは12:03:52.6014737／12:03:52.8850000 JST。今回のビルド開始後の生成であることを照合した。
NSISのFileVersion・ProductVersionとMSI内部ProductVersionは `0.1.0`。
MSI内部はProductName `Listing Pilot`、ProductLanguage `1033`、SummaryTemplate `x64;0`。
MSIはWindows Installer COMで読み取り専用OpenDatabase（モード0）を使用し、導入／修復処理は呼んでいない。
両成果物とも署名者・タイムスタンプ証明書なし。未署名に対するOS警告・実行可否は未確認。

最終MSIビルド後の `listing-pilot.exe` は7,710,208 bytes、version 0.1.0、NotSigned、
SHA-256 `491A20035292602CBB1FDED7A29D3A421A6ECC69D56D450F8A3C18563F6E3B5B`。
Tauriが配布形式情報をパッチするため、MSI生成前のNSIS用EXEも別保存した。
そのSHA-256は `9DA0C9DA7D07ED10C20C977A47D91432A968A096E169BA3921B29E2F2B9538EC`。

生成スクリプトの静的確認ではNSISがcurrentUser／user権限、MSIがperMachine（内部ALLUSERS=1）。
NSISのWebView2設定はdownloadBootstrapper。NSISにはアプリデータ削除チェックと、
選択時にAppData／LocalAppDataのidentifier領域を再帰削除する処理がある。
後続の隔離試験では未選択として状態を記録する。未選択時のデータ残存も実測まで保証しない。

ビルド直後・本追記前に、追跡178ファイルと既存ADR／指定レビュー文書の計182ファイルを開始時SHA-256と比較し、全件一致。
今回の追跡ファイル変更は本記録のみ。既存SQLite・media・画像原本・開発保存データにはアクセスせず、
コード・両ロックファイル・アプリ設定を変更していない。Commit・Push・Release公開・タグ作成も行っていない。

証跡はGit管理外の `.verification/phase60/build-20261119-115515/`。
各コマンドのログ・終了コード・開始終了時刻・対象SHA、環境、旧／新成果物情報、MSI内部情報、
変更前後のファイルハッシュ、同一ハッシュを確認した成果物コピー、NSIS／WiX生成スクリプトを保存した。
レビュー報告・後続試験手順・本記録差分はGit管理外の `.review/phase60/`。

計画のI-01〜I-12はNSIS・MSIともにすべて未実施。VMも今回作成・操作していない。
ユーザーの成果物レビュー後、復元可能なWindows x64 VMで両形式を独立に試験する。
実導入・初回起動・実WebView2／CSP・DBと画像の再起動保持・CSV操作・権限要求・
アンインストール時の削除範囲とデータ残存、別PC・電源断は検証待ち。
ビルド成功をインストール成功やPhase 6.0全体完了とは扱わない。

## Phase 5.1 Shopify Dev Store受入テスト成功（2026-11-19 報告・日本時間）

ユーザーから、Listing Pilotで2商品のShopify用CSVを出力し、Dev Storeへインポート後、
Shopifyの再エクスポートCSVと照合して受入テストが成功したとの報告を受けた。
以下はユーザー実施・照合の結果であり、CodexがShopifyを操作したり、再エクスポートCSVを直接確認した結果ではない。

| SKU | 価格 | Status | 公開状態 |
|---|---|---|---|
| `LP-P51-MUG-002` | 0円 | `draft` | 非公開 |
| `LP-P51-DISH-002` | 1,200円 | `draft` | 非公開 |

- 商品名・日本語説明文・ブランド・SKU・Handleが正常で、再エクスポートCSVとの照合でも一致した。
- Option1は `Title / Default Title`、Option2・Option3は空欄だった。
- 画像を含めない仕様も維持されていた。

これにより、下記2026-11-17の記録でユーザー担当・未実施としていた本実装CSVのDev Storeインポート／再エクスポートは、
上記2商品について受入成功となった。以前の記録は当時の検証状況として保持する。
ストア通貨設定の確認方法、既存Handleの上書き設定、インストーラー・別PC・障害復旧については、今回の報告から追加の検証済み事項とはしない。
今回は本記録のみ追記し、コードは変更していない。文書のみの変更のためRust／TypeScriptのビルド・テストは再実行していない。

## Phase 5.1 Shopify専用CSV（2026-11-17 日本時間）

開始HEADは `ee30fafbb8f188f31f20a42567a3186bf311ef00`。開始時の既存変更は未追跡のADR 0013のみ。
ADR案を仕様として保持し、状態と実装上の共通化／確認状況を追記した。Commit・Push・branch操作は実施していない。
Schema Version 5、Migration、依存／ロック、識別子、CSP・Asset Protocol・Capabilitiesは変更していない。

### 最終ソース状態での自動確認

| コマンド | 結果 |
|---|---|
| `npm run build` | 終了0。TypeScript両設定・Vite本番ビルド成功、41 modules |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了0、警告なし |
| `cargo test --manifest-path src-tauri/Cargo.toml --quiet` | 終了0。**191 passed / 0 failed / 0 ignored**、main／doc-test各0件 |
| `git diff --check` | 終了0、空白エラーなし（GitのLF→CRLF通知あり） |

既存176件を維持し、Shopify用15件を追加した。固定10列・UUID順・1商品1行、全セル引用符、
日本語／カンマ／引用符／説明文改行を独立CSVリーダーで往復し、実ファイルのBOMなし・LFを確認。
管理メモ非流出、0円／NULL価格、タイトルフォールバックと説明文warning、Vendor空欄、
1〜20件／不正UUID・重複・欠落商品・21件、未知入力フィールド拒否を確認した。
SKU・Title・Description・Vendorの先頭空白／不可視文字／各行の数式開始文字をプレビューと実行で拒否する。
DB制約が許さない先頭ASCII空白は候補スナップショット単位で検査し、制約を緩和していない。

集合順序非依存、Handleの改名／SKU変更非依存、単品・汎用一括トークンとの分離、warningToken必須、
商品・公開文章・価格・ブランド変更、ダイアログ後・確定直前の別SQLite接続変更を検査した。
最終照合前にDB Mutexが解放されていること、途中失敗時の全ステージング清掃、既存宛先非置換、
未知ファイル保持と清掃失敗の通知、確定結果不明時のCSV保持、管理領域への出力拒否も確認した。
画像0枚と原本欠落でも実出力成功。201画像／2GiBメタデータは候補単位で汎用上限非適用を確認した。
15MB未満の厳密な境界と日本語UTF-8実バイト長の超過拒否を確認。既存単品／一括全テストも成功。

初回テストは190成功／1失敗（先頭空白SKUが既存DB制約で拒否されたテスト設定の誤り）。
設定を修正し全191件成功後、最終ソース状態で再実行して全191件成功した。
初回Clippyのテスト内clone指摘5箇所は修正済み。Viteの初回はsandboxのspawn EPERMで失敗し、
コード・依存を変えず必要な権限で同じコマンドを再実行して成功。最終実行も成功した。

### 実Tauri／WebView2

`npm run tauri dev -- --config .verification/phase51/dev-config.json` を実行した。
新規 `.verification/phase51/data` へappDirectoriesOverrideを限定し、CDP接続時にPath IPCの返却先を照合。
通常LocalDataや以前の検証データは使用せず、新規3商品／1グループを既存Rust IPCで作成した。
Reactボタン・チェック・グループ選択を実WebView2で操作し、ネイティブ選択／キャンセルはWindows UI Automationを使用。

| 確認した境界・場面 | 観測結果 |
|---|---|
| 一覧 → Shopify確認 | グループをまたいだ2商品のUUID選択を保持。0件は無効。二重クリック・StrictModeでもプレビューIPCは1回 |
| 確認UI → Rust候補 | 管理名フォールバック／説明文未設定の警告、SKU・タイトル・Vendor・価格・Handleを表示。画像Issue・管理メモを表示しない |
| 確認の独立性 | 警告だけを承認してもJPY確認までは出力不可。画像0枚のready商品でもJPY確認が必要 |
| 出力中の操作 | 二重クリックでも出力IPCは1回。戻る／再読込は無効、カスタム項目設定はinert |
| キャンセル／再入場 | ネイティブキャンセルを独立表示。警告と通貨の両確認を解除。再読込・一覧へ戻って再入場でも解除 |
| 古い確認 | 実IPCで公開タイトルを変更後、出力をダイアログ前に拒否。候補を消して再読込と再確認へ誘導 |
| CSV生成 → 結果表示 | 2商品と1商品の両方で実出力成功。リポジトリ外の専用OS一時フォルダーに完成フォルダー1つ・CSV1つを生成し商品件数／場所を表示 |
| 出力内容 → 読戻し | BOMなしUTF-8・LF、日本語・引用符・説明文改行を確認。商品一覧は出力前後で完全一致 |
| blocked | 価格NULL商品は両確認後も出力不可。0円商品は成功したCSVに含む |
| 数式リスク | Vendorの数式文字列を実Rustプレビューで拒否。UUIDと日本語の列名を表示し、原文を出さない。明示再読込で復帰 |
| 既存汎用経路 | 実UIから汎用一括・単品もそれぞれ出力成功。汎用CSVのBOM＋CRLFを確認 |
| 実行時ログ | integrationシナリオのJavaScript例外0・ブラウザーログerror 0、CSP違反の記録なし |

出力親はOS一時領域の新規 `listing-pilot-phase51-ui-*`。監視中リポジトリ内のWindowsエラー5を
解決済みとは扱わず、安全検証・権限・非置換確定は緩和していない。
開発監視ログには隔離WebView2のロックファイルに対するEBUSYが記録されたが、CSV出力と上記操作は成功した。
シナリオ完了後のclose IPCは既存Capabilitiesにより拒否された。権限を追加せず、起動したPIDと実行パスを
照合してCloseMainWindowで通常終了（WaitForExit成功）し、開発セッションを停止した。
この後片付け時の拒否はintegrationのエラー0件とは分けて記録する。

証跡は `.verification/phase51/result.json`、`csv-audit.json`、`two-product-preview.png`、
`two-product-success.png`、`null-price-blocked.png`、`formula-vendor-error.png`。
最終ソースの変更はProductListの字下げのみで、その後にTypeScript／ViteとRustの全チェックを再実行した。

### 未検証と引継ぎ

本実装CSVのShopify Dev Storeインポート／再エクスポートはユーザー担当で未実施。ストア通貨の照会も行っていない。
JPY確認・一致Handle上書きオフを前提に、新規テスト用2商品CSVと期待値をレビュー提出先へ同梱する。
インストーラー／Release・別PC・ネットワーク共有・電源断／実ディスク障害は今回未検証。
今回のCSV追加ではMigration・媒体管理やインストーラー設定を変更しておらず、`npm run tauri build` は実行していない。
レビュー成果物は `.review/phase51-20261117/` の差分パッチ・新規ファイルZIP・SHA-256 manifest・日本語実装報告書。

## Phase 4.6 レビュー後の2点修正（2026-11-17 日本時間）

変更対象はbatch_export_service、AppError、日本語エラー表示、ProductList、関連Rustテストと本記録だけ。
前回提出した作業ツリーを維持し、単品エクスポートのAPI・CSV・エラー契約は変更していない。

- 一括のCSV事前検査でだけ `CsvFormulaRisk` を `BatchCsvFormulaRisk` へ変換する。
  IPCは `{code:"batch_csv_formula_risk",product_id:商品UUID,column:問題列}`。
  表示に対象商品UUIDと日本語の列名を含め、元の数式文字列・ローカルパスは含めない。
  他のエラーはそのまま伝播する。単品の `{code:"csv_formula_risk",column:問題列}` は維持する。
- グループ取得失敗時は空の選択肢一覧と同時にgroupFilterを空へ戻す。
  成功応答でも選択中UUIDが存在しなければ同様に「すべてのグループ」へ戻す。
  有効なUUIDは改名・再取得後も保持する。選択済み商品Setは変更しない。

### 修正後のチェック

| 確認 | 結果 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了0。**176 passed / 0 failed / 0 ignored**、main/doc-test各0件。既存175件を維持し、新規1件を追加 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功、警告なし |
| `npm run build` | 終了0。TypeScript両設定とVite本番ビルド成功、39 modules |
| `git diff --check` | 成功 |

既存の数式リスク全体停止テストを商品UUID・列の厳密な照合に強化した。
新規テストではSKU・公開説明文の問題列と商品UUIDをJSONで検査し、同じ商品を単品出力した際の
従来エラーJSONが変わらないことも実サービスで確認する。全体停止・出力先が空・DB不変性も維持した。
Viteの初回はサンドボックス内でspawn EPERMとなり、同じコマンドを必要な権限で再実行して成功した。

### 実WebView2での修正箇所の確認

verificationスキルの既存方針に従い、新規 `.verification/phase46-review1/data` だけに保存先を限定した。
`npm run tauri dev -- --config .verification/phase46-review1/dev-config.json` で起動し、接続時にPath IPCを照合。
通常LocalData・前回検証データは変更せず、新規の2商品・1グループだけを使用した。

- 改名・正常再取得では同じグループUUIDの絞り込みを保持し、表示中1件／非表示1件の選択を保持。
- グループ一覧応答だけを取得エラーへ限定注入。選択欄は「すべてのグループ」、実表示は2件、選択済み2件のまま。
- 成功応答から現在のグループだけを除く限定注入でも、選択肢と表示が同時に全グループへ戻る。復帰後も選択UUIDを保持。
- 全グループへ戻った後の「表示中の選択を解除」「表示中をすべて選択」が実際の2商品に作用する。
- 実Rust IPCと一括確認UIから数式エラーを起こし、問題商品のUUIDと「出品候補タイトル」を表示。
  正常商品のUUIDと元の数式文字列はエラー文に含まず、古い候補は無効化される。
- UUIDの入力順を逆にしても同じ商品・列を返す。単品の従来日本語数式エラー文も完全一致。
- 最終シナリオのJavaScript例外・ブラウザーログerrorは各0件、CSP違反の記録なし。

証跡は `.verification/phase46-review1/result.json`、`groups-failed-all-visible.png`、`batch-formula-product.png`。
フォルダー選択より前に止まる数式エラーと一覧絞り込みの確認に限定し、今回の修正後はRelease／インストーラーを再生成していない。
前回の全出力・再起動検証は以下の記録のとおりで、今回は再実行していない。
レビュー成果物は `.review/phase46-review1-20261117/`。Commit・Pushは行っていない。

## Phase 4.6 一括エクスポート（2026-11-17 日本時間）

開始時HEADは指定の `1a20c79`、作業ツリーはクリーン。変更前のRust **158件成功・失敗0**を確認した。
商品選択、所属グループ絞り込み、一括確認／警告承認、全件成功方式の汎用CSV・原本画像出力を追加した。
単品のCSV生成と原本コピーを共用し、単品API・トークン・CSV契約を維持する。
Schema Version 5、Migration、依存ロック、CSP、Capabilities、Asset Protocolに変更はない。
設計は [ADR 0012](decisions/0012-batch-product-export.md) を参照。

### 最終変更後の自動チェック

| 確認 | 観測結果 |
|---|---|
| `npm run build` | 終了0。TypeScript両設定・Vite本番ビルド成功、39 modules |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功、指摘なし |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功、指摘なし |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **175 passed / 0 failed / 0 ignored**、main/doc-test各0件 |
| `npm run tauri build` | 終了0。通常設定のRelease EXE・MSI（3.62MiB）・NSIS（2.59MiB）を生成 |
| `git diff --check` | 成功、空白エラーなし |

Releaseは最適化ビルド2分51秒で成功した。生成先は `src-tauri/target/release/listing-pilot.exe`、
`src-tauri/target/release/bundle/msi/Listing Pilot_0.1.0_x64_en-US.msi`、
`src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.0_x64-setup.exe`。
インストーラーは実行・配布していない。既存identifierの末尾 `.app` に対するmacOS向け警告は従来どおりで、変更していない。

Vite初回はサンドボックスの子プロセス制限による `spawn EPERM`。必要な実行権限で同じコマンドが成功した。
この回避のためのコード・依存・環境設定の変更はしていない。

新規Rustテスト17件は既存単品Fixtureの子モジュールとして追加し、OS一時ディレクトリの専用DB・media・出力先だけを使う。
以下を検証した。

- 0件・重複・不正／非正規UUID・不在商品・商品件数上限、入力順非依存トークン、対象集合変更、警告承認不足。
- 画像200枚と1GiBの境界／超過、不正サイズ、checked_addによる集計。境界値にはメモリ上のSnapshotを使い、1GiBを実コピーしていない。
- 2商品、繰り返し出力のバッチUUID相違、単品出力との3CSVのバイト一致、BOM・CRLF・引用符・NULL／0・内部メモ非公開の既存回帰。
- 別グループ同名属性のUUID区別、旧グループ／非表示値除外、負数・先頭ゼロ・小数精度保持。
- ready／warning混在、画像なし商品の承認後出力、JPEG・PNG・WebPの原本バイト・画像順・メイン画像。
- CSV数式リスクの商品が1件でもあれば事前検査で全体停止、画像欠落時blockedと復元後の旧トークン拒否。
- 商品・画像順・属性定義・値・グループ名／所属変更による旧確認拒否。
- 別SQLite接続から出力中に更新し、最終全商品再照合で完成公開を拒否。コピー段階でDB Mutexを保持しないことも確認。
- 2商品目の画像ハッシュ不一致で1商品目を含め全ステージングを清掃、サイズ不一致、出力前後のDB不変性。
- 固定レイアウト以外のパス・ADS・traversal拒否、create_new、既存完成先衝突と内容保持。
- 未知の入れ子ファイルによる清掃失敗で元エラー／残存場所を返す。入れ子junctionは書き込みも清掃も拒否し、リンク先を保持。
- 確定直前にステージングを別名へ移動させ、結果不明として清掃せず原本出力を保持する。

既存158件は削除せずすべて回帰実行した。既存単品の画像検証・媒体ロック・衝突・清掃・Migration・グループ保存テストも成功している。

### 実Tauri / WebView2

`npm run tauri dev -- --config .verification/phase46/dev-config.json` を使用した。
`appDirectoriesOverride` を新規の `.verification/phase46/data` に限定し、CDP接続ごとにPath IPCの返却先が
この専用領域であることを照合した。通常のLocalDataは開いていない。
CDPでReactのボタン・入力を操作し、Rust IPC・SQLiteをモックせず読み戻した。
ネイティブ画像選択・フォルダー選択・キャンセルはWindows UI Automationで操作した。

| シナリオ | 観測結果 |
|---|---|
| 選択と絞り込み | すべて／所属グループ切替、表示中だけの全選択／解除、非表示選択の保持、全解除、0件出力無効が成功 |
| 改名・一覧再取得 | グループ改名後も選択UUIDを保持。一覧外で削除した選択商品を再読込で除外し、残り2件の選択を保持 |
| 全商品確認 | SKU・商品名・所属・ready／warning混在・候補・現在属性・画像合計2枚を確認。旧グループ値を含まない |
| 二重送信 | 同一評価内で開始ボタンを2回押下してプレビューIPCは1回。StrictModeのEffect再実行でも再送なし。出力ボタン2回でもIPCは1回 |
| 操作固定 | プレビュー読込を保留した間と出力中は戻る／再読込が無効。確認画面では一覧選択を変更できず、項目設定はinert |
| warning | 未承認の出力は無効。明示承認後だけ出力可能。フォルダー選択キャンセル・再読込・一覧へ戻って再入場で承認解除 |
| 取得失敗 | プレビュー応答だけをエラーへ限定注入。古い候補を表示せず、日本語エラーと明示再読込で復帰 |
| 実一括出力 | ネイティブ選択からOS専用一時フォルダーへ2商品を1つの完成フォルダーとして出力。商品数・属性数・画像数・出力先を画面表示 |
| 出力内容 | 商品UUID別の3CSV、BOM・CRLF終端・引用符・現在属性・Number精度、PNG2枚のバイト／SHA-256・順序を照合 |
| 不変性 | 出力前後の商品一覧・編集スナップショット・画像メタデータ一致。管理原本と取り込み元のハッシュも一致 |
| 確認後の変更 | 別IPCで商品タイトルを更新後、UI出力がダイアログ前に拒否。候補を無効化し、日本語エラー・最新再確認へ誘導 |
| 実blocked | 専用データの登録済み画像を一時的に欠落させ、実Rustプレビューのblockedと全体出力無効を確認。直後に同じバイトを戻し復帰 |
| 出力失敗後の再操作 | 監視中リポジトリ内では最終移動が失敗、ステージングの全清掃と日本語エラーを確認。再読込後にOS一時フォルダーへ成功 |
| 再起動 | 自分が起動したテストアプリだけをCloseMainWindow＋WaitForExitで正常終了。同じ隔離設定で再起動し、商品・全値・定義・画像メタデータ・元画像ハッシュ一致 |
| 再起動後の選択 | セッション内選択は0件へ戻り、改名済みグループと商品を表示。新規一括確認で警告承認が必要 |
| 実行時エラー | 成功した最終integration／additional／restartシナリオで捕捉JavaScript例外・ブラウザーログerrorは各0件。CSP違反の記録なし |

検証スクリプトの初期試行にはセレクターとIPC回数計測の誤りがあり、修正して再実行した。
終了用IPCは既存の権限で拒否されたため、権限を追加せずOSの正常ウィンドウ終了を使用した。
これらの検証操作由来のエラーは、最終アプリ操作シナリオの結果と区別している。

証跡はGit除外の `.verification/phase46/` に保存した。
`baseline.json`、`diagnostic.json`、`integration-result.json`、`additional-result.json`、`restart-result.json`、
`batch-preview.png`、`batch-success.png`、`batch-blocked.png`、`restart.png` を参照。
開発監視はテスト用WebView2キャッシュに対するEBUSYも出したが、IPC・データ照合は成功している。

### 既知のexport_write調査

変更前の既存単品エクスポートで、監視中の `.verification/phase46/baseline-out` では `export_write`、
OS専用一時フォルダーでは成功を再現した。単品基準商品は画像0枚。
追加した内部診断（段階名・ErrorKind・OS番号だけでパスや商品情報を含めない）では
`Export stage=final_move kind=PermissionDenied os=Some(5)` を観測した。
ステージング作成・CSV書き込み／syncは完了し、最終非置換移動で失敗、清掃後の親は空だった。
一括でも画像2枚のコピーを含め同じ最終移動エラーを再現し、全清掃後に別の一時フォルダーで再試行成功した。

観測上は監視中のリポジトリ配下に依存する。監視プログラム等のハンドル競合は可能性であり、
具体的なロック元や一時的アクセス拒否の根本原因は未確定。パス検証・Windows移動フラグ・権限を緩和していない。
開発時はリポジトリ外の専用フォルダーを使う。管理DB・管理mediaへの出力は禁止のまま。

### 残る確認範囲

- 実ディスク容量不足・電源断・強制終了時のファイルシステム耐久性、ネットワークドライブ、他PC。
- 上限に近い実画像200枚・合計1GiBの長時間コピー。上限判定は合成Snapshotで検証済み。
- 実画面でのJPEG／WebP一括出力は未実施（Rustでは両形式のバイト一致を確認）。
- 清掃失敗・移動結果不明はRustで模擬し、UIの実OS障害としては再現していない。
- 通常設定のRelease EXEでの実操作、本番CSP下の今回の一括フロー、インストーラー導入・更新・アンインストール。
- 同一ユーザーの任意のファイル差し替え攻撃全般への耐性は既存モデルの範囲。電源断後の出力ステージング自動清掃は追加していない。

Commit・Push・リリース公開は行わない。レビュー成果物は `.review/` に置き、コードレビューとユーザー手動テストを待つ。


## Phase 4.5.2 第2回レビュー後の軽微改善（2026-11-17 日本時間）

AttributeManagerの操作後再取得で、操作前の選択グループ名と入力欄が一致している場合だけ
最新名へ同期する。未保存の改名入力と競合時の保持・再読込案内は維持する。
グループ作成確認には、成功した場合にほかの未保存入力を破棄し、失敗時は保持する旨を明記した。

- `npm run build`：終了0。TypeScript両設定・Vite本番ビルド成功（38 modules）。
- 実WebView2・専用 `.verification/phase452/data` で6シナリオ成功：別IPCの改名後に項目作成して最新名同期、
  未保存改名がある再取得では入力保持、競合／明示再読込、改名成功、重複作成失敗時の全入力保持、作成成功時の選択／フォーム初期化。
- 確認文言をdialog上で照合。捕捉JavaScript例外・ブラウザーログerrorは0件。
- 結果はGit除外の `.verification/phase452/review2-improvements-result.json`。`git diff --check` 成功。
- 今回はRust・Schema・CSV・画像管理を変更せず、Rustテスト・Release／インストーラーの再検証は実施していない。

## Phase 4.5.2 グループ管理・商品所属UI（2026-11-17 日本時間）

開始時HEADは `a44c5b3`、作業ツリーはクリーン。グループAPI／型、小さな確認dialog、
グループ別項目管理、商品所属選択・一覧／詳細表示を実装した。
Rust・Migration・Schema Version 5・CSP・Capabilities・Asset Protocol・依存ロックは変更していない。
実装判断は [ADR 0011](decisions/0011-attribute-groups.md) を参照。

### 最終ソースに対するチェック

| コマンド | 観測結果 |
|---|---|
| `npm run build` | 終了0。TypeScript両設定とVite本番ビルド成功、38 modules |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了0。**158件成功・失敗0・無視0**、main/doc-test各0件 |
| `npm run tauri dev -- --config .verification/phase452/dev-config.json` | 起動と正常終了後の再起動に成功、実WebView2で操作 |
| `git diff --check` | 終了0。新規API／dialogも別途確認 |

初回Viteビルドはサンドボックスの子プロセス起動で `spawn EPERM`。実行権限を拡張して成功。
コード・依存変更で回避していない。通常設定のRelease／MSI／NSIS再生成は今回実施していない。

### 実WebView2のシナリオ別結果

`appDirectoriesOverride` を `.verification/phase452/data` に限定し、Path IPCの返却先が
この専用領域であることを毎回照合してから操作した。通常のLocalDataには接続していない。
CDPで実際のReactフォームを入力・クリックし、保存結果を実Rust IPCで読み戻した。
Windowsネイティブファイル／フォルダー選択はUI Automationで操作した。

| シナリオ | 結果 |
|---|---|
| グループ一覧・既定識別・改名 | 固定UUIDが初期選択。既定改名後も同じID。既定の非表示ボタンは無効 |
| グループ作成・重複 | 作成後の再取得と自動選択成功。同名は拒否して入力保持 |
| 項目管理 | グループ別の表示、別グループ同名許可、同一グループ同名拒否、改名・非表示／復元成功 |
| グループ非表示／復元 | 未使用で成功。項目作成が無効になり、復元で定義を再利用。所属商品ありは日本語エラー |
| 管理画面の未保存入力 | 切替キャンセルでグループ名・項目名保持、確定でフォームだけ破棄。新規グループの自動選択にも確認あり |
| 表示設定 | グループ非表示チェックと項目非表示チェックが独立して機能 |
| 新規商品 | 既定初期選択、別グループへ登録、選択定義だけ保存 |
| 商品切替確認 | 未変更なら即切替。未保存値ありはdialog。キャンセル／実Escapeで元の選択・値を保持 |
| 商品切替確定 | 未保存カスタム値だけ破棄。SKU・商品名・公開タイトル保持。戻ると保存済み値を再表示 |
| 所属変更保存 | 旧／新グループ名を確認。キャンセルではDB所属不変。承認後は指定UUIDに変更し旧値を保全 |
| 一覧・詳細 | グループ改名を一覧・詳細に反映。詳細の値は現在グループだけ |
| 商品・定義・グループ競合 | 別IPCで更新してからUI保存し拒否を確認。編集入力保持、勝手な再送なし。再読込にも破棄確認 |
| 読込失敗 | グループ／定義取得失敗時に商品保存禁止、管理一覧失敗時に変更禁止。商品一覧操作は継続 |
| 保存後の再取得失敗 | グループ作成は「保存済み・再読込失敗」と表示して再送を停止。商品保存は成功通知と一覧取得エラーを分ける |
| 二重送信 | 同じ評価内でフォームsubmitを2回実行。グループ作成・商品保存とも実IPCは1回、DBも1件 |
| 無効グループ | フォーム開始後に選択先をアーカイブ。保存拒否で入力保持、商品は作られない |
| 非同期読込中のキャンセル | 定義取得を保留してフォームを閉じ、応答を解放しても一覧画面を維持 |
| 画像回帰 | 実ダイアログでPNG2枚を取り込み、Asset表示、並べ替え、個別削除に成功。取り込み元のハッシュ不変 |
| プレビュー／CSV | 現在グループの陶器値だけを出力し、旧グループの木値を含まない。古いsnapshotTokenはstale_data |
| 出力品質 | 一時ディレクトリで実フォルダー選択→UIエクスポート成功。CSVの固定ヘッダー・BOM・CRLF、画像のバイト／SHA-256一致を確認 |
| 出力時の不変性 | 出力前後の編集スナップショット、管理画像バイト、取り込み元ハッシュが一致 |
| warning／blocked | warning実商品で未確認時の出力無効。blockedは応答を限定注入して出力無効を確認。実画像の破壊はしていない |
| 商品削除 | 全グループの値2件を確認表示し、対象商品と全値を削除。別商品の値・画像は保持 |
| 正常終了・再起動 | 同じ専用領域で商品・所属・グループ・定義・全保存値・画像メタデータが一致。旧グループ値の再表示と実画像描画も成功 |

読込失敗・保存後再取得失敗・二重送信待機は、実WebView2のIPC fetch境界で限定注入した。
本番コードに検証用分岐は追加していない。全成功シナリオで捕捉JavaScript例外と
ブラウザーログのerrorは0件（CSP違反ログなし）。通常CSPのReleaseでの操作は未実施。

アプリ停止中に専用SQLiteを読取専用で検査し、user_version=5、integrity_check=ok、
foreign_key_check=0件を確認。主商品の全グループ値2件・削除商品の値0件、画像ハッシュ一致を再確認した。
既存Rust回帰はpreview/exportのblocked、文字コード・列順、JPEG／PNG／WebP原本不変も含む。

### 証跡・制約

Git対象外の `.verification/phase452/` にスクリプト、結果JSON、スクリーンショットを保持。
主な成功結果は `result.json`、`failure-scenarios-result.json`、`additional-result.json`、
`integration-result.json`、`database-result.json`、`restart-result.json`。
初期検証スクリプトの待機条件・管理画像相対パスの修正前の失敗記録も保持している。
スクリーンショットは `groups.png`、`membership-confirm.png`、`conflict.png`、
`preview-export.png`、`restart.png`。新規依存・CI用フロントエンドテスト基盤は追加していない。

ワークスペース内 `.verification/phase452/out` を選んだ出力は `export_write` となった。
既存検証と同様の `%TEMP%` 配下の新規出力先では成功したが、前者の原因は未確定。
既存の出力実装は変更せず、この出力先での問題は残る検証上の制約として報告する。
Vite監視は専用WebViewプロファイル内のロックファイルに `EBUSY` を出したが、
実IPCとUIシナリオは完走した。通常利用の保存領域・画像・既存検証成果物は変更していない。

インストーラー導入・上書き更新、通常保存先での操作、別PC、実電源断・ディスク障害、
通常Release CSPでの今回のUIは未検証。検証用プロセスは確認後に正常終了した。
ステージング・Commit・Pushは行わず、レビュー待ちとする。

## Phase 4.5.1 グループ専用フィールド・Schema v5（2026-11-16 日本時間）

基準HEADは `1f2dcc7`。着手時の作業ツリーはクリーン。契約と移行手順は
[ADR 0011](decisions/0011-attribute-groups.md)。既存0001〜0004、依存ロック、アプリ識別子、
CSP・Asset Protocol・Capabilities、メディア管理実装は変更していない。

| 実行コマンド | 結果 |
|---|---|
| `npm run build` | 成功、終了0。TypeScript両設定とVite本番ビルド |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功、終了0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功、終了0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功、終了0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **158件成功、失敗0・無視0**。既存146件＋グループ関連12件。main/doc-testは各0件 |
| `npm run tauri build` | 成功、終了0。通常設定のWindows Release本体・MSI・NSISを生成（導入・起動は未実施） |
| `git diff --check` | 成功、終了0。未追跡のMigration・Rust・ADRも別途確認 |

### 自動テスト

最初にMigration専用3テストを成功させ、その後保存・読取・出力を実装して全件を再確認した。
テストは専用メモリDBまたは一意の一時DB・media・出力先を使用し、通常LocalDataは使用していない。

- foreign_keys=ONでv0〜v4→v5、v5再実行、v6/将来版拒否。全既存列・ID、2商品×2定義の全値、
  Numberの先頭ゼロ・小数文字列、非表示状態、画像メタデータと参照を保持。
- 最終トリガー生成時に故意に失敗させ、コピー・旧テーブルDROP・RENAME後もDDL・値・商品・user_version全体が戻ることを確認。
- 移行後のforeign_key_checkは0行、integrity_checkはok、foreign_keysは1。NULL・不在所属拒否、
  グループ内の非表示項目を含む名前重複拒否、別グループの同名許可、定義所属の変更拒否。
- グループの作成・決定的一覧・改名・名前検証・BINARY一意・競合・復元、既定/商品所属ありのアーカイブ拒否。
- 商品作成時の既定、更新時の所属保持、所属変更前の値保持と元グループの値再利用。
  別グループ項目の更新/クリア、変更後所属と不一致の旧項目、無効/不在所属を拒否。
- 有効な値と不正項目が混在する保存、および途中SQL失敗で、商品・所属・更新日時・全対象値をロールバック。
- 現在グループだけの編集候補・プレビュー・CSV。所属変更だけでも旧snapshotTokenを拒否。
  グループ情報、定義所属、非表示状態がトークン比較に入ることを確認。
- 全グループの値を含む削除確認、所属だけが変わった古い確認の拒否、全値の削除と定義保持。
  既存画像取込・順序・削除・隔離、CSV列/BOM/CRLF/画像順/原本コピーの回帰も全件成功。

### 実デスクトップ・再起動

`npm run tauri dev` 相当のCLIを `--no-watch --config .verification/phase451/dev-config.json` で起動。
保存先はappDirectoriesOverrideにより `.verification/phase451/data/` に隔離し、実IPCのPath応答でも一致を確認した。
デバッグポート9366の実WebView2をCDPで操作し、モックは使用していない。

- グループ4IPCの呼出しと、同名・古い更新日時・既定/使用中グループのアーカイブ拒否。
- 既定グループを改名後、旧項目設定が既定の項目だけを表示すること。
- 旧商品フォームから既定項目だけを新規保存。別グループ商品を開くとその所属の項目だけを表示・保存し、旧値と所属を保持。
- 別グループ値のクリア拒否時に商品名も保存されないこと。競合時に商品名・値の入力が残り、保存ボタンが無効になること。
- 所属切替・アーカイブ/復元後も両グループの値を保持し、旧プレビュートークンによる出力を拒否。
- WindowsネイティブダイアログからPNGを取り込み、Asset Protocol経由で画像表示。
  現在グループだけのプレビューと、専用のOS一時出力先へのCSV・原本出力に成功。
- プロセスを停止して同じ隔離先で再起動。実IPCで所属、両グループの値2件、現在グループの候補1件、画像1枚を再取得し、画面表示も確認。
- 最終成功セッションと再起動セッションでJavaScript例外・コンソールエラー0件。

停止後の隔離DBを読取専用で確認し、user_version=5、foreign_key_check=0行、integrity_check=ok。
保持値2件、管理画像1件のサイズ・SHA-256と原本バイトの一致を確認した。
成果物はGit対象外の `.verification/phase451/` に保持した。開発プロセスとデバッグポートは停止済み。

### 実行環境の制限と未検証

Viteビルドとネイティブダイアログ補助プロセスはサンドボックスでspawn EPERMになり、同じ処理を通常権限で実行して成功した。
リポジトリ配下の検証用出力先ではexport_write、OS一時フォルダーでは成功した。
開発サーバーが同配下を監視している間、WebView2のCookies等にはEBUSYも記録された。
監視による影響が疑われるが、export_writeの直接原因は断定していない。アプリコードの回避変更は加えていない。
Tauri buildでは既存識別子末尾 `.app` のmacOS向け警告が出たが、Windowsの両パッケージ生成は成功し、識別子は維持した。

通常LocalDataと実ユーザーデータの移行、通常設定Releaseの画面操作、インストーラー導入/更新/削除、
別PC、電源断・ディスク故障は未検証。グループ管理・選択UIはPhase 4.5.2に残す。
運用DBで初回起動する前に、全アプリを終了しDBとmedia全体を一組でバックアップすること。
ステージング・Commit・Push・リリース公開は行っていない。

## Phase 4.5 商品1件のCSV・原本画像出力（2026-11-16 日本時間）

基準HEADは `0d0a740`。着手時の作業ツリーはクリーン。スキーマVersion 4、既存Migration、依存ロック、
アプリ識別子、CSP・Asset Protocol・Capabilitiesは変更していない。実装契約は [ADR 0010](decisions/0010-single-product-export.md)。

| 最終実行コマンド | 結果 |
|---|---|
| `npm run build` | 成功、終了0。TypeScript両設定の型検査とVite本番ビルド。Tauri buildでも再成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功、終了0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功、終了0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功、終了0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **146件成功、失敗0・無視0**。既存125件＋出力専用21件。main/doc-testは各0件 |
| `npm run tauri build` | 成功、終了0。Windows Release本体・MSI・NSIS生成 |
| `git diff --check` | 成功、終了0。未追跡の新規ソースも別途確認 |

Viteはサンドボックス内で子プロセス起動が `spawn EPERM` になった。アプリや依存は変更せず、通常権限で再実行して成功。
Tauri buildは既存識別子の末尾 `.app` に関するmacOS向け警告を出すが、識別子は維持した。

### 自動テストの確認範囲

出力専用21テスト内で複数入力を検証している。すべて一意の専用一時DB・メディア・出力先を使用。

- 日本語・カンマ・引用符・複数行、BOM、CRLF、候補タイトルの出典、内部メモの非混入、NULL価格と0円。
- Number負数・先頭ゼロ・小数6桁の保持、非表示属性の除外、属性・画像0件のヘッダー。
- `= + - @`、先頭空白・タブ・制御文字・不可視文字・埋込改行の数式リスク拒否、SKU・公開文章・属性名/値/単位の対象列識別。
- JPEG・PNG・WebPのバイト一致、登録サイズ・SHA-256照合、欠番のあるDB順序からCSVだけ連番化、管理DBの全表比較とVersion 4。
- ready、warningの同一候補確認必須、blocked、商品・SKU・価格・文章・定義・値の変更、画像追加/削除/並替えによる競合。
- コピー後・確定前の商品変更の拒否、メディアpermit競合と解放後の再実行。
- LocalData配下・非ディレクトリ・欠落出力先、Windows junction出力先と管理画像の親junctionの拒否。
- 画像欠落、サイズ/ハッシュ不一致時の全体中止と清掃、同名完成フォルダーの非置換、別の親への再実行。
- create_new失敗、途中作成失敗、未知の追加ファイルを残す限定清掃、清掃失敗時の元エラー＋残存場所、確定結果不明時の保持。

### 実WebView2（隔離した開発版）

`npm run tauri dev -- --config .verification/phase45/dev-config-2.json` で起動。
`appDirectoriesOverride` による専用 `.verification/phase45/data2` を実IPCで確認してから操作した。
通常のLocalData・既存フィクスチャは操作していない。デバッグ接続と開発版は検証後に停止。

| シナリオ | 観測した結果 |
|---|---|
| ready正常出力 | 実ネイティブフォルダー選択→UI成功表示、商品1件・画像1枚、CSV3ファイルと画像を確認 |
| warning確認後出力 | 未確認ではボタン無効、未確認IPCはexport_warning_required。明示確認後に成功 |
| blocked | 専用管理画像を退避して欠落を再現。UI出力無効・IPCはexport_blocked。原位置へ戻すとready・実画像表示 |
| キャンセル | warning・readyでネイティブダイアログをキャンセル。専用通知を表示し、成功表示なし |
| 完成先衝突 | 同じ親へ再出力しexport_collision、既存出力と原本を保持 |
| 日本語CSV・画像 | 日本語SKU/公開文章、BOM、内部メモ非混入。元画像と出力のバイト一致・登録SHA-256一致 |
| 画像なし | 警告確認後にUI成功表示、画像0枚・空のimages/とヘッダー付きCSV |
| 変更後の競合 | 公開タイトル更新後、古いsnapshotTokenによるIPCをstale_dataで拒否 |
| 二重送信 | ダイアログ表示中のボタン無効と二回目のexport_productがmedia_busyになることを確認 |
| 再起動 | 同じ隔離データで再起動し、商品全体・画像メタデータ・画像バイト・既存出力を比較。プレビュー画像も表示 |
| 既存機能回帰 | 再起動後にネイティブ追加取込、順序変更と共通プレビュー反映、画像削除UI、商品削除UI。独立した出力成果物は残る |
| JS/CSP | 対象の実行区間でJavaScript例外0、securitypolicyviolation 0 |

検証証跡はGit対象外の `.verification/phase45/` に保持。
`smoke-5.mjs` は画像なしUI出力・原本一致まで成功した後、検証コードの拒否結果の取得方法で停止。
`tail.mjs` は衝突・競合・blockedと復元まで成功した後、検証画像寸法の誤った期待値で停止。
修正した `finish.mjs` と `restart.mjs` は終了0で成功し、`finish-result.json`・`restart-result.json` を保存。
初回のスクリプト文字コード・ダイアログ要素識別子の問題も検証用コードで修正した。これらをアプリの成功と混同しない。

### 制限・未検証

- 開発版でリポジトリ配下を出力先にした一回の試行はexport_writeで安全側に停止し、不完全な完成フォルダーは公開されなかった。
  ファイル監視のEBUSYも観測したが、因果関係は未確定。専用OS一時フォルダーでは同じ出力操作に成功。
- 実WebView2での確認は開発版。Release本体の画面操作、本番CSP下の今回の出力操作、MSI/NSISの実インストール・更新・アンインストールは未実施。
- 別PC、実電源断、実ディスク故障/満杯、ネットワーク共有、外部プログラムによる検査直後のパス差替えは未検証。
- 清掃失敗・確定結果不明は決定的な自動テストで確認した。実ストレージ故障による同条件は再現していない。
- Windows以外は完成ディレクトリの安全な非置換移動を実装しておらず、出力を失敗させる。
- Gitのステージング・Commit・Pushは実施していない。

## Phase 4.4 出品プレビュー・入力検証（2026-11-16 日本時間）

開始HEADは `59546b1`、開始時の作業ツリーはクリーン。
読取専用の候補生成サービス・DTO・IPC・React画面を追加し、商品詳細から開けるようにした。
スキーマVersion 4を維持し、Migration・依存・lockfile・識別子・通常保存先・CSP・権限を変更していない。
設計契約は [ADR 0009](decisions/0009-listing-preview.md) を参照。

### 自動チェック

| コマンド（リポジトリルートから実行） | 結果 |
|---|---|
| `npm run build` | 最終のAPI修正後に成功。TypeScript両設定とVite本番ビルド成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 成功、終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 成功、終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 成功、終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 125 passed / 0 failed / 0 ignored。既存120件と専用5件 |
| `git diff --check` | 成功、終了コード0 |

Rustの最終編集後に上記Rustチェックを実施。以後のソース修正はフロントエンドAPIの取得共有だけで、
その修正後にnpmビルドと実WebView2検証を実施した。
Viteの初回はサンドボックスの子プロセス起動制限でspawn EPERMとなり、同一コマンドを
承認済みの通常権限で実行して成功。アプリの依存や設定による回避はしていない。

専用Rustテストではタイトル出典・決定的な警告順序・ready/warning/blocked・公開説明文の改行・
価格0・内部メモの非公開・有効値だけの抽出・小数と単位の保持・定義名変更と復元・
画像順序と再読込・欠落／危険なパス・商品不在・DB部分読取失敗を確認した。
生成前後の全4テーブルの行、商品更新日時、元画像バイト列の不変とスキーマ4も確認した。
メディアpermit取得中のプレビューは取得エラーとなり、解放後は再取得できる。
既存のメディア競合、リンク／junction拒否、再起動時整合性、Phase 4.1〜4.3の回帰テストも成功。

### 実WebView2（隔離した開発版）

`npm run tauri dev -- --config .verification/phase44/dev-config-2.json` で起動し、
appDirectoriesOverrideで専用データ領域を指定。実IPCで解決した保存先が専用領域であることを
検証スクリプトで照合してから、作成・編集・画像取り込み・削除を行った。通常データは使用していない。
CDPはローカルポート9364を使用。検証後に開発プロセスを停止した。

- 商品詳細からの遷移、タイトル未設定時の管理名候補とtitle_fallbackを含む4警告を確認。
- プレビューに内部メモが含まれず、Numberの001.230000と単位cmが保持されることを確認。
- プレビューから編集し、公開用タイトル・説明文・価格を保存して再表示。説明文のpre-wrapを確認。
- カスタム項目を非表示にした後、再読み込みで値が除外されることを確認。
- 既存のネイティブダイアログと実IPCで2画像を取り込み、readyと実画像デコード成功を確認。
- 並べ替え後の再読み込みで先頭のメイン画像が変わること、商品更新日時に依存しないことを確認。
- 繰り返し取得前後の編集スナップショットが完全一致することを確認。
- DOMのerrorイベントを送ってブラウザー側読込失敗のプレースホルダーを確認し、再読み込みで復帰。
  このケースはイベントによるシミュレーションであり、破損画像の実デコード失敗を再現したものではない。
- 新規に取り込んだ専用画像を検証領域に退避して欠落を再現。blocked、image_unavailable、
  画像ごとのプレースホルダーを確認し、退避した画像を戻してreadyへ復帰。
- 商品詳細への復帰、既存の画像削除IPC、商品削除の確認UIからの確定を確認。
- 追加の専用商品をプレビュー表示中に削除し、再読み込みでproduct_not_foundを表示。
  古いタイトル・準備状況が残らないことを確認。
- 最終の2スクリプトともJavaScript例外0件、CSP違反0件。

初回の実画面検証ではStrictModeの初回Effect再実行による二重取得がmedia_busyとなった。
API層で同一商品の進行中Promiseだけを共有し、完了後は破棄する修正で解消。
新しい専用保存先で最初から再検証して成功した。
Git対象外の `.verification/phase44/result.json` と `reload-result.json` に成功結果、
同ディレクトリの初回失敗記録と検証スクリプトに再現手順を保持する。
開発用watcherにはWebView2キャッシュのEBUSY通知が出たが、実IPCと画面検証は成功した。

### 検査の限界・未実施事項

画像確認は既存の安全なパス検証と通常ファイルのopenまでで、全件SHA-256再計算・全件再デコードはしない。
アプリのメディア変更はpermitで排他するが、同一ユーザーの外部プログラムによる検査後の改変まで保証しない。
DB読取失敗はRust専用テストで確認し、実WebView2の取得失敗表示には商品不在を使用した。
今回の本番CSP下のRelease画面操作、`npm run tauri build`、インストーラー、別PC、電源断は未実施。
Phase 4.5では最新データを再取得し、出力に必要な画像完全性検証を追加する必要がある。

### 変更ファイルとGit

- Rust新規：domain/listing_preview.rs、services/listing_preview_service.rs、commands/listing_preview.rs、listing_preview_tests.rs。
- Rust登録更新：domain/mod.rs、services/mod.rs、commands/mod.rs、lib.rs。
- React/API新規：ListingPreview.tsx、listingPreviewTypes.ts、lib/api/listingPreview.ts。
- 既存UI更新：ProductDetail.tsx、products.css。
- 文書：README.md、architecture.md、verification.md、decisions/0009-listing-preview.md。

計17ファイル。ステージング・commit・pushは行っていない。検証成果物は既存のGit除外を維持。

## Phase 4.3 公開用タイトル・説明文（2026-11-12 日本時間）

開始HEADは `64d25ec`、作業ツリーはクリーン。Migration 0004でスキーマ4へ移行する。
既存0001〜0003、通常保存先・識別子・CSP・Capabilities・Asset Protocol・メディア処理は変更していない。
JSON入力契約のテスト用にserde_jsonをdev-dependenciesへ追加した。既存lock内の版を使い、
Cargo.lockの変更はlisting-pilotの直接依存一覧1行のみ。npm依存・lockfileは変更していない。
設計は [ADR 0008](decisions/0008-listing-content.md)。ステージング・Commit・Pushは行っていない。

### 自動検証（最終ソース編集後）

環境：Windows x64、Node 24.16.0 / npm 11.13.0、Rust / Cargo 1.99.1 stable MSVC。
Rustコマンドは既存キャッシュを使う `--offline` でも成功した。

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 終了コード0、TypeScript・Vite成功。最初のsandbox内実行はspawn EPERM、通常権限で同一コマンドを再実行して成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml --offline` | 終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --offline --all-targets -- -D warnings` | 終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml --offline` | 終了コード0、120 passed / 0 failed / 0 ignored、main/doc-testは各0件 |
| `npm run tauri build` | 終了コード0、通常設定のWindows Release・MSI（3,665,920 bytes）・NSIS（2,630,352 bytes）生成。Rust release 2分38秒 |
| `git diff --check` | 終了コード0 |

配布ビルド内のTypeScript・Viteも成功。既存識別子末尾 `.app` のmacOS向け警告は残るが、
Windows生成は成功した。識別子は変更していない。生成物のSHA-256を専用検証領域へ記録した。

新規10件と既存110件を実行。旧スキーマ用テストのデータ準備は旧形式SQLに変更し、
現行Repositoryが未追加列へアクセスすることを避けた。直接書き込み制約のテストは列名を明示し、
列数不一致だけで成功扱いにならないようにした。

- 移行：0〜3から4、再適用・ファイルDB再接続、5／99拒否、0004の2番目のALTER失敗による
  1番目のALTER・中間スキーマ・user_versionの全体ロールバック、既存商品全列・画像全列・定義・値の保持。
  foreign_key_check空、integrity_checkはok。既存の実画像付きスキーマ2起動試験でも元バイト列を保持。
- 入力：Unicodeスカラー値でタイトル200／201、説明10,000／10,001、前後空白、NULL化、
  trim前の制御文字拒否、CRLF→LF、本文中のタブ・改行・HTML文字列の保持、Unicode非変換。
- 互換性：Serdeで省略・NULL・文字列を区別し、不正JSON型を拒否。旧作成はNULL、旧更新は保持、
  片方だけの更新・解除は他方を保持。管理名変更との独立、内部メモ非転記、再取得・再起動後の保持。
- 保存：公開文章だけでも更新トークンを進める。競合・不正入力は全商品情報を保持。
  カスタム値の途中SQL失敗で基本情報・公開文章・更新日時・全対象値を作成／更新ともロールバック。
- 削除：共通画像fixtureに公開文章を持たせ、既存の画像／値付き削除、失敗・再起動、他商品と共通定義保持、
  非置換隔離・COMMIT不明時の保護を回帰確認。更新日時を変えない2つの公開列の直接変更も古い確認として拒否。

### 実WebView2・実IPC・SQLite

`npm run tauri dev -- --config .verification/phase43/run2/dev-config.json` を使用。
操作前にAppLocalData解決IPCが今回専用の `run2/data` と一致することを確認した。
通常のLocalData、過去Phaseのデータ、既存fixtureは使用・変更・削除していない。
実Rust IPCを使い、スナップショット取得失敗と保存後一覧取得失敗の2ケースのみ検証用fetch応答を注入した。

| 操作 | 観測結果 |
| --- | --- |
| 新規登録・二重送信 | 管理名・内部メモ・公開文章・カスタム値を同時保存。連続submitでも1商品 |
| 詳細・編集 | 公開文章の保存値を表示。説明文のLF・タブ・HTML文字列を保持し、pre-wrapで表示 |
| 管理情報との分離 | 管理名だけ変更しても公開名を保持。内部メモを公開説明へ転記しない。未設定の入力欄は空で詳細は「未設定」 |
| 境界・旧呼び出し | 絵文字201文字を拒否し入力を保持、200文字を保存。省略更新で両方保持、NULLで片方だけ解除、CRLFをLFへ変換 |
| 同時編集・競合 | 公開文章とカスタム値を同時更新。古い商品トークンでは両文章とカスタム値の入力を保持し再保存を停止 |
| 読み込み・再取得失敗 | スナップショット取得失敗では保存不可。保存後の一覧取得失敗では成功と取得エラーを併記し、更新IPCは1回 |
| プロセス再起動 | Tauri終了→同じ専用設定で再起動。商品全フィールド・カスタム値スナップショットが一致。その後両文章の空白入力でNULL化 |
| 画像・商品削除 | 今回生成したPNG2枚をネイティブダイアログで取り込み・表示・並べ替え。公開名変更後の旧削除確認はstale_data。最新のUI確認で商品・画像2枚・カスタム値1件を削除 |
| 保持・整合性 | 他商品・共通定義を保持。隔離先PNG2枚のSHA-256がDBメタデータ・取り込み元と一致。最終DBはschema=4、画像0／値0／定義1、foreign_key_check空、integrity_check=ok |

検証スクリプトのキャンセル後遷移想定を修正し、完了済み操作を保持して後続を再開した。
削除後の画像一覧APIは空配列ではなくproduct_not_foundを返す既存仕様だったため、検証側の期待値を修正し、
後続の保持・隔離検査を実行した。この期待値不一致によるCDP評価例外はアプリの通常操作の例外とは区別する。
後続のresume／restart／deleted検証でJavaScript例外・コンソールエラー・CSP違反は各0件。
Vite監視が専用WebView2のCookie等へEBUSYを出すことはあったが、起動・IPC・保存・再起動は成功した。
結果JSON・画面・専用データはGit対象外の `.verification/phase43/` に保持している。
検証用Tauri／Viteは終了し、Listing Pilotプロセスが残っていないことを確認した。

今回の実操作は開発版WebView2。通常設定の配布EXE操作、本番CSP下の実操作、インストーラー導入・更新・削除、
別PC、電源断・実ディスク故障は未検証。実ユーザーデータに対する移行は実施していない。
利用前には全アプリを停止し、DBとmedia全体を一組でバックアップする。

## Phase 4.2 商品別カスタム値（2026-11-12 日本時間）

開始HEADは `7671e89`、作業ツリーはクリーン。スキーマ3と0001〜0003、依存／lockfile、
通常の保存先・識別子・CSP・Capabilities・Asset Protocol・メディアOSロックは変更していない。
設計は [ADR 0007](decisions/0007-product-attribute-values.md)。Commit・Push・ステージングは行っていない。
テストは専用メモリDB／UUID付き一時DBとmedia、実アプリは新設 `.verification/phase42/` のみを使用。
本番LocalData・既存fixture・過去の検証結果は使用・削除していない。

### 自動検証（最終ソース編集後）

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 終了コード0、TypeScript・Vite成功（Release生成内でも成功） |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了コード0、110 passed / 0 failed / 0 ignored、main/doc-testは各0件 |
| `npm run tauri build -- --no-bundle --config .verification/phase42/run3/dev-config.json` | 終了コード0、保存先のみ隔離したRelease生成、Rust release 2分33秒 |
| `npm run tauri build` | 終了コード0、通常設定のWindows Release・MSI・NSIS生成、Rust release 2分21秒 |
| `git diff --check` | 終了コード0 |

既存98件を維持し、値7件・商品削除5件を追加した。Phase 4.1の「値のFKで商品削除を拒否」テストは、
今回の値削除対応に合わせて値DELETEへのABORT障害注入へ変更し、商品・画像・非表示値・画像バイト列の
ロールバックと再起動後保持の検証を維持している。画像単体削除などの回帰テストも成功。

- 値：2,000 Unicode文字境界、trim、改行・制御文字、整数／小数／負数／0、桁数境界、不正形式、
  先頭ゼロ・小数末尾ゼロ・精度、未設定と0、ファイルDB再接続。
- 保存：商品と複数値の作成／更新、値だけの更新による商品トークン更新、対象だけの空欄削除、
  未指定・非表示値の保持、定義変更／非表示／復元、商品／定義競合、重複／不在／対象外ID、
  値の途中SQL失敗で作成・更新全体をロールバック、旧APIと空編集対象で既存値を保持。
- 削除：値0件、値あり画像0／2枚、非表示値、他商品・共通定義の保持、商品トークンを変えない
  直接SQLによる値追加／変更／削除も検出、定義名変更だけは削除可能、値削除後の失敗で全件復元、
  COMMIT不明時の全値照合・孤立値参照・不明状態での画像保持。

### 実WebView2・実IPC・SQLite

`npm run tauri dev -- --config .verification/phase42/run3/dev-config.json` で起動し、
AppLocalData解決IPCが `run3/data` と一致することを操作前に確認した。
アプリコードは通常版と共通。障害注入の2ケースを除き、実Rust IPCを使用している。
専用Releaseでも同じ保存先を使用し、本番CSPの応答にunsafe-inline／unsafe-evalがないことを確認した。

| 操作 | 観測結果 |
| --- | --- |
| 新規登録・詳細表示 | Text／Number／単位を表示。Textの00123、負数と小数の表記を保持。2回連続submitで商品1件 |
| 編集・空欄 | Numberの0保存と初期表示、不正な1e3の拒否、空白へ戻した対象値だけを削除 |
| 非表示・復元 | 実IPCで定義を非表示にし、通常フォームから除外。別項目保存後も陶器を保持、復元後に再表示 |
| 定義競合 | フォーム開始後の名前変更を拒否し、入力を保持・保存ボタン停止。新規登録の定義競合も商品を作成しない |
| 商品競合 | 別の実IPC更新後の保存を拒否し、入力した商品名を保持 |
| 読込失敗 | get_product_edit_snapshotのエラー応答をfetchで注入し、fieldsetと保存を停止。空値として保存しない |
| 保存後の取得失敗 | 実更新後のlist_products応答だけをエラー注入。保存成功通知と取得エラーを表示、更新IPCは1回。復旧後の一覧再取得で正常反映 |
| 旧API | attributeValuesを省略した実update_product後も全値が一致 |
| 画像 | 新規32×24 PNG2枚をRustネイティブダイアログで選択。表示・並べ替え・削除キャンセル／確定、SHA-256一致。商品スナップショットは不変 |
| 開発版再起動 | 商品・定義・全値が完全一致し、画像の順序・表示も維持 |
| Release起動・削除 | 高精度文字列と単位を表示。値3件の確認／キャンセル／削除に成功、他商品の値・共通定義3件を保持 |
| Releaseの画像保護 | 商品削除後の隔離画像を含むPNG2枚のSHA-256が元の2枚と一致 |
| Release二重起動 | 同じ保存先・同じWebView2引数の2個目で項目管理4 API、商品一覧・編集snapshot・削除previewの計7 APIがmedia_busy |

最終停止後の専用DBはschema=3、integrity_check=ok、foreign_key_check=0行。
対象商品と画像登録は削除済み、他商品1件・その値1件・共通定義3件を保持。
`images` / `restart` / `release` 区間でJavaScript例外・コンソールエラー・CSP違反0件。
値の操作区間も全操作assertionとJS例外／コンソールエラー0件の確認まで通過したが、初回のCSP監視変数が
未初期化で最終集計assertionが失敗した。監視変数を初期化後、後続3区間は全assertion成功。
値操作区間全体のCSPイベント計測成功とは主張しない。

初回Viteはsandboxのspawn EPERM。同じコマンドを制限外で再実行して成功し、コードや権限は緩めていない。
追加テストの初回はPowerShellパイプの文字コードを修正して再実行した。
UIスクリプトはキャンセル後の戻り先、Tauriの書換え不可invokeへの障害注入、CSP監視初期化を修正した。
各試行の記録を保持し、新設run3で上記操作を実行した。WebViewキャッシュへのwatcher EBUSY警告は継続。
二重起動の初回は異なるdebug引数でWebView2が応答せず、同じ引数で再検証して成功した。
通常終了要求後も残った当該検証プロセスだけは絶対パスとPIDを照合して終了した。
検証中の画像・DBの完全削除や再初期化は行っていない。

通常設定のReleaseは生成のみで起動していない。検証用プロセスはすべて終了した。
通常EXE・MSI・NSIS・隔離ReleaseのサイズとSHA-256は `run3/build-artifacts.json` に保存した。
最終Git状態は変更19ファイル・新規未追跡5ファイル。ステージ済み変更はない。

ローカル証跡：`.verification/phase42/run3/` の `smoke.mjs`、`blocked.mjs`、`*-result.json`、
`*-data-check.json`、`detail.png`、`final-release.png`。恒久的CI UIテストではない。
インストーラーの導入・上書き更新・アンインストール、別PC、電源断・実ディスク故障は未検証。

## Phase 4.1 共通カスタム商品項目（2026-11-12 日本時間）

開始時は `main / a3080010222cc2082bde773e22c6500d738a1eb7`、作業ツリーはクリーン。
共通項目の定義管理だけを追加。商品別の値入力・物理削除・型/単位の変更APIは公開していない。
設計は [ADR 0006](decisions/0006-custom-product-attributes.md)、DDLは新規0003を参照。
本番LocalData・既存バックアップ・既存fixtureは使用・変更せず、UUID付き一時DB/mediaと
新設 `.verification/phase41/` の専用保存先を使用した。Commit・Push・ブランチ切り替えは実施していない。

### 最終自動検証

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 終了コード0、TypeScript・Vite成功（Releaseビルド内でも成功） |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了コード0、98 passed / 0 failed / 0 ignored、main/doc-testは各0件 |
| `npm run tauri build` | 終了コード0、Windows Release・MSI・NSIS生成（Rust release 2分39秒） |
| `npm run tauri build -- --no-bundle --config .verification/phase41/run2/dev-config.json` | 終了コード0、保存先だけを隔離した専用Release生成（2分17秒） |
| `git diff --check` | 最終結果は終了コード0 |

既存86件を削除・無効化せず、新規12件を追加した。既存テストの期待スキーマを3、未知スキーマを4へ更新し、
移行前Version 2の準備は既存0002 SQLを直接適用するよう修正した。

| 追加テスト | 確認内容 |
| --- | --- |
| `migrate_zero_one_two_to_three_preserves_every_product_and_image_column` | 0/1/2→3、商品全カラム・画像全メタデータ/参照保持、integrity/FK検査 |
| `migration_three_failure_rolls_back_all_intermediate_schema_and_existing_data` | 0003途中のテーブル名衝突でDDL・user_versionを戻し、既存商品とsentinelを保持 |
| `version_three_reapply_restart_and_future_version_rejection_preserve_data` | 3再適用・ファイルDB再接続、定義/単位/アーカイブ/高精度文字列値保持、4/99拒否 |
| `create_text_number_optional_units_unicode_and_binary_names` | Text/Number、単位あり/なし/空白、UUID v4・UTC、trim、Unicode文字数、BINARY区別 |
| `reject_invalid_names_units_types_and_text_units_without_creating_rows` | 空白名・上限超過・制御文字/改行・未知の型・Text単位を拒否 |
| `rename_duplicates_archive_restore_and_stale_updates_preserve_identity_and_values` | UUID/作成日時/型/単位/値保持、通常/非表示重複、復元、古いトークン、不在 |
| `conditional_repository_updates_reject_changes_after_service_read` | 読取後に別更新があっても条件付きSQLで名前変更/状態変更を拒否、不在と区別 |
| `timestamp_advances_at_same_time_and_backwards_clock_for_all_operations` | 同時刻/逆行を決定的に注入、名前変更/非表示/復元で最低1ns進む |
| `list_order_uses_archive_state_created_at_and_uuid_tiebreaker` | 通常先、作成日時が同じ場合のUUID順 |
| `database_constraints_reject_invalid_definitions_orphan_duplicate_and_blank_values` | DB型/単位/名前制約、孤立値・複合PK重複・Unicode空白値を拒否、参照先DELETEをRESTRICT |
| `schema_two_startup_migration_keeps_registered_images_and_original_bytes` | 実PNG取り込み済みVersion 2のstartup移行、商品/画像/バイト列保持、誤回収0件 |
| `attribute_values_restrict_product_delete_and_preserve_product_images_bytes_after_restart` | 値を直接用意してアーカイブ、実商品削除サービスの失敗で商品/全画像/値を戻し、ファイル非移動・再起動保持 |

新規テストは専用ファイルDBを使用。将来用の値を作るSQLやテストスキーマの準備はtestモジュール内だけにある。
商品登録・編集・SKU重複・楽観ロック、画像取り込み・並べ替え・削除、隔離、起動時回収、
実別プロセスのロック保護を含む既存回帰テストもすべて成功した。

### Windows実WebView2

`npm run tauri dev -- --config .verification/phase41/run2/dev-config.json` で起動。
保存先を `.verification/phase41/run2/data` に固定し、実IPCのAppLocalData解決結果が一致してから操作した。
React→API層→Rust IPC→SQLite→UIの全経路を使い、APIモックは使用していない。

| 操作 | 観測結果 |
| --- | --- |
| 項目作成 | TextとNumberを選択、Numberのときだけ単位入力表示、単位あり/なし、trim保存 |
| 重複 | 作成・名前変更・非表示の名前再利用を拒否し、日本語エラー表示 |
| 名前変更 | UUID/作成日時保持、一覧反映 |
| 競合 | 編集中に実IPCで別更新後、保存を拒否し入力保持・再読み込み案内。保存ボタン停止、更新の自動再送なし |
| 非表示・復元 | 確認キャンセルで更新IPC 0回、確定で通常一覧から除外、表示切替・復元成功 |
| 二重送信 | 同一イベントループで作成フォームを2回送信して実IPC 1回、非表示確定2回クリックも実IPC 1回 |
| 既存商品 | 登録・編集・一覧、SKU重複、古い更新トークンの拒否 |
| 既存画像 | 今回生成の32×24 PNG 2枚をRustネイティブダイアログで選択、実表示、バイト列/SHA-256一致、並べ替え、削除キャンセルと確定隔離 |
| 商品削除確認 | キャンセルで商品・残存画像を保持 |
| Releaseで再起動 | 項目の名前/型/単位/非表示状態、商品全フィールド、画像順序/メタデータ/バイト列を保持。通常一覧では非表示項目を除外、切替後は非表示状態を表示 |
| Releaseで更新・削除 | 本番CSP下で項目名変更、非表示確認キャンセル、商品削除が成功。項目定義は3件とも保持 |
| Release二重起動 | 実アプリ2つを起動し、2つ目の項目管理全4 APIと商品一覧がmedia_busyで拒否された |
| 隔離DB検査 | スキーマ3、integrity_check=ok、foreign_key_check=0行、画像SHA-256一致、値テーブル0件 |

成功した `attributes` / `images` / `restart` / `release` 区間でJS例外・コンソールエラー・CSP違反0件。
専用ReleaseのHTML応答で本番CSPとscriptハッシュを確認し、unsafe-inline/unsafe-evalがないことを検証した。
全検証アプリ終了後のDBもスキーマ3・integrity_check=ok・外部キー違反0行だった。
UIで削除した検証商品/画像レコードは0件、項目は3件保持し、隔離されたPNG2枚のハッシュは元画像2枚と一致した。
二重起動中のGet-FileHashはSQLiteの共有条件で読み取り拒否されたため、当該区間のDBファイルハッシュ一致は主張しない。
ローカル資料は `.verification/phase41/run2/smoke.mjs`、`*-result.json`、スクリーンショット、`data-check.py`。
これらはGit対象外であり、CIの恒久的なUIテストではない。

初回Viteはサンドボックスの子プロセス制限で `spawn EPERM`。同じビルドを通常権限で再実行して成功した。
初回の追加Rust再起動テストはfixtureのDB名不一致を修正して成功した。
UIスクリプトのCSP監視変数初期化漏れと、既存画像ボタンの古いセレクターを修正し再検証した。
最初の隔離保存先と失敗記録は保持した。アプリの仕様・権限を緩める回避変更は行っていない。
開発watcherにはWebViewキャッシュのEBUSY警告が出たが、上記成功区間の動作・データ検査は成功した。

### Windows Release生成物

通常設定のEXEは起動せず、専用ビルドの前に `.verification/phase41/run2/normal-release.exe` へ保持した。
生成物のSHA-256は同フォルダーの `build-artifacts.json` に記録した。

| 通常設定の生成物 | サイズ（bytes） |
| --- | ---: |
| `src-tauri/target/release/listing-pilot.exe` | 7,294,976 |
| `src-tauri/target/release/bundle/msi/Listing Pilot_0.1.0_x64_en-US.msi` | 3,645,440 |
| `src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.0_x64-setup.exe` | 2,615,760 |

通常EXEのSHA-256は `B060BB19E91D019854085717F4C9BDF0DCB7BD7183389936DF275D2E5E7CB6F3`。
既存識別子末尾 `.app` に関するmacOS向け警告は継続している。識別子は変更していない。
MSI/NSISは生成のみで、インストール成功の根拠にはしていない。

専用Releaseは `.verification/phase41/run2/isolated-release.exe`、SHA-256は
`B7253CAA3A113A3A3228C096F22F9037E8E7B42742F964875294D6F413094AD8`。
通常Releaseと同じアプリコード・本番CSP・識別子を使い、AppLocalDataだけを専用パスに固定した。
検証後は全専用プロセスを通常終了し、`target/release/listing-pilot.exe` は通常版へ戻してハッシュ一致を確認した。

### 変更範囲とデータ保護

- Rust追加：`domain/attribute.rs`、`services/attribute_service.rs`、`infrastructure/db/attribute_repository.rs`、`commands/attribute.rs`、`attribute_tests.rs`。
- Rust変更：各mod.rs、lib.rsの登録、error.rs、migrations.rs、tests.rs、image_tests.rs。
- UI追加：`features/attributes/types.ts`、`attributes.css`、`components/AttributeManager.tsx`、`lib/api/attributes.ts`。変更：App.tsx、request.ts。
- DB追加：`migrations/0003_create_custom_attributes.sql`。文書：README、architecture、data-model、backup-and-restore、verification、ADR 0006。

0001/0002、依存/lockfile、商品編集・削除・画像/起動時サービス、media_storage、識別子・通常保存先・CSP・Capabilitiesは変更していない。
定義管理にDELETEはなく、非表示後も定義と参照値を保持する。既存の商品削除は空の値テーブルで成功し、
値がある場合はRESTRICTで中止することを確認した。
**移行テストでは既存の商品全カラム・画像メタデータ・参照・画像バイト列を保持した。** 本番データ上の移行は実行していない。
移行前の停止中DB・media一組のバックアップが必要で、旧版はスキーマ3を開けない。

インストーラーの導入・上書き更新・アンインストール、別PC、電源断・実ディスク故障は未検証。
商品別値入力と、それに対応する商品削除の確認/値削除はPhase 4.2で実装する。

最終Git状態は変更16ファイル、新規未追跡11ファイル（合計27）。削除・ステージ済み変更はない。
`git status --short --untracked-files=all` の全出力は `.verification/phase41/run2/git-status.txt`。
新規ファイルもレビューし、依存追加・定義の危険なDELETE・更新トークン照合漏れがないことを確認した。

## Phase 3.5 監査に基づく品質改善（2026-11-10 日本時間）

開始は `main / b70c86e`、作業ツリーはクリーン、ローカル `origin/main` との差分0。
指摘を現行コード・失敗する再現テストで確認し、M-1/M-4/M-2/L-9と関連するL-8を修正した。
指摘ごとの採否・理由・残存リスクは [監査対応記録](qa-hardening-phase35.md) を参照。
本番LocalDataにはアクセスせず、既存の検証資料・fixtureも変更していない。
新設 `.verification/phase35/` とUUID付き一時DB・mediaのみを使用した。Commit・Push・ブランチ切り替えは行っていない。

### 自動検証

| コマンド | 最終結果 |
| --- | --- |
| `npm run build` | 終了コード0、TypeScript・Vite成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了コード0、86 passed / 0 failed / 0 ignored。main・doc-testは各0件 |
| `npm run tauri build` | 終了コード0、通常設定のWindows Release・MSI・NSIS生成 |
| `git diff --check` | 終了コード0 |

既存71件は削除・無効化せず、更新APIの新しい必須引数をテストへ反映した。
追加15件には別プロセス試験用の子テスト1件を含む。通常実行ではその子テストは何も作らず終了し、親テストが実別プロセスとして呼び出す。
最終Rust結果は `.verification/phase35/final-rust-tests.log`。

修正前は古いフォームの更新拒否、originals直下の通常ファイル、Windows拡張子大小文字差の再現テスト3件が失敗。
修正後は同じ条件で成功した。条件付きSQLの直接競合、商品・日時保持、SKU重複、商品削除スナップショット、
時計逆行、通常/平坦/入れ子ディレクトリ、junction、共有拒否、回収先衝突・途中失敗・再起動を追加確認した。
回収先衝突は本番と同じ移動処理に固定名を注入し、上書き禁止と全バイト列保持を確認した。
並べ替えの実COMMIT失敗はdeferred FK違反で再現し、順序の全ロールバックとファイル保持を確認した。
実際のCOMMIT成功後だけエラーを返す障害は未再現。

Windowsの実別プロセスがOSロックを保持中、もう一方のstartup serviceが `media_busy` で接続公開を拒否し、
商品登録/編集/削除へDB操作を通さないこと、DBバイト列が変わらないことを検証した。
通常終了と子プロセス強制終了の両方で、次の起動が成功し、既存商品・画像参照・SHA-256を保持した。
これは実アプリの任意の書き込み瞬間や電源断を強制した試験ではない。

初回Viteはサンドボックスの `spawn EPERM` で失敗し、同じコマンドを通常権限で再実行して成功。
初回Clippyのテストモジュール配置エラーは末尾へ移動して解消し、その後のfmt/check/clippy/testを再実行した。
依存関係・lockfile・既存マイグレーション・CSP・Capabilities・識別子は変更していない。

### Windows実WebView2：開発版と専用Release

開発版は `npm run tauri dev -- --config .verification/phase35/dev-config.json`。
`appDirectoriesOverride` で `.verification/phase35/data` へ隔離し、起動後も実IPCの
`plugin:path|resolve_directory`（AppLocalData）で一致を確認してから操作した。
実React UI・Rust IPC・ネイティブファイル選択を使用し、モックは使っていない。
試験スクリプトとJSON・スクリーンショットは同フォルダーの `smoke.mjs`、`*-result.json`、`*.png` に保持する。

| シナリオ | 実測結果 |
| --- | --- |
| 商品登録・通常編集 | フォーム入力で登録・編集成功、保存後一覧に反映 |
| 古いフォーム | 編集中に実IPCで別更新後、フォーム保存が競合案内を表示。入力した商品名・メモを保持し、最新DB商品と日時は変化なし |
| 二重送信 | 同一イベントループ内の `requestSubmit()` 2回に対し、実IPCのfetchを計測して `update_product` は1回 |
| 画像取り込み・表示 | 今回生成した32×24 PNG 2枚をネイティブダイアログで選択。管理バイト列・SHA-256が選択元と一致、実画像表示成功 |
| 並べ替え・画像削除 | 実UIで順序変更、画像削除確認のキャンセル保持、確定削除、残存画像の表示を確認 |
| 商品削除 | キャンセル保持、確認後に商品更新した場合の拒否、最新内容での再確認・確定削除に成功 |
| 二重起動 | 開発版と専用Releaseでそれぞれ実アプリ2つを起動。2つ目の商品登録/編集/削除・画像取り込み/並べ替え/削除はすべて `media_busy`。UIに再起動案内 |
| 起動時回収 | 停止中に未参照PNG、originals直下の通常ファイル、未完了stagingを作成。再起動で3件を回収し、それぞれのハッシュ一致 |
| 大小文字差 | 同時に登録画像の拡張子を `.PNG` に変更。小文字DBパスで引き続き表示でき、再起動で隔離されずDB・SHA-256保持 |
| SQLite検査 | 専用DBの `integrity_check = ok`、`foreign_key_check = 0行`、参照画像ファイル・ハッシュ一致 |
| バックアップ/復元 | 全アプリ停止中にDB・mediaをコピーし一覧/全SHA-256一致。別領域へ復元しSQLite検査後、専用Releaseで商品・画像・順序を表示。通常終了・再起動後も一致 |
| 実行時エラー | 成功した各観測区間でJS例外・コンソールエラー・CSP違反なし。専用ReleaseのHTML応答で本番CSPとscriptハッシュを確認し、unsafe-inline/unsafe-evalなし |
| Release診断 | 二重起動する専用Releaseを標準エラーのリダイレクト付きで起動し `Dataset initialization failed: MediaBusy` を取得。通常起動の永続ログはない |

最初の二重起動試験は同じWebViewプロファイルに異なるデバッグポート引数を渡したため、
WebView2が `0x8007139F` でウィンドウ作成を拒否した。ブラウザー引数を揃えた再試験で2つの実ウィンドウと上記API拒否を確認した。
ローカルUIスクリプトには計測対象の読み取り専用プロパティと複数検証商品の扱いに誤りがあり、
修正して成功結果を取得した。アプリ側の回避修正は行っていない。
開発版のwatcherがEBWebViewファイルに `EBUSY` を記録したが、上記成功区間の実IPC・画面動作に影響はなかった。
これらはアプリのデータ保護テストの失敗と区別する。

### Windows Release生成物

通常設定の `npm run tauri build` と、保存先だけを変える次の専用Releaseビルドが成功した。

```powershell
npm run tauri build -- --no-bundle --config .verification/phase35/portable-config.json
```

専用設定は `app.appDirectoriesOverride: "./data"` のみ。専用Releaseは
`.verification/phase35/restored-app/listing-pilot.exe`、保存先は隣の `data/`。
本番CSP・アプリコード・識別子は通常Releaseと同じ。専用ビルドは2分29秒、SHA-256は
`06FA4EB6A0F1132C396374561ED7489E3DDD99D801060E0BF8EBEDBD4334A2D4`。

| 通常設定の生成物 | サイズ（bytes） |
| --- | ---: |
| `src-tauri/target/release/listing-pilot.exe` | 7,207,936 |
| `src-tauri/target/release/bundle/msi/Listing Pilot_0.1.0_x64_en-US.msi` | 3,612,672 |
| `src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.0_x64-setup.exe` | 2,597,399 |

通常EXEのSHA-256は `7E28777DB9DDD309849B830CC52B70134F4EAB2ADCBAB29BB94E64F4E80D4AAC`。
通常EXEは実行せず、専用ビルド後のtargetへ通常EXEを戻してハッシュを照合した。
全生成物のハッシュは `.verification/phase35/build-artifacts.json`、ビルドログは
`release-build.log` / `isolated-release-build.log`。既存識別子末尾 `.app` に関するmacOS向け警告は継続。
MSI/NSISは生成のみで、導入・アンインストール・上書き更新・署名・別PC試験は行っていない。

### 残る制約

symlink実作成はWindowsの権限不足（WinError 1314）でできず、junction/reparse試験と区別する。
実ACL拒否は未実施（共有拒否は実施）。大小文字区別を有効にしたWindowsディレクトリ、別ファイルシステム、
同一ユーザーの別プログラムによる検査直後のパス差し替え、電源断・実ディスク故障は未検証。
旧版と新版の混在起動では旧版のDB書き込みを制御できないため、旧版を完全終了して使用する。
UIの自動検証スクリプトはローカル資料であり、CIの恒久的なフロントエンドテスト基盤は追加していない。
追加調査は上記の確認範囲に限定し、未解決条件を [監査対応記録](qa-hardening-phase35.md) に分類した。

## Phase 3.4 統合検証・リリース品質確認（2026-11-10 日本時間）

開始時は `main` / HEAD `d8dacf2`、直前は `fbd1e0c`、作業ツリーはクリーン。
既存71件の回帰とWindows実WebView2の操作・データ保護を検証した。
アプリ側の修正を必要とする不具合は見つからず、変更はREADME、本書、新規 `backup-and-restore.md` のみ。
ソース、依存・lockfile、マイグレーション、アイコン、識別子、通常の保存先、CSP、Capabilitiesは変更していない。
Commit・Push・ブランチ切り替え・インストーラー実行は行っていない。

### 実施済み：自動検証

| コマンド | 結果 |
| --- | --- |
| `npm run build` | 終了コード0。TypeScriptとVite本番ビルド成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了コード0、71 passed / 0 failed / 0 ignored。main・doc-testは各0件 |
| `npm run tauri build` | 終了コード0、通常設定でWindows Release・MSI・NSIS生成成功 |
| `git diff --check` | 終了コード0 |

最終再実行のコマンド別終了コードとログは `.verification/phase34/final-check-results.json`、`final-check-*.log` に保存した。

既存71件はそのまま維持。JPEG・PNG・WebP、非対応・破損画像、20MiB・4,000万画素・1回20枚の制限、
商品内重複・別商品への独立取り込み、20枚を超える商品の並べ替えを含む。
SQL途中失敗・実COMMIT失敗時のロールバック、COMMIT結果不明の解決分岐、欠落・Windows共有ロック、
非置換配置・隔離先衝突、junction/reparse point、不正パス、共通permitによる競合拒否、
OSロック下のDB参照再取得と回収失敗時の保持も成功した。通常のsymlink作成そのものの実機追加試験は行っていない。
テストは専用一時DB・メディアを使い、本番LocalDataへアクセスしていない。

### 実施済み：Windows Release生成

| 通常設定の生成物（リポジトリルートからの相対パス） | サイズ（bytes） |
| --- | ---: |
| `src-tauri/target/release/listing-pilot.exe` | 7,201,792 |
| `src-tauri/target/release/bundle/msi/Listing Pilot_0.1.0_x64_en-US.msi` | 3,608,576 |
| `src-tauri/target/release/bundle/nsis/Listing Pilot_0.1.0_x64-setup.exe` | 2,594,114 |

通常ReleaseのRustビルドは2分26秒で成功。既存識別子の末尾 `.app` に関するmacOS向け警告は継続。
アプリ名 `Listing Pilot`、識別子 `com.listingpilot.app`、アイコンと配布設定は維持。
通常EXEは実行せず `.verification/phase34/normal-release.exe` に保存し、検証後に元のtarget配置へコピーして
生成直後のSHA-256 `83AF3076E719A93D63220A57C183805F89429BBF7172CEA594FFCEA082820ED3` と照合した。
MSI・NSISのサイズとSHA-256は `.verification/phase34/build-artifacts.json` に保持する。
生成成功はインストール成功を意味しない。

### 実施済み・制約あり：隔離Releaseと開発版の実操作

検証PCはWindows build 26200、WebView2 Runtime 157.0.4341.42。
本番LocalDataと既存の検証画像・成果物は使わず、新設 `.verification/phase34/` だけを使用した。
画像は今回生成した32×24 PNG 5枚とJPEG 1枚。実画面ではPNG・JPEGを確認し、WebP・上限・非対応形式はRustテストで確認した。

通常の設定ファイルを変更せず、次で保存先だけを上書きした別のReleaseを生成した。

```powershell
npm run tauri build -- --no-bundle --config .verification/phase34/portable-config.json
```

設定内容は `app.appDirectoriesOverride: "./data"` のみ。公式同梱スキーマ・Tauri PathResolverの実装で、
実行ファイル隣の専用dataとAsset Protocolの `$APPLOCALDATA` に適用されることを事前確認した。
本番CSP・Capabilities・識別子・アプリコードは通常Releaseと同じ。ビルドは終了コード0、2分19秒で成功。
この専用ReleaseのSHA-256は `C856B9CA720430A8A335CC45C3578160C0CC861C0F4D44E7E4BFB7CD358D658F`。
起動後も `plugin:path|resolve_directory`（AppLocalData）を最初に実照会し、期待した専用パスとの一致を確認してから商品操作した。

以下のルートはすべて `C:\Users\owner\Desktop\sm-dev\listing-pilot\.verification\phase34\` の下。
DBは各dataルート直下の `listing-pilot.sqlite3`、メディアは同じルートの `media/`、WebViewプロファイルも同じルート内。

| 用途 | 実際の実行ファイル | dataルート |
| --- | --- | --- |
| Release統合操作・再起動 | `listing-pilot.exe` | `data/` |
| バックアップ復元後の起動・商品削除再検証 | `restored-app/listing-pilot.exe`（同一バイナリ） | `restored-app/data/` |
| 削除確定・ファイル残存の回復試験 | `recovery-app/listing-pilot.exe`（同一バイナリ） | `recovery-app/data/` |
| 開発版疎通 | リポジトリの `src-tauri/target/debug/listing-pilot.exe` | `dev-data/`（dev-config.jsonの絶対パス） |

開発版は `npm run tauri dev -- --config .verification/phase34/dev-config.json` で起動。
実WebView2にlocalhost限定CDPで接続し、Reactフォーム・ボタン、Rust IPC、Rustネイティブ画像選択を使用した。
異常入力・古いスナップショットの注入や結果照会にはCDPから実IPCも呼び出した。Tauriモックは使っていない。

| シナリオ | 観測結果 |
| --- | --- |
| 商品A・B登録、A編集 | AはUIフォームから登録・名称/ブランド/価格/メモを変更。詳細取得値と保存結果が一致。Bは実IPC登録 |
| 異常入力 | 空SKU・空商品名は `required_field`、負の価格は `invalid_price`、SKU重複は `duplicate_sku`、不在IDは `product_not_found` |
| 画像取り込み | AにPNG 3枚・JPEG 1枚、Bに同一内容PNG 1枚を取り込み実表示。管理バイト列・SHA-256が選択元と一致 |
| 並べ替え | UIで先頭を移動、全ID順と0始まり連番を照合。先頭/末尾の移動無効化、古い順序の `stale_data`、追加画像の末尾配置を確認 |
| 画像1枚削除 | 確認からキャンセルでAPI結果不変。先頭を削除し残存4枚・連番・実表示を確認。deletedのバイト一致、復元起動時に削除画像が戻らないことを確認 |
| 商品削除 | 商品名・SKU・4枚の確認、キャンセル保持、確認後の商品更新による拒否と再確認案内、最新内容での確定削除を確認 |
| 画像0枚の商品C | UI確認に0枚表示、削除後一覧から消えることを確認。不必要な移動を行わない内部経路は既存テストで確認 |
| 他商品の保持・再起動 | Bの商品・画像メタデータが不変。通常終了後の専用Release再起動でA・Cが戻らず、Bの画像が表示される |
| 古い画像構成での商品削除 | 画像追加・削除・順序変更による拒否は既存Rustテストで成功。実UIでの注入は商品更新ケースを確認 |
| 開発版 | 別の空DBへ商品登録、一覧再取得、詳細表示の実IPC成功 |
| 実行時エラー | 成功した各観測区間でJavaScript例外・CSP違反なし。通常操作で新規コンソールエラーなし |

メイン画像は先頭に連動する実装と表示を確認。UI保存中の厳密な二重クリックIPC回数計測は未実施。
Rustの共通permit・不正入力拒否・ロールバックの検証で代替しており、すべてのタイミング競合を実UIで網羅したとはしない。

### 実施済み：SQLite・メディア整合性と回復

アプリ停止中、専用DBをPython sqlite3の読み取り専用URIで開き、修復書き込みを行わず検査した。
バックアップ前・復元直後は2商品/登録画像5枚、最終のsource・restored・recoveryは各1商品/登録画像1枚。
全DBで `integrity_check = ok`、`foreign_key_check = 0行`。
孤立画像レコード0、登録画像の欠落0、UUID由来相対パスとMIME拡張子、0始まり連番、
ファイルサイズ・SHA-256・originalsの全参照を照合した。最終の各領域には隔離画像5枚を保持し、選択元ともハッシュ一致。

追加回復試験ではバックアップの新しいコピーだけで商品A・画像レコードをトランザクション削除し、
管理画像4枚はoriginalsに残した。専用Releaseの次回起動で
`Quarantined 4 incomplete or orphaned media files` を観測し、4枚の `.recovered` と元ハッシュの一致、
Bの参照中画像の実表示・ハッシュ不変、既存deleted画像の保持を確認した。
これはDB確定後・ファイル移動前の状態再現であり、プロセスの当該瞬間の強制終了・電源断・実ディスク故障の再現ではない。

### 実施済み：本番CSPとセキュリティ

Releaseの実HTMLレスポンスヘッダーで制限的CSPとTauriによるscriptハッシュ追加を確認した。
`unsafe-inline`・`unsafe-eval` はなく、実画像はAsset Protocolから表示できる。
実WebView2で専用DB・生成した選択元PNG・deleted画像の3種類をAsset画像URLとして要求し、すべてHTTP 403で拒否。
これらの意図した拒否時のみネットワークエラー3件を観測し、正常操作のエラーと分けて記録した。
staging・その他quarantineは設定のscope外であることを静的確認し、個別の実URL要求は未実施。

Capabilitiesは `core:default` のみ。Reactのinvokeは `src/lib/api/` に閉じ、
公開Commandsに任意SQL・任意パス読取/移動はない。外部リンク起動・外部API通信の実装は見当たらない。
UUID/MIMEからのパス検証、全親要素のリンク/reparse検査、Windows `MoveFileExW(flags=0)`、
OSロック保持、DB確認→OSロック→ロック下再照会はコードと既存テストで確認した。
包括的な通信監視・侵入試験は行っていない。

### 実施済み・制約あり：バックアップと復元

専用Releaseを `CloseMainWindow` → `WaitForExit` で正常終了し、2商品/登録画像5枚/削除済み隔離画像1枚の時点で
DB・media内の全ファイルを新設 `backup/` へコピーした。コピー元・バックアップ・`restored-app/data/` の
全8ファイル（DB、lock、originals 5枚、deleted 1枚）の相対パスとSHA-256が一致。
コピー元を変更していないことも照合した。バックアップの `journal_mode = delete`、`user_version = 2` を読み取り確認し、
今回のバックアップにwal/shm/journalの残存はなかった。

別領域にコピーした同一Releaseを起動し、全商品フィールド、画像順、Aの4枚/Bの1枚の実表示とバイト一致、
削除済み画像の非再登録を確認。コピー直後のDB内容と整合性も一致した。
その後は復元側で商品削除の再検証を行ったため、復元領域の最終状態はバックアップ取得時とは異なる。
元の `backup/` は保持している。

ユーザー向け手順は [backup-and-restore.md](backup-and-restore.md) に追加。
WAL・hot journalが残る異常終了データからの復元、異なるアプリ版への移行、本番保存先への復元は未実施。

### 失敗・環境制約と対処

- 初回Viteはサンドボックスの子プロセス起動制限による `spawn EPERM` で失敗。
  実行制限外で同一 `npm run build` を再実行して終了コード0。コード・依存の変更は不要だった。
- 最初のPowerShellプロファイルは既存fnmのmultishell作成がアクセス拒否。以後 `-NoProfile` 相当で実行し、設定は変更していない。
- 新規ローカルpickスクリプトのUTF-8 BOM不足による構文エラーを修正。
  ネイティブ画像選択直後の取り込み完了待機不足も修正し、再実行成功。アプリ不具合とは判定していない。
- 初回の再起動接続はCDP開始前の `ECONNREFUSED`。起動待機を追加。
  復元初回はWebView初期化前のabout:blankをハーネスが再読込して空画面で停止したため、
  当該専用プロセスだけを停止し、アプリURL待機に修正して再起動・復元検証成功。
- 商品削除の初回スクリプトは、直前の意図的403の履歴をCDPが再送し、最後のconsole検査が失敗。
  操作・データのassertは成功していたが全体成功とは扱わず、ログ履歴をクリアして復元側でシナリオ全体を再実行し成功。
- 開発版Vite watcherは専用WebView2のCookies/Cookies-journalで `EBUSY` を報告。
  開発版の登録・一覧・詳細操作は成功したが、watcher問題が解消したとはしていない。ReleaseではViteを使わない。

スクリプト・成功/失敗JSON・画面・専用DB・生成画像・バックアップ・復元・回復試験データは
Git対象外の `.verification/phase34/` に保持した。既存の成果物は変更していない。
検証アプリ・開発サーバー・デバッグ接続は終了した。

### 未実施・正式リリース前の残課題

- 通常設定の配布EXEそのものの実操作、署名・配布先PCでの信頼確認。
- MSI/NSISの新規導入、初回起動、アンインストール、導入後/アンインストール後のデータ保持、上書き更新・旧版アップグレード。
  専用VM・Windows Sandbox・隔離ユーザーを今回用意しておらず、実PCの既存インストール状態は変更していない。
- 別PC、Windows 10/11比較、実電源断・ディスク故障、WAL残存時復元、全競合タイミングの実UI検証。
- 同一ユーザーの別プロセスによる検査直後の親ディレクトリ差し替えへの既存制限は残る。

判定は「自動回帰・配布物生成・保存先だけを隔離したReleaseの主要実操作とバックアップ復元は検証済み」。
インストーラー導入と通常配布環境の品質確認を含む正式リリース判定は保留とする。

## Phase 3.3 安全な商品削除（2026-11-10 日本時間）

開始時HEADは `a95f846`、作業ツリーはクリーン。Commit・Push・ブランチ切り替えは行っていない。
商品削除プレビュー・確認dialog・商品と全画像の原子的DB削除・COMMIT後の複数画像隔離を追加した。
既存マイグレーション・依存関係・lockfile・識別子・保存先・CSP・Capabilities・Asset Protocolは変更していない。

### 自動検証

| コマンド | 最終結果 |
| --- | --- |
| `npm run build` | 終了コード0。TypeScript検査・Vite本番ビルド成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了コード0、71 passed / 0 failed / 0 ignored（既存57＋追加14） |
| `npm run tauri build` | 終了コード0。Windows releaseビルド、MSI（3.44 MiB）・NSIS（2.47 MiB）生成成功 |
| `git diff --check` | 終了コード0 |

追加14テストには複数ケースを含み、次の障害経路も検証した。

- 0・1・4枚の商品削除、商品と画像のDB削除、他商品・他画像の保持、再接続・再起動後の状態。
- 正常隔離の元バイト列・SHA-256・取り込み元画像の保持、deleted領域の非再処理。
- 更新日時不一致、同一日時のままの商品名・SKU・ブランド・メモ変更、画像追加・削除・並べ替え後の古い確認拒否。
- 重複・別商品・不正UUID・不在ID・不足画像IDの拒否。プレビュー不在時の `product_not_found`。
- 画像削除途中と商品DELETEのSQLエラーをtriggerで注入し、全DBレコードのロールバック・ファイル非移動を確認。
- 遅延外部キーによる実COMMIT失敗後のロールバック確認。解決ヘルパーへエラーを注入し、
  全参照なし、商品残存、一部参照残存、別商品への画像ID・パス参照、非autocommit、DB照会不能を確認。
  一部参照の不整合・照会不能は `product_deletion_unknown`、全ロールバックはDB操作エラーとなる。
- 複数画像で隔離先衝突・欠落・正常隔離を混在させ、4＝2＋1＋1の集計と衝突先の非上書きを確認。
- 全画像・商品ディレクトリ欠落の許容、不正パス・MIME/拡張子不一致・通常ファイルでない対象の全体拒否。
- Windows共有モード0の読み取りエラーは欠落にせず全体拒否。読み取り可・移動不可なら1枚保留・残り2枚隔離。
- Windows junctionの元画像パス拒否、隔離先junctionへの移動拒否と元管理画像の保持。
- 画像0・3枚で共通permitとDB Mutexを保持し、商品削除・取り込み・並べ替え・画像削除がDB待ちせずbusyとなること。
- 隔離保留画像が次回起動で `.recovered` に回収され、既存隔離ファイルは保持されること。

全自動テストは専用一時DB・メディアを使用し、本番LocalDataと既存test-fixturesは使用していない。
初回Viteはサンドボックスの子プロセス起動制限による `spawn EPERM`。通常権限で同じビルドを再実行して成功した。
新規テストの初回コンパイルではImagePreviewのPartialEqを要求するassertを修正し、再起動テストのDBパスを
Fixtureと同じ `test.sqlite3` に揃えた。既存テスト・Domainの画像比較仕様は変更していない。

### Windows実WebView2

`npm run tauri dev -- --config .verification/phase33/config.json` を使用し、公式 `appDirectoriesOverride` で
保存先を新設 `.verification/phase33/data` だけに限定した。実Rust IPC、React UI、ネイティブ画像選択を使用。
既存Phase 3.2のサンプル画像は読み取り・コピーだけ行い、既存の成果物は変更していない。

| シナリオ | 観測結果 |
| --- | --- |
| 商品登録・画像取り込み | 専用対象商品へ4枚、別商品へ1枚を取り込み表示 |
| 最新内容の確認・キャンセル | 商品名・SKU・4枚表示。キャンセル後も2商品を保持 |
| 初期フォーカス | プレビュー取得後のbusy解除前にdialogを開く問題を修正し、キャンセルへのフォーカスを確認 |
| 古い確認の拒否 | 確認中に実商品更新APIで名前・更新日時を変更。確定は拒否され、dialogが閉じて再確認を案内。商品は保持 |
| 最新内容での削除 | 再取得した確認に変更後の商品名を表示。確定後に一覧へ戻り、対象商品が消える |
| 隔離先衝突 | 専用衝突ファイルを1枚分置き、保留1枚の警告を実UIで確認。正常3枚は隔離し、衝突先は不変 |
| バイト列の保持 | 管理画像4枚のSHA-256・選択元バイト一致を確認。保留1枚は元管理パスに保持 |
| 画像0枚の商品 | 新規専用商品をUIの確認から削除し、一覧から消えることを確認 |
| 正常終了・再起動 | 対象商品・画像0枚の商品が再表示されない。別商品と画像APIの結果は削除前と一致 |
| 保留画像の回収 | 起動ログ `Quarantined 1 incomplete or orphaned media files`、回収画像SHA-256一致、正常隔離3枚は保持 |
| 実行時エラー | prepare/other/check/restartの各観測区間でJavaScript例外・CSP違反0件 |

証跡はGit対象外の `.verification/phase33/` に、スクリプト・結果JSON・商品/画像情報・
`confirm.png`・`deleted.png`・専用データとして保持した。アプリは検証後に正常終了した。
Vite watcherは専用WebView2プロファイルのCookies等で `EBUSY` を報告したが、上記操作・再起動検証は成功した。
最後のCOMMIT結果解決の不整合判定追加は、全71テストと型・静的チェックで再検証した。

### 変更と残る制約

追加：`services/product_deletion_service.rs`。
変更：Rustの `commands/product.rs`、`domain/product.rs`、`error.rs`、`image_tests.rs`、
`infrastructure/db/product_repository.rs`、`lib.rs`、`services/mod.rs`。
React/APIの `ProductDetail.tsx`、`ProductImages.tsx`、`ProductList.tsx`、`lib/api/products.ts`、`lib/api/request.ts`。
文書のREADME、architecture、data-model、ADR 0005、本書。

COMMIT成功後にI/Oエラーが返る実ストレージ故障は再現せず、結果解決分岐への注入で確認した。
削除中ボタン無効化・同期refガードはコードレビュー、Rustの競合拒否は自動テストで確認した。
厳密な二重クリックのIPC回数計測、配布版の実操作、インストーラー導入、別PC、電源断・実ディスク故障は未検証。
配布ビルドは通常設定で生成し、実行・インストールしていない。既存識別子の末尾 `.app` に関するmacOS向け警告は継続する。
親ディレクトリを検査直後に外部プログラムが差し替える攻撃への制限、Windows以外の削除隔離を保留とする制限は維持する。
画像の完全消去、アプリ内復元、複数商品一括削除は対象外。

## Phase 3.2 画像1枚の安全な削除（2026-11-10 日本時間）

開始時はmain / HEAD `46bd236`、作業ツリーはクリーン。Commit・Push・ブランチ切り替えは行っていない。
確認dialog付き削除、`delete_product_image`、単一IMMEDIATE内のDELETEと順序正規化、DB確定後の非置換隔離を追加した。
DBスキーマ、0001/0002 SQL、依存関係・lockfile、識別子、CSP・Capabilities・Asset Protocolは変更していない。

### 自動検証

| コマンド | 最終結果 |
| --- | --- |
| `npm run build` | 終了コード0。TypeScript検査とVite本番ビルド成功 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 終了コード0 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 終了コード0 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 終了コード0 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 終了コード0、57 passed / 0 failed（既存45＋追加12） |
| `git diff --check` | 終了コード0 |

追加12テストの複数ケースで次を確認した。

- 先頭画像の削除、同一商品・別商品の他画像保持、0始まり連番、新メイン画像、最後の画像の削除、再接続後の保持。
- 商品更新日時・他画像メタデータ・管理画像バイト列・取り込み元SHA-256の保持。
- 不在・別商品・不正UUIDの拒否、改ざんDBパス・ディレクトリ・Windows junctionの拒否。
- DELETE失敗と順序更新途中の失敗による全変更ロールバック、ファイル非移動、ガード再取得。
- 遅延外部キーによる実COMMIT失敗後の参照再確認。結果解決ヘルパーへエラーを注入した、確定済み削除・画像ID参照・パス参照・非autocommit・DB参照不能の各分岐。
- 非置換隔離・同一バイト保持、既存隔離先との衝突時に元画像と衝突先を保持し、DB削除済み・保留件数を返すこと。
- Windows共有モード0で読めない元画像を欠落と扱わずDB削除前に拒否すること、読み取り可能でも削除共有のないファイルは移動失敗・保留になること。
- 共通permit保持中に削除・取り込み・並べ替えがDB Mutex取得前にbusyで返ること。
- DB削除済み・未隔離の画像の起動時回収、deleted領域の非再処理、最終隔離時の欠落・不正ファイルの再判定。

全テストは専用一時DBとメディアを使用。本番LocalData・既存test-fixturesは使用していない。
初回の新規junctionテストはcmdのmklinkへ渡すパス中の `/` がスイッチと解釈されて失敗した。
検証ハーネスのパスをcanonicalizeした親とファイル名で組み立て直し、対象テストと全57テストが成功した。
初回のViteは制限環境の `spawn EPERM` で失敗。同一コマンドを通常権限で再実行し成功した。

### Windows実WebView2

`npm run tauri dev -- --config .verification/phase32/config.json` で、公式 `appDirectoriesOverride` を
今回新規作成した `.verification/phase32/data` に限定して起動した。
新規8×8 PNG 4枚、専用商品だけを使い、CDPによるReact UI操作・実IPCとネイティブ画像選択で確認した。

| 境界 | 観測結果 |
| --- | --- |
| 取り込み → UI | 4枚取り込み・表示成功 |
| 確認 → キャンセル | 対象ファイル名と確認dialog表示、キャンセルボタンの初期フォーカス、キャンセル前後で画像API結果一致 |
| 確定 → DB → UI | 先頭画像だけ削除、残存3枚のID・連番一致、新メイン表示、削除後の前後移動成功 |
| 正常終了 → 再起動 | 3枚の順序・画像表示が保持され、削除済み画像は表示されない |
| 隔離先衝突 → UI | 専用の衝突ファイルを置き、DB削除成功・隔離保留の警告を実画面で確認 |
| 最後の画像削除 | 残り3枚を1枚ずつ削除し、既存空状態を表示。専用SQLiteの画像件数0をアプリ終了後に確認 |
| 保留 → 次回起動 | 元の管理画像が `.recovered` に回収され、元バイト列と一致 |
| ファイル保持 | 正常隔離3枚は選択元とバイト一致、衝突ファイルは元のsentinelバイトを維持、元画像4枚を保持 |
| 実行時エラー | prepare/check/restart/finalの各検証区間でJavaScript例外・CSP違反0件 |

既存の検証成果物は変更せず、今回の証跡はGit対象外の `.verification/phase32/` に保存した。
`desktop.png`、`before.json`、`order.json`、各モードの結果JSON、`file-results.json`、スクリプトと専用データを保持した。
再起動の一回は前プロセス終了直後のViteポート1420競合で失敗した。開発サーバー終了確認後に再実行して成功した。
Vite watcherは専用WebView2プロファイルの一時ファイルに対して `EBUSY` を報告したが、各操作検証は完了した。
検証用アプリは最後に正常終了した。

### 変更ファイルと残る制限

Rust：`commands/image.rs`、`domain/image.rs`、`error.rs`、`image_tests.rs`、
`infrastructure/db/image_repository.rs`、`infrastructure/media_storage.rs`、`lib.rs`、`services/image_service.rs`。
React/API：`ProductImages.tsx`、`imageTypes.ts`、`lib/api/images.ts`、`lib/api/request.ts`。
文書：`README.md`、`docs/architecture.md`、`docs/data-model.md`、本書、新規ADR 0005。

今回 `npm run tauri build`、インストーラー導入、配布版CSP下の削除、別PC、電源断、実ディスク故障は未検証。
UI保存中ボタン無効化・二重Command件数の厳密な実行時計測は未実施。同期refガードはコード確認、Rustの競合拒否は自動テストで検証した。
COMMIT後にエラーが返る実ストレージ障害は再現せず、結果解決分岐をテスト入力で確認した。
パス検査直後の外部プログラムによる親差し替えは既存設計と同じ制約があり、専用領域のアクセス権を前提とする。
Windows以外の削除隔離は安全側の保留を返す。商品削除・アプリ内復元・完全消去は未実装。

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
