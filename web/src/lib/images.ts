export interface SpotifyImage {
  url?: string | null;
  height?: number | null;
  width?: number | null;
}

const ALLOWED_IMAGE_HOSTS = new Set([
  "i.scdn.co",
  "mosaic.scdn.co",
  "image-cdn-ak.spotifycdn.com",
]);

export function spotifyImageUrl(value: unknown): string | null {
  if (!value) return null;
  if (typeof value === "string") return allowedImageUrl(value);
  if (Array.isArray(value)) {
    const images = value.filter(isSpotifyImage);
    images.sort((a, b) =>
      Math.abs((a.width ?? a.height ?? 0) - 320) -
      Math.abs((b.width ?? b.height ?? 0) - 320)
    );
    return allowedImageUrl(images[0]?.url ?? null);
  }
  if (isSpotifyImage(value)) return allowedImageUrl(value.url ?? null);
  return null;
}

export function directImageUrl(
  value: { image_url?: string | null } | null | undefined,
): string | null {
  return allowedImageUrl(value?.image_url ?? null);
}

export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? "")
    .join("") || "♪";
}

export function viewTransitionName(id: string, scope = "detail"): string {
  return `cover-${sanitizeTransitionPart(scope)}-${sanitizeTransitionPart(id)}`;
}

export function transitionHref(href: string, transitionName: string): string {
  const url = new URL(href, "http://spotrak.local");
  url.searchParams.set("vt", transitionName);
  return `${url.pathname}${url.search}${url.hash}`;
}

function allowedImageUrl(value: string | null | undefined): string | null {
  if (!value) return null;
  try {
    const url = new URL(value);
    if (url.protocol !== "https:") return null;
    if (!ALLOWED_IMAGE_HOSTS.has(url.hostname)) return null;
    return url.toString();
  } catch {
    return null;
  }
}

function sanitizeTransitionPart(value: string): string {
  return value.replace(/[^a-zA-Z0-9_-]/g, "-");
}

function isSpotifyImage(value: unknown): value is SpotifyImage {
  return typeof value === "object" && value !== null && "url" in value;
}
