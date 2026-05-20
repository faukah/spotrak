import { ApiError } from "../api/client";

export function isUnauthorized(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401;
}

export function redirectToLogin(): void {
  if (typeof window === "undefined") return;
  if (
    window.location.pathname === "/login" ||
    window.location.pathname.startsWith("/public/")
  ) return;
  const next =
    `${window.location.pathname}${window.location.search}${window.location.hash}`;
  window.location.href = `/login?next=${encodeURIComponent(next)}`;
}
