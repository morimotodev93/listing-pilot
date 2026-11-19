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
