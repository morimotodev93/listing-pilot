# ADR 0004: 商品画像の保存と管理

**ステータス:** 採用・実装済み（インストーラー経由の導入・別PCでの検証は残課題）

**対象:** Listing Pilot Phase 2

## 1. 背景

Listing PilotはPhase 1でSQLiteによる商品管理を実装した。

Phase 2では、商品UUIDに紐付いた複数画像の取り込み、非破壊保存、メタデータ管理、プレビュー表示を実装する。

画像ファイルとSQLiteの更新は単一のトランザクションにできないため、整合性と障害復旧を考慮する必要がある。

## 2. 決定事項

### 画像の保存

- 画像本体はSQLite BLOBではなく、アプリ管理ディレクトリにファイルとして保存する。
- SQLiteには画像UUID、商品UUID、保存先相対パス、画像メタデータ等を保存する。
- 元ファイルは変更・移動・削除しない。
- 保存画像は元画像と同一のバイト列を保持する。
- 保存ファイル名は画像UUIDを使用する。
- 元のファイル名はメタデータとして保持する。

### ディレクトリ

Tauriの `app_local_data_dir()` 配下を使用する。

```text
media/
  originals/
    {product-uuid}/
      {image-uuid}.{ext}
  .staging/
  quarantine/
```

一時保存先と正式保存先は同じボリューム内に配置する。

### データモデル

`product_images` テーブルを追加し、商品と画像を1対多で関連付ける。

マイグレーションの正本は、既に作成された `src-tauri/migrations/0002_create_product_images.sql` とする。

想定するメタデータは画像ID、商品ID、元ファイル名、相対パス、MIME形式、ファイルサイズ、画像寸法、SHA-256、表示順序、作成日時である。

同一商品の同一内容の画像は重複登録しない。他の商品への登録は許可する。

実装前に既存SQLの実際のカラム、商品外部キー、パス一意制約、商品内ハッシュ一意制約を確認した。重大な不一致はなく、SQLを変更せずRustを定義に合わせた。

### 取り込み処理

Rust側でファイル選択・検証・コピー・DB登録を行う。

基本手順：

1. 商品の存在確認
2. ネイティブダイアログで画像選択
3. 一時領域へのコピーと画像検証
4. ハッシュによる重複判定
5. 正式保存先への配置
6. SQLiteへのメタデータ登録
7. 成功・重複・失敗の結果返却

複数画像は原則として個別に処理し、一部失敗時も成功済みの画像を保持する。

DB登録失敗時は補償処理を行う。コミット成否が不明な場合は参照状態を確認し、登録済みファイルを誤って削除しない。

異常終了により残された一時ファイルや孤立ファイルは、起動時の整合性検査で検出する。孤立ファイルは安全性を確認して隔離し、自動的な完全削除は行わない。

### 画像表示

Tauri Asset Protocolを使用する。

- Asset Protocolのスコープは `media/originals/` 配下に限定する。
- `.staging`、`quarantine`、SQLiteファイルは公開しない。
- CSPは画像表示に必要な `img-src` のみ拡張する。
- Reactは `src/lib/api/` を通して検証済みの画像情報を受け取る。
- ユーザーから任意のファイルパスを受け取って読み込む汎用IPCは作成しない。

### 初期制限

- JPEG、PNG、WebPに対応する。
- 1ファイル20MiB、4,000万画素、1回20枚を初期上限とする。
- 拡張子だけではなくファイル内容を検証する。
- リサイズ、形式変換、AI生成、並べ替えUI、画像削除はPhase 2の対象外とする。

## 3. アーキテクチャ

既存のRust単一crateとモジュラーモノリスを維持する。

- Commands：Tauri IPCの入口
- Services：画像取り込みと整合性管理
- Domain：画像モデルと検証ルール
- Infrastructure：SQLiteおよびファイルシステム操作

Reactコンポーネントから直接 `invoke()` を呼び出さない。

Repository trait、DIコンテナ、イベントバス、ジョブ管理基盤は導入しない。

## 4. セキュリティ

- WebViewに広範なファイルシステム権限を付与しない。
- ダイアログ操作はRust側で実施する。
- 保存先はアプリ管理ディレクトリに限定する。
- パストラバーサルやシンボリックリンクによる管理領域外アクセスを防止する。
- CSPはADR 0003の制限的な方針を維持する。
- エラーメッセージには内部の絶対パスや機密情報を含めない。

## 5. バックアップ

Phase 2以降、商品データの完全なバックアップにはSQLiteと `media/` の両方が必要となる。

バックアップはアプリ終了状態で取得する。

## 6. 検証条件

- 既存商品のデータがマイグレーション後も保持される。
- 複数画像を取り込める。
- 元ファイルと保存ファイルのSHA-256が一致する。
- 重複画像と不正画像を処理できる。
- DB・ファイル操作失敗時の整合性を確認できる。
- 再起動後も画像を表示できる。
- スコープ外ファイルはAsset Protocolで読み込めない。
- Windowsの開発・配布ビルドで正常動作する。

## 7. 不採用

- SQLite BLOBへの元画像保存
- 元画像の移動または上書き
- Reactへの汎用ファイルアクセス権限の付与
- 広範なAsset Protocolスコープ
- Phase 2での本格的な画像変換・AI処理
- 過剰なアーキテクチャ抽象化

## 8. 関連資料

