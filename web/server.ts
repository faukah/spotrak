import { handle } from "./dist/server/entry.mjs";

const hostname = Deno.env.get("HOST") ?? "0.0.0.0";
const port = Number(Deno.env.get("PORT") ?? "4322");

Deno.serve({ hostname, port }, handle);
