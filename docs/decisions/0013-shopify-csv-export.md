# 0013: Shopify向けCSVエクスポート（Phase 5.1）

- 日付：2026-11-17（日本時間）
- 状態：実装済み・ローカル実機検証済み（コードレビュー／ユーザーによるDev Store受入確認待ち）
- 対象基準：`ee30faf`（Phase 4.6完了時点）
- 関連：ADR 0010（単品汎用出力）、0011（グループ属性）、0012（一括汎用出力）

## 背景と検証済みの事実

Listing Pilotは1商品1SKU・円建て整数価格・公開用タイトル／説明文を管理し、
汎用の単品／一括CSV＋画像原本エクスポートを備える。Shopifyへの直接登録はまだ行わない。

2026-11-17にShopify Dev Storeで次を確認した。

- Shopify管理画面からダウンロードした `product_template.csv` は61列。新しい見出しに `Title`, `URL handle`, `Description`, `Vendor`, `SKU`, `Option1 name`, `Option1 value`, `Price`, `Published on online store`, `Status` が含まれる。
- Shopifyからの `products_export.csv` は47列で、`Handle`, `Body (HTML)`, `Variant SKU`, `Variant Price`, `Published` などの旧系統の見出しで出力された。
- 10列へ絞った新テンプレート形式と旧エクスポート形式を別商品でインポートし、両商品がDev Storeに登録された。再エクスポート `products_export (2).csv` で、両商品ともSKU・価格・`Status=draft`・`Published=false`、デフォルトオプション `Title / Default Title` を確認した。
- 新テンプレート形式で入力した日本語の説明文は、再エクスポート時に `Body (HTML)` の `<p>…</p>` として表現された。
- **この結果はCSV形式の互換性を示すが、ストア通貨がJPYであること、既存商品の安全な更新、実販売への適合性は検証していない。**

Shopify公式ドキュメントは新しい列名を案内している。新規商品では原則 `Title` が必須、SKU等のバリエーション値には `Option1 name` と `Option1 value` が依存する。ファイルのUTF-8/LF、15MB制限、上書きオプションの挙動にも注意する。

- https://help.shopify.com/ja/manual/products/import-export/using-csv
- https://help.shopify.com/en/manual/products/import-export/import-products

## 決定：既存の汎用出力とは独立したShopify用プロファイル

- 「選択商品をShopify用CSVで出力」を**別アクション**として追加する。対象は1〜20商品の選択済みUUID集合。既存の `export_product` / `export_products`、汎用CSVの列・ディレクトリ構造・画像コピー・トークン契約は変更しない。
- Shopify専用CSVは1ファイル、1商品1行、1デフォルトバリエーション。画像URL、メタフィールド、在庫、送料、税、カテゴリ、コレクション、SEO、複数バリエーションは扱わない。
- ストアへの送信、認証、Shopify API、ストア通貨の自動取得、既存商品更新（上書き）と双方向同期は実装しない。
- 商品のShopify側ID・同期状態をDBへ保存しない。DB Schema Version 5、Migration、画像原本、既存依存関係、Tauriセキュリティ設定を維持する。

## CSV契約

ヘッダーは以下の**正確な10列・順序**に固定する。

```csv
Title,URL handle,Description,Vendor,SKU,Option1 name,Option1 value,Price,Published on online store,Status
```

| Shopify列 | 値の生成元／ルール |
|---|---|
| `Title` | 既存の `ListingPreview.title`。公開用タイトル優先、未設定時は管理商品名を候補として使用し明示警告 |
| `URL handle` | `lp-` + 正規形UUID文字列。例：`lp-123e4567-e89b-42d3-a456-426614174000`。商品改名・SKU変更・並び順に左右されない |
| `Description` | `ListingPreview.description` のみ。未設定は空欄とし明示警告。管理メモを流用しない。HTMLへ自動変換しない |
| `Vendor` | `Snapshot.product.brand`。未設定は空欄。Shopify側での既定値付与の可能性を表示上説明 |
| `SKU` | `ListingPreview.sku`。必須（既存商品モデルの制約を利用） |
| `Option1 name` | 固定 `Title` |
| `Option1 value` | 固定 `Default Title` |
| `Price` | `ListingPreview.price_yen` を通貨記号なしの非負10進整数で書く。`0` は有効。未設定（NULL）は**Shopify出力不可**。小数・為替変換なし |
| `Published on online store` | 固定 `false` |
| `Status` | 固定 `draft` |

