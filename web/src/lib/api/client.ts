// oxlint-disable typescript/no-unsafe-type-assertion -- API JSON boundaries are typed by callers/OpenAPI.
import type { ApiErrorBody } from "./types";

export const API_ENDPOINT =
  (import.meta.env.PUBLIC_API_ENDPOINT ?? "http://127.0.0.1:8080").replace(
    /\/$/,
    "",
  );

const GET_CACHE_TTL_MS = 60_000;
const GET_CACHE_MAX_ENTRIES = 200;
const getCache = new Map<string, { expiresAt: number; value: unknown }>();
const inFlightGets = new Map<string, Promise<unknown>>();

function shouldCacheGet(path: string, init: RequestInit): boolean {
  const method = (init.method ?? "GET").toUpperCase();
  if (method !== "GET" || init.body || init.cache === "no-store") return false;
  return (
    path === "/auth/me" ||
    path === "/users/me" ||
    path.startsWith("/stats/") ||
    (path.startsWith("/public/") && path.includes("/stats/")) ||
    path.startsWith("/tracks/") ||
    path.startsWith("/artists/") ||
    path.startsWith("/albums/") ||
    path.startsWith("/history") ||
    path.startsWith("/search")
  );
}

export function clearApiCache(): void {
  getCache.clear();
  inFlightGets.clear();
}

export class ApiError extends Error {
  code: string;
  details: unknown[];
  status: number;

  constructor(status: number, body: ApiErrorBody) {
    super(body.error.message);
    this.name = "ApiError";
    this.status = status;
    this.code = body.error.code;
    this.details = body.error.details ?? [];
  }
}

export async function apiFetch<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const method = (init.method ?? "GET").toUpperCase();
  const cacheable = shouldCacheGet(path, init);
  if (cacheable) {
    const cached = getCache.get(path);
    if (cached && cached.expiresAt > Date.now()) {
      getCache.delete(path);
      getCache.set(path, cached);
      return cached.value as T;
    }

    const inFlight = inFlightGets.get(path);
    if (inFlight) return inFlight as Promise<T>;

    const request = apiFetchInner<T>(path, init).then((value) => {
      setCachedGet(path, value);
      return value;
    });
    inFlightGets.set(path, request);
    try {
      return await request;
    } finally {
      inFlightGets.delete(path);
    }
  }

  const value = await apiFetchInner<T>(path, init);
  if (method !== "GET") clearApiCache();
  return value;
}

async function apiFetchInner<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");

  if (init.body && !(init.body instanceof FormData)) {
    headers.set("Content-Type", "application/json");
  }

  const method = (init.method ?? "GET").toUpperCase();
  if (!["GET", "HEAD", "OPTIONS"].includes(method)) {
    headers.set("X-Spotrak-CSRF", "1");
  }

  const response = await fetch(`${API_ENDPOINT}/api/v1${path}`, {
    credentials: "include",
    ...init,
    headers,
  });

  if (response.status === 204) {
    return undefined as T;
  }

  const contentType = response.headers.get("content-type") ?? "";
  const body = contentType.includes("application/json")
    ? await response.json()
    : await response.text();

  if (!response.ok) {
    if (typeof body === "object" && body && "error" in body) {
      const error = new ApiError(response.status, body as ApiErrorBody);
      if (response.status === 401 && typeof window !== "undefined") {
        window.dispatchEvent(
          new CustomEvent("spotrak:unauthorized", { detail: error }),
        );
      }
      throw error;
    }
    throw new Error(typeof body === "string" ? body : response.statusText);
  }

  return body as T;
}

function setCachedGet(path: string, value: unknown): void {
  getCache.set(path, { value, expiresAt: Date.now() + GET_CACHE_TTL_MS });
  while (getCache.size > GET_CACHE_MAX_ENTRIES) {
    const oldest = getCache.keys().next().value;
    if (!oldest) break;
    getCache.delete(oldest);
  }
}

export function loginUrl(): string {
  return `${API_ENDPOINT}/api/v1/auth/spotify/start`;
}
