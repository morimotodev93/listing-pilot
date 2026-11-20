import type { GetImageUrlsInput, ImageUrlSnapshot, SaveImageUrlsInput } from "../../features/products/shopifyImageUrlTypes";
import { request } from "./request";

export function getShopifyImageUrls(input: GetImageUrlsInput): Promise<ImageUrlSnapshot> {
  return request("get_shopify_image_urls", { input });
}

export function saveShopifyImageUrls(input: SaveImageUrlsInput): Promise<ImageUrlSnapshot> {
  return request("save_shopify_image_urls", { input });
}
