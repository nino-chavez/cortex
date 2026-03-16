<script lang="ts">
	import { convertFileSrc } from '@tauri-apps/api/core';

	interface CaptureRow {
		id: number;
		timestamp: string;
		app_name: string;
		image_path: string;
	}

	let {
		captures,
		activeCaptureId,
		onselect
	}: {
		captures: CaptureRow[];
		activeCaptureId: number | null;
		onselect: (capture: CaptureRow) => void;
	} = $props();

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleTimeString('en-US', {
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="flex gap-1 overflow-x-auto px-4 py-3" style="scrollbar-width: thin; scrollbar-color: #262626 transparent;">
	{#each captures as capture (capture.id)}
		<button
			onclick={() => onselect(capture)}
			class="group flex shrink-0 flex-col items-center gap-1 rounded-lg p-1 transition-colors {capture.id === activeCaptureId
				? 'bg-[#1C1C1C] ring-1 ring-blue-500/50'
				: 'hover:bg-[#141414]'}"
		>
			<div class="h-14 w-24 overflow-hidden rounded bg-[#141414]">
				<img
					src={convertFileSrc(capture.image_path)}
					alt=""
					class="h-full w-full object-cover"
					loading="lazy"
				/>
			</div>
			<span class="text-[9px] text-[#525252] group-hover:text-[#A3A3A3]">
				{formatTime(capture.timestamp)}
			</span>
		</button>
	{/each}

	{#if captures.length === 0}
		<div class="flex w-full items-center justify-center py-4 text-sm text-[#525252]">
			No captures to display
		</div>
	{/if}
</div>
