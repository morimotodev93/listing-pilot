export type ImageUrlSetting = {
  imageId: string;
  platform: string;
  storeKey: string;
  url: string;
  imageSha256: string;
  altText: string;
  updatedAt: string;
};

export type ImageUrlRow = {
  imageId: string;
  sortOrder: number;
  sha256: string;
  setting: ImageUrlSetting | null;
};

export type ImageUrlSnapshot = {
  productId: string;
  platform: "shopify";
  storeKey: string;
  snapshotToken: string;
  images: ImageUrlRow[];
};

export type GetImageUrlsInput = { productId: string; storeKey: string };
export type ImageUrlEdit = { imageId: string; url: string; altText: string };
export type SaveImageUrlsInput = GetImageUrlsInput & {
  snapshotToken: string;
  images: ImageUrlEdit[];
};
