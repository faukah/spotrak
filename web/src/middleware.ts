import { defineMiddleware } from "astro:middleware";
import { SERVER_API_ENDPOINT } from "./lib/api/server";

const PUBLIC_FILES = new Set([
  "/favicon.ico",
  "/robots.txt",
  "/manifest.webmanifest",
]);
type SessionStatus = "valid" | "invalid" | "unavailable";

export const onRequest = defineMiddleware(async (context, next) => {
  const { pathname } = context.url;

  if (!["GET", "HEAD"].includes(context.request.method)) return next();
  if (isPublicPath(pathname)) return next();

  const session = await sessionStatus(context.request);
  if (session === "valid") return next();
  if (session === "unavailable") {
    return new Response("Authentication service unavailable", { status: 503 });
  }

  const returnTo =
    `${context.url.pathname}${context.url.search}${context.url.hash}`;
  return context.redirect(`/login?next=${encodeURIComponent(returnTo)}`, 302);
});

function isPublicPath(pathname: string): boolean {
  return (
    PUBLIC_FILES.has(pathname) ||
    pathname === "/login" ||
    pathname.startsWith("/login/") ||
    pathname === "/public" ||
    pathname.startsWith("/public/") ||
    pathname.startsWith("/_astro/")
  );
}

async function sessionStatus(request: Request): Promise<SessionStatus> {
  const cookie = request.headers.get("cookie");
  if (!cookie) return "invalid";

  try {
    const response = await fetch(`${SERVER_API_ENDPOINT}/api/v1/auth/me`, {
      headers: { Accept: "application/json", cookie },
    });
    if (response.ok) return "valid";
    if (response.status === 401 || response.status === 403) return "invalid";
    return "unavailable";
  } catch {
    return "unavailable";
  }
}
