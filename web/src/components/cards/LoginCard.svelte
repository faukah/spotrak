<script lang="ts">
  import { loginUrl } from '../../lib/api/client';
  export let message = 'Sign in to continue.';
  import * as Card from '../ui/card';
  import { Button } from '../ui/button';
</script>

<Card.Root class="login-card">
  <Card.Header>
    <Card.Title>Sign in with Spotify</Card.Title>
    <Card.Description>{message} Connect once. The app keeps a private, local history and stats database.</Card.Description>
  </Card.Header>
  <Card.Content>
    <Button href={loginUrl()} size="lg">Continue with Spotify</Button>
  </Card.Content>
</Card.Root>

<style>
  :global(.login-card) {
    max-width: 34rem;
  }
</style>
