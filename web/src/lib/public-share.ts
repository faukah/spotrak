import { serverApiFetch } from "./api/server";
import type { SummaryStats } from "./api/types";

export interface PublicShareContext {
  token: string;
  apiPrefix: string;
  pagePrefix: string;
  valid: boolean;
}

export function publicSharePaths(
  token = "",
): Omit<PublicShareContext, "valid"> {
  return {
    token,
    apiPrefix: `/public/${token}`,
    pagePrefix: `/public/${token}`,
  };
}

export async function publicShareContext(
  request: Request,
  token = "",
): Promise<PublicShareContext> {
  const paths = publicSharePaths(token);
  if (!token) return { ...paths, valid: false };
  const summary = await serverApiFetch<SummaryStats>(
    request,
    `${paths.apiPrefix}/stats/summary`,
  );
  return { ...paths, valid: !!summary };
}

export async function publicDetailContext<T>(
  request: Request,
  token: string | undefined,
  detailPath: string,
): Promise<PublicShareContext & { initialDetail: T | null }> {
  const context = await publicShareContext(request, token ?? "");
  const initialDetail = context.valid
    ? await serverApiFetch<T>(request, `${context.apiPrefix}${detailPath}`)
    : null;
  return { ...context, valid: context.valid && !!initialDetail, initialDetail };
}
