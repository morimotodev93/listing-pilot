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

export type ImportItem = {
  filename: string;
  status: "imported" | "duplicate" | "failed";
  message: string | null;
};
export type ImportReport = { cancelled: boolean; items: ImportItem[] };
