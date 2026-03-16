<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	let {
		appFilter = $bindable(null),
		timeFrom = $bindable(null),
		timeTo = $bindable(null)
	}: {
		appFilter: string | null;
		timeFrom: string | null;
		timeTo: string | null;
	} = $props();

	let apps = $state<string[]>([]);

	$effect(() => {
		invoke<string[]>('get_distinct_apps').then((a) => (apps = a));
	});
</script>

<div class="flex items-center gap-2 border-b border-[#262626] px-4 py-2">
	<!-- App filter -->
	<select
		bind:value={appFilter}
		class="rounded border border-[#262626] bg-[#141414] px-2 py-1 text-xs text-[#A3A3A3]"
	>
		<option value={null}>All apps</option>
		{#each apps as app}
			<option value={app}>{app}</option>
		{/each}
	</select>

	<!-- Date range -->
	<input
		type="date"
		bind:value={timeFrom}
		class="rounded border border-[#262626] bg-[#141414] px-2 py-1 text-xs text-[#A3A3A3]"
	/>
	<span class="text-xs text-[#525252]">to</span>
	<input
		type="date"
		bind:value={timeTo}
		class="rounded border border-[#262626] bg-[#141414] px-2 py-1 text-xs text-[#A3A3A3]"
	/>

	<!-- Clear -->
	{#if appFilter || timeFrom || timeTo}
		<button
			onclick={() => {
				appFilter = null;
				timeFrom = null;
				timeTo = null;
			}}
			class="text-xs text-[#525252] hover:text-[#A3A3A3]"
		>
			Clear
		</button>
	{/if}
</div>
