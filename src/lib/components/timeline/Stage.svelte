<script lang="ts">
	import { convertFileSrc } from '@tauri-apps/api/core';

	interface CaptureRow {
		id: number;
		timestamp: string;
		app_name: string;
		bundle_id: string;
		window_title: string;
		image_path: string;
	}

	let { capture, ocrText = null }: { capture: CaptureRow; ocrText: string | null } = $props();

	let showText = $state(false);

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleTimeString('en-US', {
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		});
	}
</script>

<div class="relative flex h-full flex-col">
	<!-- Metadata bar -->
	<div class="flex items-center gap-4 bg-[#0A0A0A] px-6 py-2 text-sm">
		<span class="font-medium text-[#FAFAFA]">{capture.app_name}</span>
		<span class="text-[#525252]">{capture.window_title}</span>
		<span class="ml-auto text-[#A3A3A3]">{formatTime(capture.timestamp)}</span>
		{#if ocrText}
			<button
				onclick={() => (showText = !showText)}
				class="rounded px-2 py-0.5 text-xs text-[#A3A3A3] hover:bg-[#1C1C1C] hover:text-[#FAFAFA]"
			>
				{showText ? 'Hide Text' : 'Show Text'}
			</button>
		{/if}
	</div>

	<!-- Screenshot / Text split -->
	<div class="flex flex-1 overflow-hidden">
		<!-- Screenshot -->
		<div class="flex flex-1 items-center justify-center bg-[#050505] p-4">
			<img
				src={convertFileSrc(capture.image_path)}
				alt="Screenshot at {formatTime(capture.timestamp)}"
				class="max-h-full max-w-full rounded object-contain"
			/>
		</div>

		<!-- OCR text panel -->
		{#if showText && ocrText}
			<div class="w-80 overflow-y-auto border-l border-[#262626] bg-[#0A0A0A] p-4">
				<h3 class="mb-2 text-xs font-medium uppercase tracking-wide text-[#525252]">Extracted Text</h3>
				<pre class="whitespace-pre-wrap text-xs leading-relaxed text-[#A3A3A3]">{ocrText}</pre>
			</div>
		{/if}
	</div>
</div>
