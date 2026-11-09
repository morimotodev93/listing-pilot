export type Product = {
  id: string; sku: string; name: string; brand: string | null;
  price_yen: number | null; description_note: string | null;
  created_at: string; updated_at: string;
};
export type ProductInput = Pick<Product, "sku" | "name" | "brand" | "price_yen" | "description_note">;
