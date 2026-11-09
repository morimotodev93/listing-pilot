import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Product, ProductInput } from "../../features/products/types";

const messages: Record<string, string> = {
  required_field: "SKUと商品名を入力してください。",
  duplicate_sku: "このSKUはすでに登録されています。別のSKUを入力してください。",
  invalid_price: "価格は0〜9,007,199,254,740,991円の整数で入力してください。",
  product_not_found: "商品が見つかりません。一覧を再読み込みしてください。",
  database_operation: "データの読み書きに失敗しました。時間をおいて再度お試しください。",
  database_initialization: "データベースを初期化できませんでした。保存先のアクセス権や空き容量を確認し、アプリを再起動してください。",
};
async function request<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new Error("商品管理はデスクトップアプリで利用できます。");
  try { return await invoke<T>(command, args); }
  catch (error: unknown) {
    const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "";
    throw new Error(messages[code] ?? "処理に失敗しました。アプリを再起動してお試しください。");
  }
}
export const listProducts = () => request<Product[]>("list_products");
export const getProduct = (id: string) => request<Product>("get_product", { id });
export const createProduct = (input: ProductInput) => request<Product>("create_product", { input });
export const updateProduct = (id: string, input: ProductInput) => request<Product>("update_product", { id, input });
