import { useCallback, useEffect, useRef, useState } from "react";
import { listProducts } from "../../../lib/api/products";
import type { Product } from "../types";

export function useProducts() {
  const [products, setProducts] = useState<Product[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const generation = useRef(0);
  const refresh = useCallback(async () => {
    const current = ++generation.current;
    setLoading(true); setError("");
    try {
      const result = await listProducts();
      if (current === generation.current) setProducts(result);
    } catch (error: unknown) {
      if (current === generation.current) setError(error instanceof Error ? error.message : "商品を取得できませんでした。");
    } finally { if (current === generation.current) setLoading(false); }
  }, []);
  useEffect(() => { void refresh(); return () => { generation.current++; }; }, [refresh]);
  return { products, loading, error, refresh };
}
