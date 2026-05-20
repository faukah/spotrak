<script lang="ts">
  import { onMount } from 'svelte';
  import { Trash2, Upload, XCircle } from '@lucide/svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { ImportJob } from '../../lib/api/types';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';
  import { Input } from '../ui/input';

  let jobs: ImportJob[] = [];
  let files: FileList | undefined = undefined;
  let importType: 'privacy' | 'full-privacy' = 'privacy';
  let loading = true;
  let uploading = false;
  let removing: string | null = null;
  let deletingHistory = false;
  let error: string | null = null;

  onMount(() => {
    void load();
    const interval = window.setInterval(() => void load(false), 5_000);
    return () => window.clearInterval(interval);
  });

  async function load(showLoading = true) {
    loading = showLoading;
    try {
      const response = await apiFetch<{ imports: ImportJob[] }>('/imports');
      jobs = response.imports;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to load imports';
    } finally {
      loading = false;
    }
  }

  async function upload() {
    if (!files?.length) return;
    uploading = true;
    error = null;
    try {
      const form = new FormData();
      Array.from(files).forEach((file) => form.append('files', file));
      await apiFetch<ImportJob>(`/imports/${importType}`, { method: 'POST', body: form });
      files = undefined;
      await load();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to upload import';
    } finally {
      uploading = false;
    }
  }

  async function deleteImportedHistory() {
    const confirmed = window.confirm('Remove all listening events created by privacy/full-privacy imports? This is useful for old imports that were deleted before event tracking was added.');
    if (!confirmed) return;
    deletingHistory = true;
    error = null;
    try {
      await apiFetch<void>('/imports/history', { method: 'DELETE' });
      await load(false);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unable to remove imported history';
    } finally {
      deletingHistory = false;
    }
  }

  async function removeJob(job: ImportJob) {
    const action = job.status === 'progress' ? 'cancel' : 'remove';
    const confirmed = window.confirm(`${action === 'cancel' ? 'Cancel' : 'Remove'} import "${job.name}"?`);
    if (!confirmed) return;
    removing = job.id;
    error = null;
    try {
      if (job.status === 'progress') {
        await apiFetch<ImportJob>(`/imports/${job.id}/cancel`, { method: 'POST' });
      } else {
        await apiFetch<void>(`/imports/${job.id}`, { method: 'DELETE' });
      }
      await load(false);
    } catch (err) {
      error = err instanceof Error ? err.message : `Unable to ${action} import`;
    } finally {
      removing = null;
    }
  }
</script>

<div class="import-layout">
  <Card.Root>
    <Card.Header>
      <Card.Title>Queue an import</Card.Title>
    </Card.Header>
    <Card.Content>
      <form on:submit|preventDefault={upload}>
        <label>
          Import type
          <select bind:value={importType}>
            <option value="privacy">Privacy data</option>
            <option value="full-privacy">Full privacy data</option>
          </select>
        </label>
        <label>
          Files
          <Input type="file" bind:files multiple accept="application/json,.json,.zip" />
        </label>
        <Button type="submit" disabled={uploading || !files?.length}>
          <Upload aria-hidden="true" />
          {uploading ? 'Uploading…' : 'Queue import'}
        </Button>
      </form>
      <div class="danger-zone">
        <p>Removing an import now removes events created by that job. For older deleted imports, clear imported history here.</p>
        <Button variant="destructive" size="sm" disabled={deletingHistory} onclick={deleteImportedHistory}>
          <Trash2 aria-hidden="true" />
          {deletingHistory ? 'Removing…' : 'Remove imported history'}
        </Button>
      </div>
      {#if error}<p class="error">{error}</p>{/if}
    </Card.Content>
  </Card.Root>

  <Card.Root>
    <Card.Header>
      <Card.Title>Jobs</Card.Title>
    </Card.Header>
    <Card.Content>
      {#if loading}
        <div class="jobs"><div class="skeleton"></div><div class="skeleton"></div></div>
      {:else if jobs.length === 0}
        <p class="state">No imports queued.</p>
      {:else}
        <ul class="jobs">
          {#each jobs as job}
            <li>
              <div>
                <strong title={job.name}>{job.name}</strong>
                <span>{job.import_type} · {job.status}</span>
                {#if job.filenames.length > 1}
                  <small title={job.filenames.join(', ')}>{job.filenames.length} files</small>
                {/if}
              </div>
              <progress max={job.total || 1} value={job.current}></progress>
              <small>{job.current}/{job.total}</small>
              <Button variant={job.status === 'progress' ? 'outline' : 'destructive'} size="sm" disabled={removing === job.id} onclick={() => removeJob(job)}>
                {#if job.status === 'progress'}<XCircle aria-hidden="true" />{:else}<Trash2 aria-hidden="true" />{/if}
                {job.status === 'progress' ? 'Cancel' : 'Remove'}
              </Button>
            </li>
          {/each}
        </ul>
      {/if}
    </Card.Content>
  </Card.Root>
</div>

<style>
  .import-layout {
    display: grid;
    grid-template-columns: minmax(18rem, 0.8fr) minmax(20rem, 1.2fr);
    gap: 0.75rem;
  }

  form,
  .jobs {
    display: grid;
    gap: 0.75rem;
  }

  label {
    display: grid;
    gap: 0.35rem;
    color: var(--color-muted);
    font-size: 0.82rem;
    font-weight: 700;
  }

  select {
    min-height: 2.25rem;
    padding: 0 0.6rem;
  }

  .danger-zone {
    display: grid;
    gap: 0.6rem;
    margin-top: 1rem;
    border-top: 1px solid var(--color-border);
    padding-top: 1rem;
  }

  .danger-zone p {
    margin: 0;
    color: var(--color-muted);
    font-size: 0.8rem;
  }

  .error {
    margin: 0.75rem 0 0;
    color: var(--color-danger);
  }

  .state {
    color: var(--color-muted);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(8rem, 0.8fr) auto auto;
    gap: 0.75rem;
    align-items: center;
    border-bottom: 1px solid var(--color-border);
    padding-bottom: 0.65rem;
  }

  li div {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
  }

  li strong,
  li span,
  li small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  span,
  small {
    color: var(--color-muted);
    font-size: 0.78rem;
  }

  progress {
    width: 100%;
    height: 0.45rem;
    border: 1px solid var(--color-border);
    border-radius: 0;
    background: var(--color-panel);
  }

  progress::-webkit-progress-bar {
    background: var(--color-panel);
  }

  progress::-webkit-progress-value {
    background: var(--color-primary);
  }

  .skeleton {
    height: 3rem;
  }

  @media (max-width: 760px) {
    .import-layout,
    li {
      grid-template-columns: 1fr;
    }
  }
</style>
