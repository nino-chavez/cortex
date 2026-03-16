<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import SearchInput from './SearchInput.svelte';
	import ResultCard from './ResultCard.svelte';
	import ResultDetail from './ResultDetail.svelte';
	import FilterBar from './FilterBar.svelte';

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
	let showFilters = $state(false);
	let detailResult = $state<SearchResult | null>(null);

	// Filters
	let appFilter = $state<string | null>(null);
	let timeFrom = $state<string | null>(null);
	let timeTo = $state<string | null>(null);

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
				const timeFromISO = timeFrom ? `${timeFrom}T00:00:00Z` : null;
				const timeToISO = timeTo ? `${timeTo}T23:59:59Z` : null;

				const res = await invoke<SearchResult[]>('search_captures', {
					query: query.trim(),
					appFilter,
					timeFrom: timeFromISO,
					timeTo: timeToISO
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
			if (!focused && !detailResult) {
				win.hide();
			}
		});

		return () => {
			unlisten.then((fn) => fn());
		};
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
		} else if (e.key === 'Enter' && results[selectedIndex]) {
			e.preventDefault();
			detailResult = results[selectedIndex];
		} else if (e.key === 'Escape') {
			if (detailResult) {
				detailResult = null;
			} else {
				getCurrentWindow().hide();
			}
		} else if (e.key === 'Tab') {
			e.preventDefault();
			showFilters = !showFilters;
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex h-screen items-start justify-center bg-transparent pt-4">
	<div class="w-[720px] overflow-hidden rounded-2xl border border-[#262626] bg-[#0A0A0A] shadow-2xl">
		<!-- Search Input -->
		<div class="flex items-center gap-2 border-b border-[#262626] p-3">
			<div class="flex-1">
				<SearchInput bind:value={query} />
			</div>
			<button
				onclick={() => (showFilters = !showFilters)}
				class="shrink-0 rounded-md px-2 py-1 text-xs {showFilters
					? 'bg-blue-500/10 text-blue-400'
					: 'text-[#525252] hover:text-[#A3A3A3]'}"
			>
				Filter
			</button>
		</div>

		<!-- Filters -->
		{#if showFilters}
			<FilterBar bind:appFilter bind:timeFrom bind:timeTo />
		{/if}

		<!-- Results -->
		<div class="max-h-[440px] overflow-y-auto">
			{#if loading && results.length === 0}
				<div class="p-8 text-center text-sm text-[#525252]">Searching...</div>
			{:else if results.length === 0 && query.trim()}
				<div class="p-8 text-center text-sm text-[#525252]">No results found</div>
			{:else if results.length === 0}
				<div class="p-8 text-center text-sm text-[#525252]">
					<p>Type to search your history</p>
					<p class="mt-2 text-xs">Tab to toggle filters</p>
				</div>
			{:else}
				{#each results as result, i}
					<ResultCard
						{result}
						selected={i === selectedIndex}
						onclick={() => (detailResult = result)}
					/>
				{/each}
			{/if}
		</div>
	</div>
</div>

<!-- Detail overlay -->
{#if detailResult}
	<ResultDetail result={detailResult} onclose={() => (detailResult = null)} />
{/if}
