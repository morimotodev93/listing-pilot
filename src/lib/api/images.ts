import { convertFileSrc } from "@tauri-apps/api/core";
import type { ImportItem, ImportReport, ProductImage, ReorderProductImagesInput } from "../../features/products/imageTypes";
import { errorMessage, request } from "./request";

type ImageResponse = Omit<ProductImage, "preview_url"> & { preview_path: string | null };
type ImportResponse = { cancelled: boolean; items: (Omit<ImportItem, "message"> & { error: { code: string } | null })[] };

export async function listProductImages(productId: string): Promise<ProductImage[]> {
  const images = await request<ImageResponse[]>("list_product_images", { productId });
  return withPreviewUrls(images);
}

export async function reorderProductImages(input: ReorderProductImagesInput): Promise<ProductImage[]> {
  return withPreviewUrls(await request<ImageResponse[]>("reorder_product_images", { ...input }));
}

function withPreviewUrls(images: ImageResponse[]): ProductImage[] {
  return images.map(({ preview_path, ...image }) => ({
    ...image, preview_url: preview_path ? convertFileSrc(preview_path) : null,
  }));
}

export async function importProductImages(productId: string): Promise<ImportReport> {
  const report = await request<ImportResponse>("import_product_images", { productId });
  return { cancelled: report.cancelled, items: report.items.map(({ error, ...item }) => ({
    ...item, message: error ? errorMessage(error) : null,
  })) };
}
