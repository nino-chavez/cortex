<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import NavBar from '$lib/components/NavBar.svelte';
	import { page } from '$app/state';

	let { children } = $props();

	// Don't show nav on the search overlay window
	let showNav = $derived(page.url.pathname !== '/search');
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{#if showNav}
	<NavBar />
{/if}
{@render children()}