- ADR 0002：SQLite Product Management
- ADR 0003：Content Security Policy
- `docs/architecture.md`
- `docs/data-model.md`
- `docs/verification.md`
- [Tauri Dialog](https://v2.tauri.app/plugin/dialog/)
- [Tauri Asset Protocol Scope](https://v2.tauri.app/security/asset-protocol/)
- [Tauri CSP](https://v2.tauri.app/security/csp/)

## 9. 実装確定事項（2026-11-09 日本時間）

- 相対パスは `media/originals/{product_id}/{image_id}.{ext}` とし、MIMEから `jpg` / `png` / `webp` を決定する。拡張子は選択元に依存しない。
- ファイルは20MiB+1までに読み取りを制限してstagingへコピーし、flush・sync後に検証する。ヘッダーの縦横の積を確認してから全デコードする。デコーダー割り当て上限256MiBも適用し、Limitsエラーは上限超過として扱う。このライブラリの割り当て制限は一部best effortであり、プロセスメモリ全体の厳密な上限ではない。
- SHA-256はコピー済みstagingから計算し、検証対象・保存対象を一致させる。取り込み中に選択元が変わる可能性を考慮し、元ファイルは読み取りだけ行う。
- 正式配置は同一ボリューム内rename。DBは単一INSERTのautocommitとし、表示順も同じSQL内で決める。ファイル・検証・ハッシュ中にDB Mutexを保持しない。
- DBエラー時も参照を再確認する。SQLiteのFAILなど書き込みが残る場合は成功として保持し、参照なしと確認できたファイルだけ補償削除する。確認不能や削除失敗は次回起動の隔離へ委ねる。
- 整合性検査はアプリ起動中のDB初期化・参照取得が成功した後、Commandを公開する前だけ行う。参照集合全体と候補ファイルを検証してから隔離する。
- `media/.lock` のOS排他ロックをプロセス生存中保持する。二つ目のプロセスは画像機能を停止し、画像を整理しない。ロックファイルの存在自体は使用中判定に使わない。異常終了時もOSがロックを解放する。
- プロセス内permitをダイアログ前から結果返却まで保持する。二重Commandもbusyとして拒否し、取り込み中のファイルを整合性検査が隔離しない。
- 各パス要素でシンボリックリンク・Windows reparse pointを拒否し、元ファイルと管理領域の通常ファイル・ディレクトリを確認する。WindowsではファイルをOPEN_REPARSE_POINTで開く。ローカルの別プログラムがチェック直後に親ディレクトリを差し替える攻撃を完全に防ぐ設計ではなく、アプリ専用ディレクトリのアクセス権を前提とする。
- Asset Protocolの設定は `$APPLOCALDATA/media/originals/**/*` のみに限定。CSPはADR 0003に従い `img-src` だけに `asset:` / `http://asset.localhost` を追加し、Capabilitiesは変更しない。
- `tauri-plugin-dialog` のRust builderを使い、JS用dialog Commandを公開しない。JS用Commandによる選択元の動的Assetスコープ拡張を避けるためでもある。
- Windows実WebView2で、画像表示・再起動・部分失敗・重複・キャンセル・二重Command拒否と、選択元を含むスコープ外のHTTP 403拒否を確認した。release版でも本番CSP下の画像表示・ダイアログ・重複通知・欠落表示と復元・範囲外拒否を確認した。検証には公式 `appDirectoriesOverride` 設定を一時的に使用し、本番LocalDataに触れていない。詳細は `docs/verification.md`。

判断理由は、元画像を保持しながら小さな補償・隔離処理でDBとファイルの障害境界を扱うためである。
クラッシュ時に未登録ファイルが残る窓は許容し、次回起動時に完全削除せず隔離する。
物理的な電源断に対するディレクトリエントリの永続化、ディスク故障、自動復旧は保証しない。DBとmediaを揃えた正常終了時バックアップを基本とする。

参考：[Tauri設定](https://v2.tauri.app/reference/config/)、[image Limits](https://docs.rs/image/0.25.10/image/struct.Limits.html)、[Rust File locking](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)。

## 10. 最終レビュー修正（2026-11-09 日本時間）

初期化の参照一覧をOSロック取得前に確定すると、直後に別プロセスが登録した画像を孤立ファイルと誤認する可能性があった。
DB初期化・事前参照取得の成功を確認してからOSロックを取得し、そのロック下で改めて全参照を取得する。
隔離判定には二度目の一覧だけを使用し、再取得が失敗した場合はファイルを動かさず初期化エラーを返す。
既存のOSロック・ImportPermit・短いDB操作の方式を維持し、事前確認に失敗した場合はメディアを作成しない。

Windowsの正式配置は `MoveFileExW` のflags=0に変更した。標準の `fs::rename` は既存ファイルを置換し得るため、
事前存在確認だけでは残る競合をOS操作の段階で拒否する。置換・別ボリュームコピーのフラグは付与しない。
標準ライブラリで絶対パスをverbatim形式にして長いパスを維持し、Kernel32への小さなFFIだけで実装する。追加依存関係は導入しない。
Windows以外のコンパイル経路は同一ボリュームのhard_linkによる非置換配置とstagingのDrop整理を使用する（今回の検証対象はWindows）。
この変更は既存正式画像と元画像の保持を優先し、UUID・原子的な正式配置・登録失敗時の補償処理を維持する。

参照：[Microsoft MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)、[Rust fs::rename](https://doc.rust-lang.org/std/fs/fn.rename.html)。
