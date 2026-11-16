# 0010: 商品1件の汎用CSV・原本画像エクスポート

- 日付：2026-11-16（日本時間）
- 状態：採用・Phase 4.5

## 出力契約

選択した親フォルダーに `ListingPilot_{商品UUID}_{ローカル日付YYYYMMDD}/` を作る。
SKUなどのユーザー入力はファイル名に使わない。同一商品・同一日の再出力は別の親フォルダーを選ぶ。
完成フォルダーが既に存在する場合は衝突として停止し、置換しない。

| ファイル | 固定列 |
|---|---|
| product.csv | product_id,sku,title,title_source,description,price_yen,main_image_path,image_count |
| attributes.csv | product_id,attribute_id,name,value,unit |
| images.csv | product_id,image_id,sort_order,relative_file,sha256 |

UTF-8 BOM、カンマ区切り、CRLFレコード区切り。データセルはすべて引用符で囲み、内部の引用符を二重化する。
公開用説明文の内部改行は保存文字列を保持する。NULLは空セル、0円は `0`。
属性・画像が0件でもヘッダーを作る。画像のCSV順は0始まり、ファイル名の連番は1始まり。
画像は `images/0001-{画像UUID}.{jpg|png|webp}`。元ファイル名は使用しない。

`listing_preview_service` の共通read/buildを使い、候補・Issueを再実装しない。
公開用説明文だけを出力し、内部メモをCSVへ含めない。有効で設定済みの属性だけを候補順に出力する。
Numberは既存の厳密な文字列検証を通し、負数・先頭ゼロ・小数精度をそのまま保持する。

## 数式リスク

CSV固有の検証を `services/export_csv.rs` に分離する。SKU・候補タイトル・公開説明文・属性名・Text値・単位を検査する。
各行の先頭空白・制御文字・不可視の書式文字を考慮し、最初の有効文字が `= + - @` なら
`csv_formula_risk` と固定の対象列識別子を返す。元の値・絶対パスはエラーへ入れない。
値の自動修正やアポストロフィ追加はしない。正当なNumberと整数価格は文字列数式判定から区別する。
UUID・MIME由来パス・ハッシュの生成規則もRustで検証する。共通プレビューIssueは変更しない。

## 確認と競合

プレビューへ追加する `snapshotToken` は、商品全体・画像メタデータと順序・全定義・商品値、
画像の利用可否とIssueをSHA-256で識別する不透明な値。同じ実行バージョン内の一時的な確認用で、保存・認証用途ではない。
内部情報はハッシュの入力だけに使い、DTOへ原文を追加しない。関連しない定義変更でも安全側に競合とする場合がある。

`export_product({input:{productId,snapshotToken,warningToken}})` はreadyだけを無確認で許可する。
warningは表示したIssueと候補への明示確認が必要で、warningTokenが現在のsnapshotTokenと一致する場合だけ許可する。
blockedは拒否する。Reactは同期refで二重送信を防止し、再読込・実行完了時に確認を解除する。
キャンセルは独立した結果であり成功にしない。

permit取得 → 単一読取トランザクションで再取得・候補照合 → ネイティブダイアログ →
再取得・候補照合 → DB Mutex解放 → 一時出力・画像検証 → 短いIMMEDIATEで全状態を再照合 → 非置換確定、の順。
最後のIMMEDIATEは、単一接続Mutexと別接続の書込排他を確定まで保持し、読取専用のままdropする。
確定後に失敗し得るCOMMITは行わない。コピー・ハッシュ計算中はDB Mutexを保持しない。
メディアOSロックは既存MediaStorage、permitはダイアログから処理終了まで保持する。

## ファイル保護と失敗

選択先は存在する通常フォルダーに限り、全パス要素のリンク・junction・reparse pointを拒否する。
canonicalizeしたアプリLocalData自身と配下は拒否し、DB・originals・staging・quarantineへ出力しない。
同じ親にUUID付き `.ListingPilot-export-{UUID}` をcreate_dirで作成する。
ファイルはcreate_new、CSVと画像はsync_allまで成功してから確定する。

画像は既存MediaStorageのUUID・MIME・管理相対パス照合と安全な読取専用openを再利用する。
20MiB+1の読取上限付きでコピーしながらサイズ・SHA-256を計算し、登録時の値と照合する。
再圧縮・再エンコードは行わない。欠落・不正パス・読取失敗・サイズ/ハッシュ不一致は全体を失敗にする。

Windowsは既存 `MoveFileExW(flags=0)` を再利用し、同一親のディレクトリを移動する。
置換や別ボリュームへのcopy/deleteフラグを指定しない。存在確認→標準renameでは保証しない。
[MicrosoftのAPI契約](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)は
既存ディレクトリへの移動でエラーを返す。Windows以外では非置換のディレクトリ移動を保証する実装がないため出力を拒否する。

通常の失敗では、当該処理がcreate_newできたファイルだけを個別削除し、空のimages・一時ディレクトリだけをremove_dirする。
再帰削除や親フォルダーの走査清掃はしない。不明な追加ファイル・リンク・清掃失敗は保持し、元のエラーと残存場所を返す。
確定結果が不明なら清掃せず、失敗結果と確認すべき選択先を返す。成功・キャンセル・失敗を混同しない。
電源断後の一時出力は起動時に自動清掃しない。

外部プログラムによる検査直後の親ディレクトリ差替えやコピー先改変を完全に防ぐ設計ではない。
電源断・ディスク故障に対するディレクトリエントリの永続性、ネットワーク共有での障害復旧、インストーラーや別PCは別途検証が必要。
スキーマVersion 4、依存ロック、アプリ識別子、CSP・Asset Protocol・Capabilitiesは変更しない。
Phase 4.6でも候補生成・CSV検証・出力先管理を別モジュールとして再利用できる構成とし、一括出力や履歴保存は追加しない。
