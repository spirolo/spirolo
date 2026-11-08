import { describe, it, expect } from "vitest";
import { http, HttpResponse } from "msw";
import { server } from "../../mocks/server";

describe("OrderList with MSW", () => {
  it("fetches orders through fetch and MSW", async () => {
    const response = await fetch("http://localhost:5000/v1/orders");
    expect(response.ok).toBe(true);
  });

  it("handles an empty list from MSW", async () => {
    server.use(http.get("*/v1/orders", () => HttpResponse.json({ items: [], total: 0 })));
    const response = await fetch("http://localhost:5000/v1/orders");
    expect(await response.json()).toEqual({ items: [], total: 0 });
  });
});
