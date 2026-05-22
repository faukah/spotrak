// oxlint-disable typescript/no-unsafe-type-assertion -- Runtime fetch payloads cross the JSON boundary here.
import type { paths } from "./generated";
import type { ApiErrorBody } from "./types";

type ApiBasePath = keyof paths & `/api/v1${string}`;
type ClientPath = ApiBasePath extends `/api/v1${infer Path}` ? Path : never;
type ClientPathWithQuery = ClientPath | `${ClientPath}?${string}`;
type StripQuery<Path extends string> = Path extends `${infer Base}?${string}`
  ? Base
  : Path;
type FullPath<Path extends string> = `/api/v1${StripQuery<Path>}`;
type MethodName =
  | "delete"
  | "get"
  | "head"
  | "options"
  | "patch"
  | "post"
  | "put"
  | "trace";
type MethodFromInit<Init> = Init extends { method: infer Method }
  ? Lowercase<Extract<Method, string>> extends MethodName
    ? Lowercase<Extract<Method, string>>
    : "get"
  : "get";
type OperationFor<Path extends ClientPathWithQuery, Init> =
  FullPath<Path> extends ApiBasePath
    ? paths[FullPath<Path>][MethodFromInit<Init>]
    : never;
type JsonResponse<Operation> = Operation extends {
  responses: infer Responses;
}
  ? 200 extends keyof Responses
    ? Responses[200] extends {
        content: { "application/json": infer Body };
      }
      ? Body
      : void
    : 204 extends keyof Responses
      ? void
      : unknown
  : unknown;
export type ApiFetchResponse<
  Path extends ClientPathWithQuery,
  Init extends RequestInit = RequestInit,
> = JsonResponse<OperationFor<Path, Init>>;

export const API_ENDPOINT =
  (import.meta.env.PUBLIC_API_ENDPOINT ?? "http://127.0.0.1:8080").replace(
    /\/$/,
    "",
  );

const GET_CACHE_TTL_MS = 60_000;
const GET_CACHE_MAX_ENTRIES = 200;
const READ_ONLY_METHODS = new Set(["GET", "HEAD", "OPTIONS"]);
const getCache = new Map<string, { expiresAt: number; value: unknown }>();
const inFlightGets = new Map<string, Promise<unknown>>();

function shouldCacheGet(path: string, init: RequestInit): boolean {
  const method = (init.method ?? "GET").toUpperCase();
  if (method !== "GET" || init.body || init.cache === "no-store") return false;
  return (
    path === "/auth/me" ||
    path === "/users/me" ||
    path.startsWith("/stats/") ||
    path.startsWith("/public/") ||
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

export async function apiFetch<
  Path extends ClientPathWithQuery,
  Init extends RequestInit = RequestInit,
>(path: Path, init?: Init): Promise<ApiFetchResponse<Path, Init>>;
export async function apiFetch<T>(
  path: string,
  init?: RequestInit,
): Promise<T>;
export async function apiFetch<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const method = (init.method ?? "GET").toUpperCase();
  enforcePublicPageReadOnly(path, method);
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

function enforcePublicPageReadOnly(path: string, method: string): void {
  if (typeof window === "undefined") return;
  if (!window.location.pathname.startsWith("/public/")) return;
  if (!READ_ONLY_METHODS.has(method)) {
    throw new Error("Public shared pages cannot make write requests.");
  }
  if (!path.startsWith("/public/")) {
    throw new Error("Public shared pages can only call public read APIs.");
  }
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

export function loginUrl(next?: string | null): string {
  const url = new URL(`${API_ENDPOINT}/api/v1/auth/spotify/start`);
  if (next) url.searchParams.set('next', next);
  return url.toString();
}
