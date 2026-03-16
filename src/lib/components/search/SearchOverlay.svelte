<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import SearchInput from './SearchInput.svelte';
	import ResultCard from './ResultCard.svelte';

	interface SearchResult {
		capture_id: number;
		timestamp: string;
		app_name: string;
		snippet: string;
		image_path: string;
		result_type: string;
	}

	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let selectedIndex = $state(0);
	let loading = $state(false);

	// Debounced search
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		if (debounceTimer) clearTimeout(debounceTimer);

		if (!query.trim()) {
			results = [];
			return;
		}

		loading = true;
		debounceTimer = setTimeout(async () => {
			try {
				const res = await invoke<SearchResult[]>('search_captures', {
					query: query.trim(),
					appFilter: null,
					timeFrom: null,
					timeTo: null
				});
				results = res;
				selectedIndex = 0;
			} catch (e) {
				console.error('Search failed:', e);
				results = [];
			} finally {
				loading = false;
			}
		}, 300);
	});

	// Hide on blur
	$effect(() => {
		const win = getCurrentWindow();
		const unlisten = win.onFocusChanged(({ payload: focused }) => {
			if (!focused) {
				win.hide();
			}
		});

		return () => {
			unlisten.then((fn) => fn());
		};
	});

	// Keyboard navigation
	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
		} else if (e.key === 'Escape') {
			getCurrentWindow().hide();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex h-screen items-start justify-center bg-transparent pt-4">
	<div class="w-[720px] overflow-hidden rounded-2xl border border-[#262626] bg-[#0A0A0A] shadow-2xl">
		<!-- Search Input -->
		<div class="border-b border-[#262626] p-3">
			<SearchInput bind:value={query} />
		</div>

		<!-- Results -->
		<div class="max-h-[440px] overflow-y-auto">
			{#if loading && results.length === 0}
				<div class="p-8 text-center text-sm text-[#525252]">Searching...</div>
			{:else if results.length === 0 && query.trim()}
				<div class="p-8 text-center text-sm text-[#525252]">No results found</div>
			{:else if results.length === 0}
				<div class="p-8 text-center text-sm text-[#525252]">Type to search your history</div>
			{:else}
				{#each results as result, i}
					<ResultCard {result} selected={i === selectedIndex} />
				{/each}
			{/if}
		</div>
	</div>
</div>
