# ADR 0003: Content Security Policy（CSP）の採用

- **ステータス:** 採用（実装後のWindows実機検証を完了条件とする）
- **決定日:** 2026-11-09
- **対象:** Listing Pilot（Tauri 2 / React / Vite / Windows）
- **関連:** `src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`

## 1. 背景

Listing Pilotはローカル中心のデスクトップアプリであり、WebView上にReact UIを表示する。今後、CSV由来の商品情報、画像、AI生成文章など、必ずしも信頼できない入力を扱う。万一HTMLやスクリプトの注入が発生した場合に備え、WebViewが読み込めるリソースを制限する。

現状は `app.security.csp: null` で、TauriのCSP保護が有効化されていない。Tauri公式ドキュメントは、できるだけ制限的なCSPを構成し、同梱資産に対するハッシュ・nonceの自動付加を利用することを推奨している。

## 2. 決定

**制限的な許可リスト型CSPを、本番ビルドで有効化する。** 開発時は `devCsp` を別途設定し、Viteの開発サーバーとHMRに必要なローカル通信だけを追加で許可する。Tauriがビルド時に行うCSPへのハッシュ・nonce注入を無効化しない。

### 本番ビルド：`app.security.csp`

| ディレクティブ | 設定 | 意図 |
| --- | --- | --- |
| `default-src` | `'self'` | 既定の読み込み元をアプリ自身に限定 |
| `script-src` | `'self'` | 同梱スクリプトのみ許可。`unsafe-inline`・`unsafe-eval`は許可しない |
| `style-src` | `'self'` | 同梱CSSのみ許可 |
| `img-src` | `'self' data: blob:` | 同梱画像と将来のローカルプレビュー用URLを許可 |
| `font-src` | `'self'` | 外部フォント配信を許可しない |
| `connect-src` | `ipc: http://ipc.localhost` | Tauri IPCに必要な通信のみ許可 |
| `object-src` | `'none'` | 埋め込みオブジェクトを禁止 |
| `base-uri` | `'none'` | HTMLのbase URLの変更を禁止 |
| `frame-src` | `'none'` | iframe等を禁止 |
| `form-action` | `'none'` | WebViewからのネイティブなフォーム送信を禁止（商品フォームはReactが処理） |

この設定は、nonce・ハッシュだけでスクリプトを許可する厳密な意味での *Strict CSP* ではない。Tauriの自動付加を生かしつつ、現在のアプリの必要機能に合わせて制限する方針である。

### 開発ビルド：`app.security.devCsp`

本番設定と同じ基本制限を維持しつつ、次だけ追加する。

- `script-src` と `style-src` に開発時限定で `'unsafe-inline'` を追加する。Vite / React Refreshの開発時注入を動作させるための例外とし、本番には持ち込まない。
- `connect-src` に `'self'`、`ws://localhost:1420`、`ws://127.0.0.1:1420` を追加する。現在のVite開発ポートは1420である。
- `unsafe-eval`、ワイルドカード（`*`）、任意の外部CDN・APIドメインは追加しない。

`TAURI_DEV_HOST`などにより開発用ホストやHMRポートを変更する場合は、その環境で必要な接続先のみを改めて許可する。

## 3. 現在は許可しないもの

- 外部JavaScript、CDN配信のCSS・フォント、外部画像ドメイン。
- WebViewからの直接AI API通信。将来のAI API通信はRust側で実施し、APIキーをフロントエンドに置かない。
- TauriのAsset Protocol。Phase 1では不要なため、`assetProtocol.enable`の既定値 `false` を維持する。
- 広範なファイルシステム権限やTauriプラグイン権限。これらはCSPとは別にCapabilitiesで最小化する。

## 4. Phase 2（商品画像）への変更条件

画像プレビューはまず `blob:` / `data:` で足りるか検証する。ディスク上の画像を `convertFileSrc()` で直接表示する方式を採る場合に限り、以下を**同時に**実施する。

1. `assetProtocol.enable` を有効化する。
2. `assetProtocol.scope` を、Listing Pilotが管理する画像用ディレクトリのみに限定する（例：`$APPLOCALDATA/media/**/*`。実際の保存レイアウトを確定後に調整）。ユーザーのホーム全体や `**/*` は許可しない。
3. 必要に応じて `img-src` に `asset:` と `http://asset.localhost` を追加する。他のディレクティブには追加しない。
4. Windows WebView2で通常画像の表示とスコープ外ファイルの読み込み拒否を確認する。

なお、CSPの`img-src`に許可を追加するだけでは、ディスク上のファイルを安全に公開できない。Asset Protocolのscope設定が別途必要である。

## 5. 検証・完了条件

- `npm run build` が成功する。
- `cargo check --manifest-path src-tauri/Cargo.toml` が成功する。
- `npm run tauri dev` で、画面表示、商品一覧、登録・編集、Rust接続確認、Vite HMRが動作する。
- `npm run tauri build` が成功し、実際のWindows配布ビルドで主要画面・Tauri Commandsが動作する。
- 開発者ツールでCSP違反を確認し、必要機能のブロックがない。確認した違反・例外は `docs/verification.md` に記録する。
- 本番設定に `unsafe-inline`、`unsafe-eval`、`*`、不要な外部ドメインを追加しない。
- 最終生成物のCSPを確認し、Tauriによるハッシュ・nonce付加を無効化していないことを確認する。

※ このADRと設定案だけでは、実行時の互換性は証明できない。特に`devCsp`は開発用、`csp`は配布用の双方を確認して確定する。

## 6. トレードオフ

**利点:** XSS時の追加的な防御、外部スクリプトの意図しない読み込み防止、将来の権限追加の理由を明示できる。

**コスト:** Viteの開発用通信や画像プレビューなど、新しい資産読み込み方式を追加するたびにCSPと実機検証を更新する必要がある。CSPだけで入力の安全性やRust側のアクセス制御が保証されるわけではない。

**不採用:** `csp: null` の継続、`default-src *` のような包括許可、必要性を検証せず `unsafe-eval` / 本番`unsafe-inline` を追加する方式。

## 7. 将来のWebデモ

Vercel等で公開するWebデモには、TauriのCSP設定は適用されない。Webデモの配信サーバー側でHTTPレスポンスヘッダーなどによる別個のCSPを設定する。

## 8. 参考資料

- Tauri公式：<https://v2.tauri.app/security/csp/>
- Tauri公式・設定リファレンス：<https://v2.tauri.app/reference/config/>
- Tauri公式・Asset Protocol Scope：<https://v2.tauri.app/security/asset-protocol/>
- MDN・Content Security Policy：<https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP>
