export async function waitForNangoConnection(
  providerName: string,
  integrationId: string,
  headers: Record<string, string>,
  signal?: AbortSignal,
) {
  const client = createClient({ baseUrl: env.VITE_API_URL, headers });
  const deadline = Date.now() + NANGO_CONNECTION_TIMEOUT_MS;

  while (true) {
    throwIfConnectionCancelled(signal);
    const { data, error } = await listConnections({ client });
    if (error) throw new Error("Failed to load integrations");
    const connection = data?.connections.find(
      (item) =>
        item.integration_id === integrationId && nangoConnectionIsReady(item),
    );
    if (connection) return connection;
    if (Date.now() >= deadline) {
      throw new Error(`${providerName} sign-in timed out. Try again.`);
    }
    await sleep(NANGO_CONNECTION_POLL_MS, signal);
  }
}

