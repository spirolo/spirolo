import { BASE } from "./shared";

export const urlApi = BASE + "/v1";
export const urlV2 = `${BASE}/v2`;
export const dynamic = process.env.API_URL + "/fallback";
export const twice = `${BASE}-${BASE}`;
