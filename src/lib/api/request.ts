import { invoke, isTauri } from "@tauri-apps/api/core";

const messages: Record<string, string> = {
  required_field: "SKUと商品名を入力してください。",
  invalid_attribute_value: "カスタム値を確認してください。Textは2,000文字以内で改行・制御文字は使えません。Numberは整数15桁・小数6桁以内の半角数字で入力してください。",
  invalid_attribute_definition: "項目名は1〜80文字、単位は20文字以内で入力し、改行・制御文字は使わないでください。単位はNumber項目だけに設定できます。",
  duplicate_attribute_name: "この項目名はすでに登録されています。非表示の項目も確認し、別の名前を入力してください。",
  attribute_not_found: "項目が見つかりません。項目一覧を再読み込みして確認してください。",
  duplicate_sku: "このSKUはすでに登録されています。別のSKUを入力してください。",
  invalid_price: "価格は0〜9,007,199,254,740,991円の整数で入力してください。",
  product_not_found: "商品が見つかりません。一覧を再読み込みしてください。",
  database_operation: "データの読み書きに失敗しました。時間をおいて再度お試しください。",
  database_initialization: "データベースを初期化できませんでした。保存先のアクセス権や空き容量を確認し、アプリを再起動してください。",
  invalid_image: "画像が破損しているか、正しい画像ではありません。",
  unsupported_image: "JPEG・PNG・WebP形式の画像を選択してください。",
  image_too_large: "画像は1枚20MiB・4,000万画素以内にしてください。",
  too_many_images: "1回に選択できる画像は20枚までです。選び直してください。",
  duplicate_image: "同じ内容の画像がこの商品に登録済みのためスキップしました。",
  image_not_found: "画像が見つかりません。画像一覧を再読み込みしてください。",
  image_deletion_unknown: "画像の削除結果を確認できませんでした。ファイルは保護のため保持しています。アプリを再起動して状態を確認してください。",
  product_deletion_unknown: "商品の削除結果を確認できませんでした。管理画像は保護のため保持しています。アプリを再起動して商品と画像の状態を確認してください。",
  invalid_image_order: "画像の並び順が不正です。画像を再読み込みしてからお試しください。",
  stale_data: "商品・画像またはカスタム項目が変更・削除されています。最新の内容を読み込み、確認し直してください。",
  file_read: "画像ファイルを読み取れませんでした。アクセス権やファイルの状態を確認してください。",
  file_save: "画像を保存できませんでした。空き容量や保存先のアクセス権を確認してください。",
  unsafe_media_path: "安全性を確認できないファイルや保存先のため、画像の処理を中止しました。",
  media_busy: "別の処理または別のListing Pilotが保存先を使用中です。処理の完了を待つか、別のアプリを終了してから、このアプリを再起動してください。",
  media_initialization: "画像の保存先を初期化できませんでした。アクセス権を確認して再起動してください。",
};

export function errorMessage(error: unknown): string {
  const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "";
  return messages[code] ?? "処理に失敗しました。アプリを再起動してお試しください。";
}

export class RequestError extends Error {
  readonly code: string;
  constructor(error: unknown) {
    super(errorMessage(error));
    this.code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "";
  }
}

export async function request<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new Error("商品管理はデスクトップアプリで利用できます。");
  try { return await invoke<T>(command, args); }
  catch (error: unknown) { throw new RequestError(error); }
}
