import { ConnectionCheck } from "../features/connection/ConnectionCheck";
import "./App.css";
import { ProductList } from "../features/products/components/ProductList";
import { AttributeManager } from "../features/attributes/components/AttributeManager";
import "../features/products/products.css";

export default function App() {
  return (
    <main className="app">
      <header>
        <span className="brand-mark" aria-hidden="true">LP</span>
        <h1>Listing Pilot</h1>
        <p className="tagline">写真から、出品準備完了まで。</p>
      </header>
      <ProductList />
      <details className="attribute-settings panel"><summary>カスタム項目設定</summary><AttributeManager /></details>
      <details className="connection-details"><summary>Rustとの接続確認</summary><ConnectionCheck /></details>
    </main>
  );
}