- UUIDはRustで正規形・一意性を検証する。順序はUUID昇順に正規化し、入力順に左右されない。バッチ上限は20商品。存在しない商品を黙って飛ばさない。
- **CSVはUTF-8（BOMなし）、カンマ区切り、LF (`\n`) レコード終端**。ヘッダーと全セルはRFC 4180相当の引用符／二重引用符エスケープを一貫して行う。日本語・カンマ・引用符・公開説明文中の改行を保持する。既存汎用CSVのBOM＋CRLF契約はそのまま。
- Shopifyインポートの上限15MB未満（UTF-8バイト長で検査）とする。超過は専用エラー。20商品で通常は十分小さいが防御的に検証する。
- 受け取り側CSVの更新挙動により空列が既存データを消し得るため、初回版は**新規登録用**と明示する。既存Handleと一致したときの挙動は利用者のShopifyインポート設定に依存する。アプリが衝突を検出できるとは主張しない。
- 価格の単位は**Listing PilotのJPY**。ストア通貨はオフラインCSV生成では検出できないため、UIにJPYストア向けであることを明示し、利用者に対象ストアの通貨を確認させる。非JPYストアでは同じ数字が別通貨として解釈されるので、変換せず使用を避ける。

## プレビューとCSV固有の出力可否

- 既存 `listing_preview_service::read/build` の商品候補・既存スナップショットトークンを再利用し、**Shopify専用の表示・可否判定**を構成する。商品名、SKU、出品タイトル、説明、ブランド、価格、ハンドル、対象商品と警告・出力不可理由を表示する。
- Shopify CSVへコピーしない画像について、`ImagesMissing` と `ImageUnavailable` は**Shopify専用プレビューの出力可否判定へ反映しない**（画像未登録でも出力可／画像ファイルを開かない）。ただし、既存の候補トークンが画像の変更でも変わる保守的な失効は許容する。汎用出力の画像警告・blocked判定は不変。
- Shopify専用警告は少なくとも「公開タイトル未設定（管理商品名フォールバック）」「公開説明文未設定」。**価格未設定は出力不可**（Shopifyで空価格が0と解釈されうるため）。危険なCSV文字列も出力不可。
- CSV式インジェクション防止は既存 `services/export_csv.rs` の厳格な `text` 検証（先頭空白・不可視文字・複数行の検査を含む）を再利用し、`Title`・`Description`・`Vendor`・`SKU` に適用する。固定列とUUID由来ハンドルは生成規則を検証する。CSV文字列の自動変形（アポストロフィ付加など）はしない。
- **プレビュー段階**で実際のCSV組み立てと同じ事前検証を行い、エラーの場合は対象商品UUIDと列名を伝える。プレビューでは成功して実行時だけ数式リスクが出る設計を避ける。
- 失敗理由を表示する際、危険なセルの原文、DBパス、管理原本パスなどは出さない。

## API・変更検出・確認

- Rust IPCの新規コマンド（命名案）：
  - `get_shopify_export_preview({ input: { productIds } })`
  - `export_shopify_products({ input: { productIds, snapshotToken, warningToken } })`
- 入力はcamelCase / `deny_unknown_fields`。1〜20件・正規UUID・重複拒否。プレビューに各商品、生成ハンドル、状態、Issue、一覧全体の `snapshotToken`、警告有無、blockedの有無を含める。
- トークンは既存バッチトークンと相互流用しない。例：`SHA-256("ListingPilot-shopify-csv-v1\\0" || 正規化商品UUID＋既存単品snapshotToken列)`。同一集合・同一状態では決定的。バージョン識別子はCSVプロファイル変更時に更新する。トークンは同一実行バージョンの一時的確認用途で、永続識別・認証・外部Shopify商品IDではない。
- blockedが1件でもあれば全体を出力不可。warningがあれば候補と問題の確認、`warningToken == snapshotToken` の明示承認が必要。再読込／キャンセル／商品集合変更／エラーで承認を破棄する。
- 通貨はバックエンドで検証できないため、CSV用UIに「**対象Shopifyストアの通貨がJPYであることを確認した**」という独立したチェックを置く。未確認では実行ボタンを有効化しない。確認の意味と制限を正しく表示する（JPYをアプリ側で検知したとは表現しない）。

## 出力・全件成功・失敗時保護

出力先はユーザーがRustネイティブダイアログで選ぶ、既存通常フォルダーとする。完成形は以下とする。

```text
{選択した親フォルダー}/
  ListingPilot_Shopify_{YYYYMMDD}_{出力UUID}/
    shopify-products.csv
```

