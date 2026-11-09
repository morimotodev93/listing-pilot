# 0001: 初期技術構成

- 日付：2026-11-06
- 状態：採用

## 決定

公式 `create-tauri-app@4.7.5` の `react-ts` テンプレートをnpmで生成し、Tauri 2を使用します。
React 19、Vite 8、TypeScript 7.0.2の安定版を採用します。npm依存はlockfileで固定し、
Rustは `rust-toolchain.toml` でstable/MSVCを指定します。

## 理由

[TypeScript公式発表](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/) と
npm公式レジストリの `latest` がTypeScript 7.0.2を示すことを確認しました。
RC、beta、nightlyは採用しません。型検査は `tsc`、変換・バンドルはViteが担当し、
TypeScriptコンパイラーのプログラマティックAPIには依存しません。
Vite 8と `@vitejs/plugin-react` 6のpeer dependencyが一致する構成です。

公式テンプレートのエントリーポイント、設定、ビルドスクリプト、アイコンは維持します。
今回利用しないopenerプラグインとサンプルロゴは削除します。
UIからRustへの呼び出しは `src/lib/api` の小さな関数に集約し、追加ライブラリや
Adapterクラス階層は導入しません。

## 今後

SQLiteなどの依存は実際の機能が必要になった時点で検討し、Rust側に追加します。
配布向けのバンドル識別子 `com.listingpilot.app` は公開前に所有する識別子へ見直してください。
