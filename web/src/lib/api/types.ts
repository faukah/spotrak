export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
    details?: unknown[];
  };
}

export interface EntityRef {
  id: string;
  name: string;
}

export interface SearchResults {
  tracks: EntityRef[];
  artists: EntityRef[];
  albums: EntityRef[];
}

export interface PublicUser {
  id: string;
  username: string;
  spotify_id: string;
  admin: boolean;
  created_at: string;
}

export interface UserSettings {
  user_id: string;
  history_line: boolean;
  preferred_stats_period: "day" | "week" | "month" | "year";
  nb_elements: number;
  metric_used: "number" | "duration";
  dark_mode: "follow" | "dark" | "light";
  timezone?: string | null;
  date_format: string;
  hour_format: "12" | "24";
  updated_at: string;
}

export interface PublicSharing {
  enabled: boolean;
  token?: string | null;
}

export interface MeResponse {
  user: PublicUser;
  settings: UserSettings;
  public_sharing: PublicSharing;
}

export interface SummaryStats {
  total_listens: number;
  total_duration_ms: number;
  unique_tracks: number;
  unique_artists: number;
  unique_albums: number;
}

export type StatsRangeKey =
  | "today"
  | "week"
  | "month"
  | "year"
  | "selected-year"
  | "all";

export interface StatsRangeResponse {
  range: StatsRangeKey;
  label: string;
  comparison_label?: string | null;
  start?: string | null;
  end?: string | null;
  previous_start?: string | null;
  previous_end?: string | null;
}

export interface HistoryEvent {
  id: string;
  track_id: string;
  track_name: string;
  album_id: string;
  album_name: string;
  artist_id: string;
  artist_name: string;
  image_url?: string | null;
  duration_ms: number;
  played_at: string;
  source: string;
}

export interface TopTrack {
  id: string;
  name: string;
  album_id: string;
  album_name: string;
  artist_id: string;
  artist_name: string;
  image_url?: string | null;
  count: number;
  duration_ms: number;
}

export interface TopArtist {
  id: string;
  name: string;
  image_url?: string | null;
  count: number;
  duration_ms: number;
}

export interface TopAlbum {
  id: string;
  name: string;
  artist_name?: string | null;
  release_year?: number | null;
  image_url?: string | null;
  count: number;
  duration_ms: number;
}

export interface BucketedTopArtist extends TopArtist {
  bucket: string;
}

export interface BucketedTopAlbum extends Omit<TopAlbum, "release_year"> {
  bucket: string;
}

export interface BucketedTopTrack extends TopTrack {
  bucket: string;
}

export interface TimelinePoint {
  bucket: string;
  count: number;
  duration_ms: number;
}

export interface DiversityTimelinePoint {
  bucket: string;
  unique_tracks: number;
  unique_artists: number;
  unique_albums: number;
  average_release_year?: number | null;
}

export interface HourRepartitionPoint {
  hour: number;
  count: number;
  duration_ms: number;
}

export interface FeatureRatioStats {
  solo_count: number;
  feature_count: number;
  solo_duration_ms: number;
  feature_duration_ms: number;
}

export interface AlbumReleaseYearPoint {
  release_year?: number | null;
  count: number;
  duration_ms: number;
}

export interface AlbumReleaseYearsStats {
  average_release_year?: number | null;
  distribution: AlbumReleaseYearPoint[];
}

export interface LongestSession {
  start: string;
  end: string;
  duration_ms: number;
  listens: number;
  tracks: HistoryEvent[];
}

export interface EntityStats extends SummaryStats {
  first_played_at?: string | null;
  last_played_at?: string | null;
}

export interface OverviewStatsResponse {
  range: StatsRangeResponse;
  available_years: number[];
  summary: SummaryStats;
  previous_summary?: SummaryStats | null;
  best_artist?: TopArtist | null;
  best_artist_stats?: EntityStats | null;
  best_song?: TopTrack | null;
  hourly_distribution: HourRepartitionPoint[];
  history: HistoryEvent[];
}

export interface AlbumRef {
  id: string;
  name: string;
  images: unknown;
}

export interface TrackDetail {
  id: string;
  name: string;
  duration_ms: number;
  explicit: boolean;
  href?: string | null;
  uri?: string | null;
  popularity?: number | null;
  disc_number?: number | null;
  track_number?: number | null;
  images: unknown;
  album: AlbumRef;
  artists: EntityRef[];
}

export interface SpotifyCurrentlyPlaying {
  is_playing: boolean;
  progress_ms?: number | null;
  item?: TrackDetail | null;
}

export interface ArtistDetail {
  id: string;
  name: string;
  href?: string | null;
  uri?: string | null;
  popularity?: number | null;
  images: unknown;
  genres: unknown;
  blacklisted: boolean;
}

export interface AlbumDetail {
  id: string;
  name: string;
  album_type?: string | null;
  release_date?: string | null;
  release_year?: number | null;
  total_tracks?: number | null;
  href?: string | null;
  uri?: string | null;
  images: unknown;
  artists: EntityRef[];
}

export interface ImportJob {
  id: string;
  name: string;
  filenames: string[];
  import_type: string;
  status: string;
  total: number;
  current: number;
  error_message?: string | null;
  created_at: string;
  updated_at: string;
}
