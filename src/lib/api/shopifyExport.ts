import { request } from "./request";
import type { ExportReport } from "./productExport";

export type ShopifyPreview = {
  snapshotToken: string;
  products: {
    productId: string;
    name: string;
    sku: string;
    title: string;
    description: string | null;
    vendor: string | null;
    priceYen: number | null;
    handle: string;
    status: "ready" | "warning" | "blocked";
    issues: { code: string; severity: "warning" | "error"; message: string }[];
  }[];
  hasWarnings: boolean;
  blocked: boolean;
};

export function getShopifyExportPreview(productIds: string[]): Promise<ShopifyPreview> {
  return request("get_shopify_export_preview", { input: { productIds } });
}

export function exportShopifyProducts(productIds: string[], snapshotToken: string, warningToken: string | null): Promise<ExportReport> {
  return request("export_shopify_products", { input: { productIds, snapshotToken, warningToken } });
}

export type ShopifyImagePreview = {
  storeKey: string;
  products: (ShopifyPreview["products"][number] & {
    imageCount: number;
    urlConfiguredCount: number;
    mainImageId: string | null;
    images: { imageId: string; position: number; urlConfigured: boolean; altConfigured: boolean; issues: string[] }[];
    imageIssues: { code: string; imageId: string | null; position: number | null; message: string }[];
  })[];
  imageCount: number;
  snapshotToken: string;
  hasWarnings: boolean;
  blocked: boolean;
};

export function getShopifyImageExportPreview(productIds: string[], storeKey: string): Promise<ShopifyImagePreview> {
  return request("get_shopify_image_export_preview", { input: { productIds, storeKey } });
}

export function exportShopifyProductsWithImages(productIds: string[], storeKey: string, snapshotToken: string, warningToken: string | null): Promise<ExportReport> {
  return request("export_shopify_products_with_images", { input: { productIds, storeKey, snapshotToken, warningToken } });
}
