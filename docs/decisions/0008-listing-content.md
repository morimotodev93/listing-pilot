# 0008: 公開用文章の保存と内部管理情報の分離

- 日付：2026-11-12（日本時間）
- 状態：採用・Phase 4.3実装

## 保存する値

productsに任意のlisting_titleとlisting_descriptionを追加する。管理用nameとdescription_noteとは
独立した値であり、自動コピーや同期は行わない。公開用説明文はプレーンテキストとして保存し、
Reactのテキスト表示とpre-wrapで改行を保持する。HTMLとして解釈しない。

公開用タイトルはRustのtrim後200 Unicodeスカラー値以内。trim前に制御文字とUnicode行・段落区切りを拒否する。
公開用説明文はCRLFをLFに変換し、LFとタブ以外の制御文字（単独CRを含む）をtrim前に拒否する。
trim後10,000 Unicodeスカラー値以内とし、本文中のLF・タブは保持する。両方とも空欄・空白のみはNULL。
Unicode正規化・全角半角変換は行わない。

公開用タイトルがNULLでもnameを保存値・フォーム初期値・詳細の代替表示には使用しない。
将来の出品プレビュー・CSV出力層でnameを出力候補とし、利用時に警告する予定。
description_noteを未設定の公開用説明文の代替にはしない。今回プレビュー・CSV・自動公開は実装しない。

## 入力契約と保存の原子性

IPCの商品inputは既存のsnake_caseを維持する。新しい2フィールドは小さな専用型ListingTextInputで受け取る。
Serdeのdefaultで省略をOmitted、フィールドが存在する場合はProvided(Option<String>)とし、
明示的nullと文字列を区別する。数値・配列・オブジェクトなどはデシリアライズで拒否する。

- 作成時の省略：NULL。
- 更新時の省略：現在の保存値を保持。
- 明示的null：そのフィールドだけ解除。
- 文字列：Rustで検証・正規化して保存。

Productは確定したOption<String>を保持する。更新はIMMEDIATE内で現在の商品を取得して省略値を解決し、
既存のid＋expectedUpdatedAtを条件とするSQLで確定する。公開文章だけの変更でも更新日時を最低1ns進める。
基本情報・公開文章・対象カスタム値は既存の同一トランザクションで保存し、失敗時はすべてロールバックする。
フォームは両方の入力値を明示して既存の保存ボタンで送信する。競合・保存失敗時は入力を保持する。

商品削除プレビューのProductにも2フィールドを含める。既存のPartialEqによる事前・IMMEDIATE内の完全比較で、
更新日時が変わらない公開文章変更も検出する。画像隔離、permit、COMMIT結果不明時の安全側処理は変更しない。

## Migration 0004

0004_add_listing_content.sqlでnullable TEXT列を2つALTER TABLE追加し、スキーマを4にする。
既存0001〜0003は変更しない。全段階のDDLとuser_version更新を既存の単一IMMEDIATEで実行し、
途中失敗時はまとめてロールバックする。0〜3は順次移行、4は再適用しても変更なし、5以上は拒否する。

既存の内部説明メモと同様に、新列はnullable TEXTのみとする。長さ・制御文字・改行変換・Unicode空白の
一貫した検証はRust境界に集約する。SQLiteのlengthやtrimによる不完全な重複検証は追加しない。
商品・画像・共通定義・商品別値を保持し、通常保存先・識別子・CSP・権限・Asset Protocolは変更しない。
検証用JSONデシリアライズには既存の依存グラフ内のserde_jsonをdev-dependenciesとして直接利用する。

検証結果と未検証事項は [検証記録](../verification.md) を参照。
