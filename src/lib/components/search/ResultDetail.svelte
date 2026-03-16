<script lang="ts">
	import { invoke, convertFileSrc } from '@tauri-apps/api/core';

	interface SearchResult {
		capture_id: number;
		timestamp: string;
		app_name: string;
		snippet: string;
		image_path: string;
		result_type: string;
	}

	let { result, onclose }: { result: SearchResult; onclose: () => void } = $props();

	let ocrText = $state<string | null>(null);

	$effect(() => {
		if (result.result_type !== 'transcription') {
			invoke<string | null>('get_capture_ocr_text', { captureId: result.capture_id })
				.then((text) => (ocrText = text))
				.catch(() => (ocrText = null));
		}
	});

	async function copyText() {
		const text = ocrText || result.snippet.replace(/<[^>]*>/g, '');
		await navigator.clipboard.writeText(text);
	}

	async function openInFinder() {
		if (result.image_path) {
			await invoke('open_in_finder', { path: result.image_path }).catch(() => {
				// Fallback: not implemented yet
			});
		}
	}

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		});
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<button class="fixed inset-0 z-40 bg-black/60" onclick={onclose} aria-label="Close detail"></button>

<!-- Detail panel -->
<div class="fixed inset-4 z-50 flex flex-col overflow-hidden rounded-2xl border border-[#262626] bg-[#0A0A0A]">
	<!-- Header -->
	<div class="flex items-center justify-between border-b border-[#262626] px-6 py-3">
		<div class="flex items-center gap-3">
			<span class="font-medium text-[#FAFAFA]">{result.app_name}</span>
			<span class="text-sm text-[#525252]">{formatTime(result.timestamp)}</span>
			<span
				class="rounded px-1.5 py-0.5 text-[10px] font-medium uppercase {result.result_type === 'transcription'
					? 'bg-purple-500/10 text-purple-400'
					: 'bg-blue-500/10 text-blue-400'}"
			>
				{result.result_type === 'transcription' ? 'Audio' : 'OCR'}
			</span>
		</div>
		<div class="flex items-center gap-2">
			<button
				onclick={copyText}
				class="rounded-md bg-[#1C1C1C] px-3 py-1.5 text-xs text-[#A3A3A3] hover:text-[#FAFAFA]"
			>
				Copy Text
			</button>
			<button
				onclick={onclose}
				class="rounded-md bg-[#1C1C1C] px-3 py-1.5 text-xs text-[#A3A3A3] hover:text-[#FAFAFA]"
			>
				Close
			</button>
		</div>
	</div>

	<!-- Content -->
	<div class="flex flex-1 overflow-hidden">
		<!-- Screenshot -->
		{#if result.image_path && result.result_type !== 'transcription'}
			<div class="flex flex-1 items-center justify-center bg-[#050505] p-4">
				<img
					src={convertFileSrc(result.image_path)}
					alt="Capture at {formatTime(result.timestamp)}"
					class="max-h-full max-w-full rounded object-contain"
				/>
			</div>
		{/if}

		<!-- Text panel -->
		<div class="w-80 shrink-0 overflow-y-auto border-l border-[#262626] p-4">
			<h3 class="mb-3 text-xs font-medium uppercase tracking-wide text-[#525252]">
				{result.result_type === 'transcription' ? 'Transcription' : 'Extracted Text'}
			</h3>
			{#if ocrText}
				<pre class="whitespace-pre-wrap text-xs leading-relaxed text-[#A3A3A3]">{ocrText}</pre>
			{:else if result.result_type === 'transcription'}
				<p class="whitespace-pre-wrap text-xs leading-relaxed text-[#A3A3A3]">
					{result.snippet.replace(/<[^>]*>/g, '')}
				</p>
			{:else}
				<p class="text-xs text-[#525252]">Loading...</p>
			{/if}
		</div>
	</div>
</div>
