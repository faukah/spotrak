<script lang="ts">
  import { onMount } from 'svelte';
  import { Trash2, Upload, XCircle } from '@lucide/svelte';
  import { apiFetch } from '../../lib/api/client';
  import type { ImportJob } from '../../lib/api/types';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';
  import { Input } from '../ui/input';

  type PendingConfirmation = {
    title: string;
    message: string;
    confirmLabel: string;
    run: () => Promise<void>;
  };

  let jobs: ImportJob[] = [];
  let files: FileList | undefined = undefined;
  let importType: 'privacy' | 'full-privacy' = 'privacy';
  let loading = true;
  let uploading = false;
  let removing: string | null = null;
  let deletingHistory = false;
  let error: string | null = null;
  let pendingConfirmation: PendingConfirmation | null = null;
  let dialogElement: HTMLDivElement | null = null;
  let cancelButton: HTMLButtonElement | null = null;
  let previousFocus: HTMLElement | null = null;

  onMount(() => {
    void load();
    const interval = window.setInterval(() => void load(false), 5_000);
    const onKeyDown = (event: KeyboardEvent) => {
      if (!pendingConfirmation) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        cancelConfirmation();
        return;
      }
      if (event.key === 'Tab') trapDialogFocus(event);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('keydown', onKeyDown);
      document.body.classList.remove('spotrak-dialog-open');
    };
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

  function deleteImportedHistory() {
    openConfirmation({
      title: 'Remove imported history?',
      message: 'Remove all listening events created by privacy/full-privacy imports? This is useful for old imports that were deleted before event tracking was added.',
      confirmLabel: 'Remove imported history',
      run: performDeleteImportedHistory,
    });
  }

  async function performDeleteImportedHistory() {
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

  function removeJob(job: ImportJob) {
    const action = job.status === 'progress' ? 'cancel' : 'remove';
    openConfirmation({
      title: `${action === 'cancel' ? 'Cancel' : 'Remove'} import?`,
      message: `${action === 'cancel' ? 'Cancel' : 'Remove'} import "${job.name}"?`,
      confirmLabel: action === 'cancel' ? 'Cancel import' : 'Remove import',
      run: () => performRemoveJob(job, action),
    });
  }

  async function performRemoveJob(job: ImportJob, action: 'cancel' | 'remove') {
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

  function openConfirmation(confirmation: PendingConfirmation) {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    pendingConfirmation = confirmation;
    document.body.classList.add('spotrak-dialog-open');
    window.requestAnimationFrame(() => cancelButton?.focus());
  }

  function closeConfirmation(restoreFocus = true) {
    const focusTarget = previousFocus;
    pendingConfirmation = null;
    document.body.classList.remove('spotrak-dialog-open');
    if (restoreFocus) window.requestAnimationFrame(() => focusTarget?.focus());
    previousFocus = null;
  }

  function cancelConfirmation() {
    closeConfirmation();
  }

  async function confirmPending() {
    const confirmedAction = pendingConfirmation;
    closeConfirmation();
    await confirmedAction?.run();
  }

  function trapDialogFocus(event: KeyboardEvent) {
    const focusable = dialogElement
      ? [...dialogElement.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')]
          .filter((element) => !element.hasAttribute('disabled') && element.getAttribute('aria-hidden') !== 'true')
      : [];
    if (focusable.length === 0) {
      event.preventDefault();
      dialogElement?.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    } else if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    }
  }
</script>

{#if pendingConfirmation}
  <div class="confirm-backdrop" aria-hidden="true"></div>
  <div class="confirm-dialog" role="dialog" aria-modal="true" aria-labelledby="import-confirm-title" aria-describedby="import-confirm-message" tabindex="-1" bind:this={dialogElement}>
    <h2 id="import-confirm-title">{pendingConfirmation.title}</h2>
    <p id="import-confirm-message">{pendingConfirmation.message}</p>
    <div class="confirm-actions">
      <Button bind:ref={cancelButton} variant="outline" size="sm" onclick={cancelConfirmation}>Keep</Button>
      <Button variant="destructive" size="sm" onclick={() => void confirmPending()}>{pendingConfirmation.confirmLabel}</Button>
    </div>
  </div>
{/if}

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

  :global(body.spotrak-dialog-open) {
    overflow: hidden;
  }

  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: color-mix(in srgb, var(--color-bg) 88%, transparent);
  }

  .confirm-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    z-index: 81;
    display: grid;
    width: min(92vw, 32rem);
    transform: translate(-50%, -50%);
    gap: 0.85rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 1rem;
    background: var(--color-bg-elevated);
    box-shadow: var(--shadow-card);
  }

  .confirm-dialog:focus {
    outline: none;
  }

  .confirm-dialog h2,
  .confirm-dialog p {
    margin: 0;
  }

  .confirm-dialog p {
    color: var(--color-muted);
  }

  .confirm-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.5rem;
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
