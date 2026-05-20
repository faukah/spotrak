<script lang="ts">
  import { onMount } from 'svelte';
  import { redirectToLogin } from '../../lib/auth/redirect';

  onMount(() => {
    const onUnauthorized = () => redirectToLogin();
    window.addEventListener('spotrak:unauthorized', onUnauthorized);
    return () => window.removeEventListener('spotrak:unauthorized', onUnauthorized);
  });
</script>
