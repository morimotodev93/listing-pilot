import { useState } from "react";
import { getGreeting } from "../../lib/api";

type ConnectionState = {
  status: "idle" | "loading" | "success" | "error";
  message: string;
};

export function ConnectionCheck() {
  const [connection, setConnection] = useState<ConnectionState>({
    status: "idle",
    message: "ボタンを押して接続を確認してください。",
  });
  const isLoading = connection.status === "loading";

  async function checkConnection() {
    setConnection({ status: "loading", message: "接続を確認しています…" });

    try {
      const message = await getGreeting();
      setConnection({ status: "success", message });
    } catch (error: unknown) {
      setConnection({
        status: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }

  return (
    <section className="connection" aria-labelledby="connection-heading">
      <h2 id="connection-heading">はじめる準備</h2>
      <button type="button" disabled={isLoading} onClick={checkConnection}>
        {isLoading ? "確認中…" : "接続を確認"}
      </button>
      <p className="status" role="status" data-state={connection.status}>
        {connection.message}
      </p>
    </section>
  );
}
