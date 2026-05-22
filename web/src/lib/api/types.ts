import type { components } from "./generated";

type Schema<Name extends keyof components["schemas"]> =
  components["schemas"][Name];

export type ApiErrorBody = Schema<"ErrorEnvelope">;
export type EntityRef = Schema<"EntityRef">;
export type SearchResults = Schema<"SearchResults">;
export type PublicUser = Schema<"PublicUser">;
export type UserSettings = Schema<"UserSettings">;
export type StatsDisplayPreferences = Schema<"StatsDisplayPreferences">;
export type PublicSharing = Schema<"PublicSharingResponse">;
export type MeResponse = Schema<"MeResponse">;
export type CurrentlyPlayingResponse = Schema<"CurrentlyPlayingResponse">;
export type SummaryStats = Schema<"SummaryStats">;
export type StatsRangeKey = Schema<"StatsRangeKey">;
export type TimeSplit = Schema<"TimeSplit">;
export type StatsRangeResponse = Schema<"StatsRangeResponse">;
export type HistoryEvent = Schema<"HistoryEvent">;
export type TopTrack = Schema<"TopTrack">;
export type TopArtist = Schema<"TopArtist">;
export type TopAlbum = Schema<"TopAlbum">;
export type BucketedTopArtist = Schema<"BucketedTopArtist">;
export type HourlyTopArtist = Schema<"HourlyTopArtist">;
export type BucketedTopAlbum = Schema<"BucketedTopAlbum">;
export type BucketedTopTrack = Schema<"BucketedTopTrack">;
export type TimelinePoint = Schema<"TimelinePoint">;
export type DiversityTimelinePoint = Schema<"DiversityTimelinePoint">;
export type HourRepartitionPoint = Schema<"HourRepartitionPoint">;
export type FeatureRatioStats = Schema<"FeatureRatioStats">;
export type FeatureAverageStats = Schema<"FeatureAverageStats">;
export type FeatureTimelinePoint = Schema<"FeatureTimelinePoint">;
export type AlbumReleaseYearPoint = Schema<"AlbumReleaseYearPoint">;
export type AlbumReleaseYearsStats = Schema<"AlbumReleaseYearsStats">;
export type ArtistRunSummary = Schema<"ArtistRunSummary">;
export type ComebackArtist = Schema<"ComebackArtist">;
export type DiscoveryStats = Schema<"DiscoveryStats">;
export type ListeningConcentrationStats = Schema<"ListeningConcentrationStats">;
export type ListeningSessionStats = Schema<"ListeningSessionStats">;
export type ListeningSessionSummary = Schema<"ListeningSessionSummary">;
export type LongestSession = Schema<"LongestSession">;
export type RepeatLoopStats = Schema<"RepeatLoopStats">;
export type RepeatLoopSummary = Schema<"RepeatLoopSummary">;
export type EntityStats = Schema<"EntityStats">;
export type OverviewStatsResponse = Schema<"OverviewStatsResponse">;
export type StatsBucketAxis = Schema<"StatsBucketAxis">;
export type StatsDashboardResponse = Schema<"StatsDashboardResponse">;
export type StatsDashboardBootstrapResponse = Schema<
  "StatsDashboardBootstrapResponse"
>;
export type AlbumRef = Schema<"AlbumRef">;
export type TrackDetail = Schema<"TrackDetail">;
export type ArtistDetail = Schema<"ArtistDetail">;
export type AlbumDetail = Schema<"AlbumDetail">;
export type ImportJob = Schema<"ImportJobResponse">;
export type ImportJobsResponse = Schema<"ImportJobsResponse">;
