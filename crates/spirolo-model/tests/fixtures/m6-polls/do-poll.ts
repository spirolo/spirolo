async function pollUntil<T>(
  read: () => Promise<T>,
  predicate: (value: T) => boolean,
  description: string,
  timeoutMs = EVENT_TIMEOUT_MS
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  let last: T;
  do {
    last = await read();
    if (predicate(last)) {
      return last;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  } while (Date.now() < deadline);
  throw new Error(`Timed out after ${timeoutMs}ms waiting for ${description}`);
}

