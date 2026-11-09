import { invoke, isTauri } from "@tauri-apps/api/core";

// UIはこの窓口だけを使う。将来のWeb用Adapterも同じ関数シグネチャに合わせる。
export async function getGreeting(): Promise<string> {
  if (!isTauri()) {
    throw new Error("接続確認はデスクトップアプリで利用できます。");
  }

  return invoke<string>("greet");
}
