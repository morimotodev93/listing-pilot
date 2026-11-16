export type Product = {
  id: string; sku: string; name: string; brand: string | null;
  price_yen: number | null; description_note: string | null;
  listing_title: string | null; listing_description: string | null;
  created_at: string; updated_at: string;
};
export type ProductInput = Pick<Product, "sku" | "name" | "brand" | "price_yen" | "description_note">
  & Partial<Pick<Product, "listing_title" | "listing_description">>;

export type ProductAttributeValue = { attributeId: string; value: string };
export type AttributeValuesInput = {
  editableDefinitions: { attributeId: string; expectedUpdatedAt: string }[];
  values: ProductAttributeValue[];
};
export type ProductEditSnapshot = {
  product: Product;
  definitions: import("../attributes/types").AttributeDefinition[];
  values: ProductAttributeValue[];
};
