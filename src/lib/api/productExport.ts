import { request } from "./request";

export type ExportReport =
  | { status: "cancelled" }
  | { status: "success"; folder: string; product_count: number; attribute_count: number; image_count: number; has_warnings: boolean }
  | { status: "failed"; error: unknown; retained_folder: string; cleanup_failed: boolean };

export function exportProduct(productId: string, snapshotToken: string, warningToken: string | null): Promise<ExportReport> {
  return request("export_product", { input: { productId, snapshotToken, warningToken } });
}
