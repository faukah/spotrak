import { defineMiddleware } from 'astro:middleware';
import { SERVER_API_ENDPOINT } from './lib/api/server';

const PUBLIC_FILES = new Set(['/favicon.ico', '/robots.txt', '/manifest.webmanifest']);

export const onRequest = defineMiddleware(async (context, next) => {
  const { pathname } = context.url;

  if (isPublicPath(pathname)) return next();
  if (await hasValidSession(context.request)) return next();

  const returnTo = `${context.url.pathname}${context.url.search}${context.url.hash}`;
  return context.redirect(`/login?next=${encodeURIComponent(returnTo)}`, 302);
});

function isPublicPath(pathname: string): boolean {
  return (
    PUBLIC_FILES.has(pathname) ||
    pathname === '/login' ||
    pathname.startsWith('/login/') ||
    pathname === '/public' ||
    pathname.startsWith('/public/') ||
    pathname.startsWith('/_astro/')
  );
}

async function hasValidSession(request: Request): Promise<boolean> {
  const cookie = request.headers.get('cookie');
  if (!cookie) return false;

  try {
    const response = await fetch(`${SERVER_API_ENDPOINT}/api/v1/auth/me`, {
      headers: { Accept: 'application/json', cookie },
    });
    return response.ok;
  } catch {
    return false;
  }
}
