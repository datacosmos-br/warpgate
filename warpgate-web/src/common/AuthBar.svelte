<script lang="ts">
  import { faSignOut } from '@fortawesome/free-solid-svg-icons'
  import Fa from 'svelte-fa'

  import { api } from 'gateway/lib/api'
  import { serverInfo, reloadServerInfo } from 'gateway/lib/store'
  import { Button, Dropdown, DropdownItem, DropdownMenu, DropdownToggle } from '@sveltestrap/sveltestrap'

  async function logout () {
    await api.logout()
    await reloadServerInfo()
    location.href = '/@warpgate'
  }

  async function singleLogout () {
    const response = await api.initiateSsoLogout()
    location.href = response.url
  }
</script>

{#if $serverInfo?.username}
  <div class="ms-auto">
    {$serverInfo.username}
    {#if $serverInfo.authorizedViaTicket}
      <span class="ml-2">(ticket auth)</span>
    {/if}
  </div>

  {#if $serverInfo?.authorizedViaSsoWithSingleLogout}
    <Dropdown>
      <DropdownToggle
        color="link"
        title="Log out options">
        <Fa
          fw
          icon={faSignOut} />
      </DropdownToggle>
      <DropdownMenu right={true}>
        <DropdownItem on:click={logout}>
          <Fa
            fw
            icon={faSignOut} />
          Log out of Warpgate
        </DropdownItem>
        <DropdownItem on:click={singleLogout}>
          <Fa
            fw
            icon={faSignOut} />
          Log out everywhere
        </DropdownItem>
      </DropdownMenu>
    </Dropdown>
  {:else}
    <Button
      color="link"
      title="Log out"
      on:click={logout}>
      <Fa
        fw
        icon={faSignOut} />
    </Button>
  {/if}
{/if}
