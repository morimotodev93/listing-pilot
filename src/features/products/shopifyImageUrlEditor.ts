import type { ProductImage } from "./imageTypes";
import type { ImageUrlEdit, ImageUrlSnapshot } from "./shopifyImageUrlTypes";

const controls = /[\u0000-\u001f\u007f-\u009f\u2028\u2029]/u;
const encoder = new TextEncoder();

export function scalarCount(value: string): number {
  return Array.from(value).length;
}

export function urlBytes(value: string): number {
  return encoder.encode(value).length;
}

// Supplementary checks only. Rust remains the authority for all saved values.
export function urlIssue(value: string): string {
  if (value === "") return "";
  if (urlBytes(value) > 4096) return "URLは4,096 UTF-8バイト以内で入力してください。";
  if (controls.test(value) || /[\s\\#]/u.test(value)
    || /%(?![0-9a-f]{2})/i.test(value)
    || /%(?:0[0-9a-f]|1[0-9a-f]|7f|c2%(?:8[0-9a-f]|9[0-9a-f]))/i.test(value)) {
    return "URLに空白・制御文字・不正な形式を含めないでください。";
  }
  if (!/^https:\/\/cdn\.shopify\.com(?::443)?(?:[/?]|$)/i.test(value)) {
    return "https://cdn.shopify.com/ の公開画像URLを入力してください。";
  }
  try {
    const url = new URL(value);
    if (url.protocol !== "https:" || url.hostname !== "cdn.shopify.com" || url.username || url.password || url.port || url.hash) {
      return "Shopify CDNの公開HTTPS URLだけを設定できます。";
    }
    if (urlBytes(url.href) > 4096) return "URL解析後の長さが4,096 UTF-8バイトを超えます。";
    return "";
  } catch {
    return "URLの形式を確認してください。";
  }
}

export function altIssue(value: string): string {
  if (scalarCount(value) > 512) return "altテキストは512文字以内で入力してください。";
  if (controls.test(value)) return "altテキストは単一行で入力してください。改行・タブ・制御文字は使えません。";
  for (const character of value) {
    const point = character.codePointAt(0)!;
    if (point >= 0xd800 && point <= 0xdfff) return "altテキストに使用できない文字が含まれます。";
  }
  if (/^[\s\u0000-\u001f\u007f-\u009f\ufeff\u200b-\u200f\u202a-\u202e\u2060-\u206f]*[=+@-]/u.test(value)) {
    return "CSVで数式として扱われる先頭文字（= + - @）は使えません。";
  }
  return "";
}

export function editIssues(edits: ImageUrlEdit[]): Map<string, { url: string; alt: string }> {
  const issues = new Map(edits.map(edit => [edit.imageId, {
    url: urlIssue(edit.url),
    alt: altIssue(edit.altText) || (!edit.url && edit.altText ? "URL未設定ではaltだけを保存できません。解除操作で両方を空にしてください。" : ""),
  }]));
  const seen = new Map<string, string>();
  for (const edit of edits) {
    if (!edit.url || issues.get(edit.imageId)!.url) continue;
    const key = new URL(edit.url).href;
    const previous = seen.get(key);
    if (previous) {
      issues.get(previous)!.url = "同じストア・商品の他画像とURLが重複しています。";
      issues.get(edit.imageId)!.url = "同じストア・商品の他画像とURLが重複しています。";
    } else seen.set(key, edit.imageId);
  }
  return issues;
}

export function snapshotMatches(snapshot: ImageUrlSnapshot, images: ProductImage[], productId: string, store: string): boolean {
  if (snapshot.productId !== productId || snapshot.platform !== "shopify"
    || snapshot.storeKey !== store.trim().toLowerCase() || snapshot.images.length !== images.length) return false;
  const locals = new Map(images.map(image => [image.id, image]));
  const ids = new Set(snapshot.images.map(row => row.imageId));
  if (locals.size !== images.length || ids.size !== images.length) return false;
  return snapshot.images.every((row, index) => {
    const image = locals.get(row.imageId);
    return image?.product_id === productId && row.imageId === images[index].id
      && row.sortOrder === image.sort_order && row.sha256 === image.sha256
      && (!row.setting || (row.setting.imageId === row.imageId
        && row.setting.platform === snapshot.platform && row.setting.storeKey === snapshot.storeKey));
  });
}

export function snapshotEdits(snapshot: ImageUrlSnapshot): Record<string, ImageUrlEdit> {
  return Object.fromEntries(snapshot.images.map(row => [row.imageId, {
    imageId: row.imageId, url: row.setting?.url ?? "", altText: row.setting?.altText ?? "",
  }]));
}

// Preserve every empty line, including a trailing newline: each is an image slot.
export function pasteLines(text: string): string[] {
  return text.split(/\r\n|\r|\n/);
}
