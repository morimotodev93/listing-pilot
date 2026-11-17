import { request } from "./request";
import type { ListingPreview } from "../../features/products/listingPreviewTypes";

export type BatchPreview = {
  snapshotToken: string;
  products: { name: string; groupId: string; groupName: string; preview: Omit<ListingPreview, "images"> & { images: { imageId: string; sortOrder: number; previewPath: string | null }[] } }[];
  imageCount: number;
  hasWarnings: boolean;
  blocked: boolean;
};

export function getExportBatchPreview(productIds: string[]): Promise<BatchPreview> {
  return request("get_export_batch_preview", { input: { productIds } });
}

export function exportProducts(productIds: string[], snapshotToken: string, warningToken: string | null): Promise<ExportReport> {
  return request("export_products", { input: { productIds, snapshotToken, warningToken } });
}

export type ExportReport =
  | { status: "cancelled" }
  | { status: "success"; folder: string; product_count: number; attribute_count: number; image_count: number; has_warnings: boolean }
  | { status: "failed"; error: unknown; retained_folder: string; cleanup_failed: boolean };

export function exportProduct(productId: string, snapshotToken: string, warningToken: string | null): Promise<ExportReport> {
  return request("export_product", { input: { productId, snapshotToken, warningToken } });
}
