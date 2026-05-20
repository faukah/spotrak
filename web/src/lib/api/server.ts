// oxlint-disable typescript/no-unsafe-type-assertion -- Server API JSON boundaries are typed by callers/OpenAPI.
export const SERVER_API_ENDPOINT =
  (import.meta.env.PUBLIC_API_ENDPOINT ?? "http://127.0.0.1:8080").replace(
    /\/$/,
    "",
  );

export async function serverApiFetch<T>(
  request: Request,
  path: string,
): Promise<T | null> {
  const headers = new Headers({ Accept: "application/json" });
  const cookie = request.headers.get("cookie");
  if (cookie) headers.set("cookie", cookie);

  try {
    const response = await fetch(`${SERVER_API_ENDPOINT}/api/v1${path}`, {
      headers,
    });
    if (!response.ok) return null;
    return (await response.json()) as T;
  } catch {
    return null;
  }
}
