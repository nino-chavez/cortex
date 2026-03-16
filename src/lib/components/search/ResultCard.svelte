<script lang="ts">
	import { convertFileSrc } from '@tauri-apps/api/core';

	interface SearchResult {
		capture_id: number;
		timestamp: string;
		app_name: string;
		snippet: string;
		image_path: string;
		result_type: string;
	}

	let {
		result,
		selected = false,
		onclick
	}: { result: SearchResult; selected?: boolean; onclick?: () => void } = $props();

	function relativeTime(iso: string): string {
		const now = Date.now();
		const then = new Date(iso).getTime();
		const diff = now - then;
		const seconds = Math.floor(diff / 1000);
		const minutes = Math.floor(seconds / 60);
		const hours = Math.floor(minutes / 60);
		const days = Math.floor(hours / 24);

		if (days > 0) return `${days}d ago`;
		if (hours > 0) return `${hours}h ago`;
		if (minutes > 0) return `${minutes}m ago`;
		return 'just now';
	}

	let thumbnailSrc = $derived(
		result.image_path && result.result_type !== 'transcription'
			? convertFileSrc(result.image_path)
			: null
	);
</script>

<button
	{onclick}
	class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors {selected
		? 'bg-[#1C1C1C]'
		: 'hover:bg-[#141414]'}"
>
	<!-- Thumbnail -->
	{#if thumbnailSrc}
		<img
			src={thumbnailSrc}
			alt=""
			class="h-12 w-12 shrink-0 rounded-md object-cover"
			loading="lazy"
		/>
	{:else}
		<div class="flex h-12 w-12 shrink-0 items-center justify-center rounded-md bg-[#1C1C1C] text-lg text-[#525252]">
			{result.result_type === 'transcription' ? '🎙' : '📷'}
		</div>
	{/if}

	<!-- Content -->
	<div class="min-w-0 flex-1">
		<div class="flex items-center gap-2 text-xs">
			<span class="font-medium text-[#A3A3A3]">{result.app_name}</span>
			<span class="text-[#525252]">{relativeTime(result.timestamp)}</span>
			{#if result.result_type === 'transcription'}
				<span class="text-[#525252]">✦</span>
			{/if}
		</div>
		<p class="mt-0.5 truncate text-sm text-[#D4D4D4]">
			{@html result.snippet}
		</p>
	</div>

	<!-- Source badge -->
	<span
		class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide {result.result_type ===
		'transcription'
			? 'bg-purple-500/10 text-purple-400'
			: 'bg-blue-500/10 text-blue-400'}"
	>
		{result.result_type === 'transcription' ? 'Audio' : 'OCR'}
	</span>
</button>
