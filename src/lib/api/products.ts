import { request } from "./request";
import type { Product, ProductInput, AttributeValuesInput, ProductAttributeValue, ProductEditSnapshot } from "../../features/products/types";

export const listProducts = () => request<Product[]>("list_products");
export const getProduct = (id: string) => request<Product>("get_product", { id });
export const createProduct = (input: ProductInput, attributeValues?: AttributeValuesInput) => request<Product>("create_product", { input, attributeValues });
export const getProductEditSnapshot = (id: string) => request<ProductEditSnapshot>("get_product_edit_snapshot", { id });
export type UpdateProductRequest = { id: string; input: ProductInput; expectedUpdatedAt: string; attributeValues?: AttributeValuesInput };
export const updateProduct = (input: UpdateProductRequest) => request<Product>("update_product", input);

export type ProductDeletePreview = {
  product_id: string;
  name: string;
  sku: string;
  updated_at: string;
  image_count: number;
  image_ids: string[];
  product: Product;
  attribute_values: ProductAttributeValue[];
};
export type DeleteProductInput = {
  productId: string;
  expectedUpdatedAt: string;
  expectedImageIds: string[];
  expectedProduct: Product;
  expectedAttributeValues: ProductAttributeValue[];
};
export type DeleteProductReport = {
  deleted: true;
  affected_image_count: number;
  quarantined_image_count: number;
  missing_image_count: number;
  pending_quarantine_count: number;
};
export const getProductDeletePreview = (productId: string) => request<ProductDeletePreview>("get_product_delete_preview", { productId });
export const deleteProduct = (input: DeleteProductInput) => request<DeleteProductReport>("delete_product", { input });
