import { request } from "./request";
import type { Product, ProductInput } from "../../features/products/types";

export const listProducts = () => request<Product[]>("list_products");
export const getProduct = (id: string) => request<Product>("get_product", { id });
export const createProduct = (input: ProductInput) => request<Product>("create_product", { input });
export const updateProduct = (id: string, input: ProductInput) => request<Product>("update_product", { id, input });
