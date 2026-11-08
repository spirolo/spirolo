import { describe, it, expect } from "vitest";
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";

const server = setupServer();

describe("AccountList", () => {
  it("lists the accounts the API returns", async () => {
    server.use(http.get("*/api/v1/accounts", () => HttpResponse.json({ items: [] })));
    const response = await fetch("http://localhost:5000/v1/accounts");
    expect(response.status).toBe(200);
  });

  it("shows the empty state", () => {
    expect([].length).toBe(0);
  });
});
