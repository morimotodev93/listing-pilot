# v0.2.0 — Phase 6.2以降のリリース判定チェックリスト

2026-11-21作成。根拠は[Phase 6.1監査](phase61-release-audit.md)と[過去の検証記録](verification.md)。
**Phase 6.2はユーザーによるPhase 6.1レビュー承認後に開始する。現時点でビルド・公開は未承認／未実施。**
以下の未チェック項目は今回の成功結果に含めない。

## Phase 6.1で確認済み

- [x] 公開v0.1.1のMigration 0001〜0005と現行SQLの一致。
- [x] 新規合成v5→v6の移行、旧全列・画像UUID／表示順・非表示グループ／項目／保存値の保持。
- [x] 原本・削除済み隔離画像・生成元のSHA-256保持。
- [x] DDL／FK／未来スキーマ／ロック／破損DBで失敗し、DBとmedia全バイト保持。
- [x] 停止済み一式のバックアップ、manifest照合、隔離復元、SQLite整合性、サービス再起動。
- [x] 最終ソースのnpm build、fmt、check、clippy、Rust 250テスト、差分検査。

## Phase 6.2 — 承認後のビルド準備・候補生成

- [ ] ユーザーのレビュー承認を記録し、基準HEAD・作業ツリー・今回からの差分を確認する。
- [ ] package.json、package-lock.json（ルート＋packagesの空文字キー）、Cargo.toml、Cargo.lock（自crate）、tauri.conf.jsonを0.2.0へ揃える。依存更新・Migration改変を混ぜない。
- [ ] com.listingpilot.app、データ保存先、CSP／Asset Protocol／Capabilitiesを維持する。
- [ ] 最終ソースに対しbuild／fmt／check／clippy／test／git diff --checkを実行し、終了コードと件数を記録する。
- [ ] `npm run tauri build -- --bundles nsis`で新規候補を生成する。MSIを配布対象にするなら`--bundles msi`も生成する。
- [ ] 各成果物の生成時刻、FileVersion／ProductVersion、x64、サイズ、SHA-256、Authenticode状態を記録し、旧成果物と区別する。
- [ ] 署名なしで配布する場合の警告・実行制限と説明をレビューする。署名・Updater・CIの新設は別判断とする。

## 隔離Release／実更新の受け入れゲート

- [ ] 専用Windowsユーザー／VM等の隔離環境を準備する。GUI検証でoverrideを使う場合は`app.appDirectoriesOverride`の専用絶対パスを**起動前**に検証し、通常データを開かない。
- [ ] 新規合成v5を使用する。実データのコピーを使う場合はユーザーから別途明示承認を得る。
- [ ] 停止済みDB・関連ファイル・media全体（quarantineを含む）を一組でバックアップし、件数・サイズ・SHA-256を記録する。
- [ ] 専用環境でv0.1.1→v0.2.0インストーラー更新、v5→v6、integrity_check、foreign_key_check、商品全列・値・画像UUID／順序／ハッシュ・隔離画像保持を確認する。
- [ ] Release WebView2で商品・画像表示、URL／alt設定、保存・再起動、10列／13列CSV、ネイティブダイアログ、範囲外Asset拒否、生成CSP／違反を確認する。
- [ ] Stickyバー6画面で800×600／400×600、先頭／中間／末尾、末尾入力・確認欄、Enter／Tab、保存エラー／競合／破棄dialog、方式切替・確認失効を確認する。
- [ ] Windowsの拡大率・低いウィンドウ高さ、フォーカス可視性、スクリーンリーダーを確認する。未確認範囲は明記する。
- [ ] export_writeエラー5を、Release／開発監視、リポジトリ内外の新規専用出力先、単品／一括／Shopify 10列・13列で切り分ける。原因と影響、未解決なら配布上の扱いを明示判断する。
- [ ] CSV完成フォルダーと内容を照合する。Phase 5.2.4のダイアログキャンセル成功だけを完成出力成功としない。
- [ ] WebView2 Runtime導入済み／未導入、オフライン、別PC／VMを確認する。MSI配布時はMSIも実導入・削除・データ保持を確認する。
- [ ] バックアップを別の新規専用領域へ復元し、ハッシュ・SQLite・GUI画像表示・終了／再起動を確認する。
- [ ] ロールバックが必要なら、移行後セットを保全し、移行前v5のDB・関連ファイル・media一式を復元する。v6のuser_versionを5へ戻さない。v6をv0.1.1で開ける保証はない。

## 最終判断

- [ ] 未解決事項と未検証範囲の重要度を再評価し、配布可否のユーザーレビューを受ける。
- [ ] Commit・Push・タグ・GitHub Release公開は、それぞれ明示指示がある場合だけ実施する。

過去Phase 6.0のv0.1.1 NSIS実機成功は、その版・実機1台・ユーザー報告の範囲でのみ有効。
v0.2.0更新、MSI、最新StickyバーのRelease受け入れの代替にはしない。
