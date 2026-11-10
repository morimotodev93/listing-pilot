# 0005: 商品・画像の安全な削除

- 日付：2026-11-10（日本時間）
- 状態：Phase 3.2の画像1枚削除、Phase 3.3の商品削除を採用・実装。

## 決定

A方式として、SQLiteの画像レコードを削除し、管理画像は保護領域に隔離して保持する。
取り込み元画像は変更せず、アプリ内復元・完全消去・一括削除・ゴミ箱UIは導入しない。
既存ADR 0001〜0004、0001/0002 SQL、依存関係、CSP、Capabilities、Asset Protocolの範囲を維持する。

## Phase 3.2で実装した画像削除

1. 共通画像操作permitを取得する。不在・別商品・不正UUIDは `image_not_found` とする。
2. DBから対象を取得し、DB Mutex解放後にUUID由来パスと通常ファイルを検査する。
   各既存親要素も検査し、リンク・Windows reparse point・通常ファイルでない対象を拒否する。
   NotFoundだけは欠落として許可し、権限・I/O障害はDB変更前に拒否する。
3. IMMEDIATEトランザクションで最新画像が検査したレコードと同一であることを再確認する。
   商品IDと画像IDで1件DELETEし、残存画像を `sort_order, created_at, id` 順に0から連番化する。
   商品更新日時を変更しない。途中失敗はロールバックし、ファイルを動かさない。
4. COMMIT後、DB Mutexを保持せず `originals/{商品UUID}/{画像UUID}.{ext}` から
   `quarantine/deleted/{商品UUID}/{画像UUID}.{ext}` へ移動する。permitは完了まで保持する。
   Windowsでは既存の `MoveFileExW` flags=0を使用し、同一ボリューム・非置換をOS操作で強制する。
   取り込み用 `place_new` の動作は変更しない。Windows以外の削除隔離は安全側の保留とする。
5. DB削除確定後の隔離失敗は成功報告に保留件数を付け、UIで警告する。管理画像を消去しない。

COMMITが失敗を返した場合はトランザクション解放後にDBを再照会する。
autocommitでなければ結果不明とし、画像IDまたは相対パスの参照が残ればファイルを保持する。
両方の参照がないことを確認できた場合だけ削除済みとして隔離へ進む。
DB状態を確認できなければ `image_deletion_unknown` を返し、成功を断定せず再起動確認を案内する。

UIは対象画像・ファイル名・元に戻せない旨を確認dialogに表示する。
キャンセル時はIPCを送らず、確定中は同期refとRustの共通permitで競合を拒否する。
削除後の一覧再取得により、先頭のメイン画像・0枚時の空表示を更新する。

## 回復と制約

DB確定から隔離までにクラッシュする窓は許容する。既存の起動時検査はDB利用可否確認後にOSロックを取得し、
ロック下で最新参照を再取得してから、未参照のoriginalsとstagingだけを `.recovered` へ隔離する。
deleted領域は走査しない。永続的削除履歴は追加せず、クラッシュ後の回収先がdeletedになる保証は求めない。

パス検査直後に同一ユーザーの別プログラムが親ディレクトリを差し替える攻撃全般は防げない。
アプリ専用保存先のアクセス権を前提とする。電源断・ディスク故障を含む永続化は保証しない。
バックアップにはアプリを停止した状態でSQLiteとmedia全体を同じ時点の組で保存する必要がある。

## Phase 3.3で実装した商品削除

`get_product_delete_preview({ productId })` は読取トランザクションで商品と全画像を一貫した状態から読み、
`product_id, name, sku, updated_at, image_count, image_ids, product` を返す。画像IDは既存の正規表示順とする。
`product` は商品の全フィールドで、同じ更新日時のまま名前・SKUなどが変わる場合の検出にも使用する。
プレビューはファイルを変更せず、不在は `product_not_found` を返す。

`delete_product({ input })` のinputは `productId, expectedUpdatedAt, expectedImageIds, expectedProduct`。
確認内容はクライアントが保持し、トークン用テーブルは作らない。UUID、商品全体・更新日時、重複のない全ID順序の
完全一致を検証する。不在・別商品ID・古い内容などは `stale_data` とし、古い確認を閉じて再確認を案内する。

1. 共通permitを取得し、DB読取トランザクションで事前照合する。画像0枚も同じガードを使用する。
2. Mutexを解放し、全画像のUUID由来パス・MIME・親・通常ファイル・リンク/reparse pointを検査する。
   確認できたNotFoundのみ許容し、1件でも危険・読取不能ならDB削除全体を中止する。
3. 単一IMMEDIATE内で確認スナップショットを再照合し、全画像のメタデータも事前検査時と完全一致を確認する。
   全画像を商品IDと画像IDで1件ずつ削除し、商品ID指定のDELETEが1件であることを確認する。
   関連参照なしを確認してCOMMITする。途中失敗は全件ロールバックし、ファイルを動かさない。
4. COMMIT失敗はtransaction解放後にautocommitを確認し、読取トランザクションで商品ID・関連画像・全画像ID・
   全相対パスを再照会する。全参照なしなら隔離し、参照が残れば保持する。
   一部だけ残る不整合や確認不能は `product_deletion_unknown` とし、再起動確認を案内する。
5. DB削除確定後、Mutexを保持せず全画像へ既存 `quarantine_deleted()` を適用する。
   1件の失敗で残りを中止せず、元管理パスに残して保留に加算する。DBレコードは再作成しない。

返却値は `deleted: true, affected_image_count, quarantined_image_count, missing_image_count, pending_quarantine_count`。
対象数は隔離済み・欠落・保留の合計で、画像0枚はすべて0となる。既存の画像削除と同じRust報告型を再利用する。
UIは最新の商品名・SKU・枚数と不可逆性を確認し、同期ref・stateで再送信や他操作を防ぐ。
成功後は商品一覧へ戻して再取得し、保留があれば警告する。permitは隔離終了まで保持する。
Windowsの非置換移動・起動時回収・保護領域非公開・復元や完全消去を行わない方針は画像1枚削除と共通とする。
