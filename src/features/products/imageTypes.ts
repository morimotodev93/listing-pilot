export type ProductImage = {
  id: string;
  product_id: string;
  original_filename: string;
  mime_type: string;
  file_size_bytes: number;
  width: number;
  height: number;
  sort_order: number;
  created_at: string;
  preview_url: string | null;
};

export type ReorderProductImagesInput = {
  productId: string;
  expectedImageIds: string[];
  orderedImageIds: string[];
};

export type ImportItem = {
  filename: string;
  status: "imported" | "duplicate" | "failed";
  message: string | null;
};
export type ImportReport = { cancelled: boolean; items: ImportItem[] };

export type DeleteImageInput = { productId: string; imageId: string };
export type DeleteImageReport = {
  deleted: true;
  affected_image_count: number;
  quarantined_image_count: number;
  missing_image_count: number;
  pending_quarantine_count: number;
};
