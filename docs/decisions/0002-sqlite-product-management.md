# 0002: SQLiteによる商品管理

- 日付：2026-11-09（日本時間）
- 状態：採用

Tauri 2 / React 19 / TypeScript 7 / Rust stable MSVCの既存構成を維持し、
rusqliteのbundled機能でSQLiteを組み込みます。システムSQLiteの導入を不要にします。
公式資料でrusqlite 0.41.1、TauriのPath API・async Command、UUID v4とUTC日時APIを確認し、
実際のlockfileとコンパイル・テストで既存構成との互換性を検証します。
既存のnpm依存・Tauriの意図的な更新は行いません。

## 決定と理由

- DBの接続・SQLはRustだけが扱います。検証と永続化の規則をReactと重複させず、任意SQLのIPCを作りません。
- Commands → Services → Repositoryの小さな分離と商品Domainを設けます。traitやDI、ORMは不要です。
- 接続は1本をMutexで共有し、商品Commandはspawn_blockingで処理します。単純なCRUDにプールは不要です。
- SKUとは独立したUUID v4を生成し、将来の画像やバッチとの関連付けを可能にします。
- SKUはtrim後にBINARY照合で一意とします。UnicodeやASCIIで異なる大文字小文字変換を持ち込まず、明確に区別します。
- 日本円を整数で保存し、JavaScript安全整数上限を適用します。多通貨は将来のマイグレーションで検討します。
- app_local_data_dirにDBを置きます。起動時の初期化失敗はUIへ通知し、データを失う再作成やメモリDBへの切替はしません。
- user_versionとトランザクションでマイグレーションを管理します。将来バージョンは拒否し、旧アプリの書き込みを防ぎます。

## 影響・今後

追加直接依存はrusqlite、serde（IPC型）、uuid（商品ID）、chrono（UTC日時）です。
SQL Plugin、ORM、状態管理ライブラリは追加しません。
当面のバックアップ・復旧は終了中のファイルコピーとし、自動化は別Phaseで検討します。
削除・CSV・画像・AI・ジョブ・WebデモはPhase 1に含めません。

## 公式資料

- [rusqlite](https://docs.rs/rusqlite/0.41.1/rusqlite/)
- [Tauri PathResolver](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html)
- [TauriのRust Command](https://v2.tauri.app/develop/calling-rust/)
- [UUID](https://docs.rs/uuid/latest/uuid/)
- [chrono](https://docs.rs/chrono/latest/chrono/)
