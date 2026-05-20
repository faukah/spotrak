import { writable } from "svelte/store";

export const selectedStatsMetric = writable<"count" | "duration">("count");
