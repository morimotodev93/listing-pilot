export type ListingPreviewImage = { imageId: string; sortOrder: number; previewUrl: string | null };
export type ListingPreview = {
  productId: string;
  sku: string;
  title: string;
  titleSource: "listing_title" | "product_name";
  description: string | null;
  priceYen: number | null;
  attributes: { attributeId: string; name: string; value: string; unit: string | null }[];
  images: ListingPreviewImage[];
  status: "ready" | "warning" | "blocked";
  issues: { code: string; severity: "warning" | "error"; message: string }[];
};