- 商品名・SKU・ブランドはファイル名に使わない。`Workspace` に固定ルートファイル `shopify-products.csv` を**限定的に**許可する。汎用の任意パス書込みAPIや再帰清掃は追加しない。
- メディアpermit取得 → 全商品の単一読取トランザクションによるスナップショット取得 → 候補／Shopify検証・CSV生成 → ネイティブフォルダーダイアログ → 再取得・検証 → DB Mutex解放下で固定CSVのステージング書込と `sync_all` → 短い `IMMEDIATE` で全商品スナップショットと宛先を再照合 → `Workspace.finish()` で**一度だけ非置換確定**。
- DB Mutexを長いI/O中に保持しない。確定後に失敗し得るCOMMITを実行しない。`Workspace.cleanup()` は自分が作ったものだけを個別削除する。結果不明や清掃失敗では保持・通知し、外部フォルダーを再帰削除しない。
- CSVのみを書き出すので原本画像コピー、画像SHA-256再検証、汎用一括の200画像・1GiB上限は適用しない。通常データを更新しない。
- 成功／キャンセル／失敗の区別は既存 `ExportReport` と同程度の契約で維持する。成功では商品件数・生成CSVの場所・警告有無を表示する。画像・属性の未出力を誤認させない表示にする。
- `export_write`（リポジトリ内への最終移動時のWindowsエラー5）は解決済みと扱わず、開発時はリポジトリ外で確認。安全検証や非置換確定を緩和しない。

## UI・テスト・対象外

- ProductListの現行UUID選択・グループフィルターをそのまま利用し、汎用一括出力とは独立した「Shopify用CSVを出力」を提供する。選択集合は開始時に固定し、処理中は選択変更を禁じる。単品だけ選択したケースも同じShopify出力経路で処理する。
- `src/lib/api/` に専用ラッパーを置き、Reactから直接Tauriをinvokeしない。画面で明示的に「画像・カスタム属性は含まれません」「新規登録用、既存商品上書きオフ」「JPYストア用」「下書き・非公開」を表示する。
- Rustでは10列の固定順・日本語・説明文の改行／カンマ／引用符・0円／NULL価格・UUIDハンドルの安定性・集合順序非依存・重複／20件超・式インジェクション（Vendorを含む）・warning承認・画像非依存・前後の変更検出・複数商品一体確定・既存フォルダー保持・清掃失敗／結果不明を試験する。
- TypeScriptビルド／Rust fmt, check, clippy, testに加え、WebView2で警告・キャンセル・再読込・グループまたぎ選択・CSV取り込みを検証する。Shopify Dev Storeで**実際に新規1〜2商品をインポート→再エクスポート**し、SKU・価格・Status・Published・日本語説明文を比較する。既存同名Handleへの再インポートは初期受け入れ試験の対象外。
- Phase 5.2以降：画像URLやShopify Files、メタフィールド、マルチバリエーション、ストア／通貨選択、直接API連携、リモート存在判定、既存商品更新、出力履歴など。

## 制約と保証しないこと

CSVの生成成功はShopifyへのインポート成功、JPY通貨設定、売価の妥当性、販売チャネルの公開状態やストア上の重複不存在を保証しない。ローカルの非置換確定はWindows上の既存手法の範囲内であり、電源断・ネットワーク共有・外部プログラムによる競合攻撃の完全な解決を意味しない。

## 実装上の共通化と確認状況

`batch_export_service::normalize/read` のみを小さく共有し、汎用のbuild・limits・prepareは使用しない。
`listing_preview_service::build` の候補・トークン生成を共通内部関数へ抽出し、`build_for_csv` は画像パスを解決しない。
Shopify専用DTOでは画像由来Issueを除き、価格未設定だけを出力不可へ引き上げる。
ShopifyトークンはこのファイルI/Oなしの候補トークンを専用ドメインで包むため、汎用単品／一括とは相互利用できない。
既存buildの呼出と画像確認・トークン生成は従来どおり。画像ファイルの欠落／復元ではShopifyトークンは変わらず、DB上の画像メタデータ変更では保守的に失効する。

Rustの追加15件を含む191件とTypeScript／Vite・Rust品質検査、隔離領域での実Tauri／WebView2を確認した。
Shopify Dev Storeでの本実装CSVのインポート／再エクスポートはユーザー担当で未実施。
実施範囲と証跡は [検証記録](../verification.md) を参照。
