import { http, HttpResponse } from "msw";

const BASE = "*/api/v1";

export const orderHandlers = [
  http.get(`${BASE}/orders`, ({ request }) => {
    const url = new URL(request.url);
    const page = Number(url.searchParams.get("page") ?? 0);
    return HttpResponse.json({ items: [], total: 0, page });
  }),
];
