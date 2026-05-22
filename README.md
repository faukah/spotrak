<!-- markdownlint-disable MD033 MD041 -->
<div align="center">
  <h1 id="header">Spotrak</h1>
  <br/>
  <h6>A no-bullshit, self-hostable music tracking dashboard for Spotify.</h1>
  <br/>
</div>

## Features

- **Live music tracking**: See what you're listening to in real-time.
- **Import your listening history**: Import your full Spotify listening history.

## Local Spotify Artist Hydration

If you have the local Spotify data export in `~/Downloads/spotify_data`, hydrate only artists
already referenced by Spotrak without calling Spotify:

```bash
uv run scripts/hydrate_artists_from_spotify_data.py --dry-run
uv run scripts/hydrate_artists_from_spotify_data.py
```

The script reads `DATABASE_URL` from the environment or `.env`, scans the local Parquet files with
Polars, upserts matching artists into Postgres, and clears hydration queue rows only when local
images were found.

For full privacy imports that are blocked by Spotify track lookup rate limits, pre-cache track
metadata from the same local export and then retry the import:

```bash
uv run scripts/precache_tracks_from_spotify_data.py --dry-run
uv run scripts/precache_tracks_from_spotify_data.py
```

This scans queued, running, and failed `full-privacy` import files, extracts `spotify:track:{id}`
values, fills the existing `tracks.raw` cache plus related album/artist rows, and does not ingest
listening events itself.

# Images, because that's the only thing that counts! :D

<p align="center">
  <img src=".github/assets/overview.png" alt="Spotrak overview" width="100%" />
</p>

<p align="center">
  <img src=".github/assets/stats_1.png" alt="Spotrak stats screenshot 1" width="100%" />
</p>

<p align="center">
  <img src=".github/assets/stats_2.png" alt="Spotrak stats screenshot 2" width="100%" />
</p>
