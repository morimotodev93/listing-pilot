import { convertFileSrc } from "@tauri-apps/api/core";
import type { ListingPreview, ListingPreviewImage } from "../../features/products/listingPreviewTypes";
import { request } from "./request";

type Response = Omit<ListingPreview, "images"> & {
  images: (Omit<ListingPreviewImage, "previewUrl"> & { previewPath: string | null })[];
};

// Share only in-flight reads (including StrictMode's initial effect replay).
// Completed snapshots are never cached; refresh always reads current backend data.
const pending = new Map<string, Promise<ListingPreview>>();

export function getListingPreview(productId: string): Promise<ListingPreview> {
  const current = pending.get(productId);
  if (current) return current;
  const operation = fetchPreview(productId).finally(() => pending.delete(productId));
  pending.set(productId, operation);
  return operation;
}

async function fetchPreview(productId: string): Promise<ListingPreview> {
  const result = await request<Response>("get_listing_preview", { productId });
  return { ...result, images: result.images.map(({ previewPath, ...image }) => ({
    ...image, previewUrl: previewPath ? convertFileSrc(previewPath) : null,
  })) };
}
