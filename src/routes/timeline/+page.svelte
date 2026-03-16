<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import Stage from '$lib/components/timeline/Stage.svelte';
	import Filmstrip from '$lib/components/timeline/Filmstrip.svelte';

	interface CaptureRow {
		id: number;
		timestamp: string;
		app_name: string;
		bundle_id: string;
		window_title: string;
		display_id: number;
		image_path: string;
		image_hash: string;
		is_private: boolean;
	}

	let captures = $state<CaptureRow[]>([]);
	let activeCapture = $state<CaptureRow | null>(null);
	let ocrText = $state<string | null>(null);
	let selectedDate = $state(new Date().toISOString().split('T')[0]);

	// Load captures for selected date
	$effect(() => {
		loadCaptures(selectedDate);
	});

	async function loadCaptures(date: string) {
		try {
			captures = await invoke<CaptureRow[]>('get_captures_for_day', { date });
			if (captures.length > 0 && !activeCapture) {
				selectCapture(captures[captures.length - 1]); // Start at most recent
			}
		} catch (e) {
			console.error('Failed to load captures:', e);
		}
	}

	async function selectCapture(capture: CaptureRow) {
		activeCapture = capture;
		try {
			ocrText = await invoke<string | null>('get_capture_ocr_text', { captureId: capture.id });
		} catch {
			ocrText = null;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (!activeCapture || captures.length === 0) return;
		const idx = captures.findIndex((c) => c.id === activeCapture!.id);

		if (e.key === 'ArrowRight' && idx < captures.length - 1) {
			e.preventDefault();
			selectCapture(captures[idx + 1]);
		} else if (e.key === 'ArrowLeft' && idx > 0) {
			e.preventDefault();
			selectCapture(captures[idx - 1]);
		}
	}

	// Downsample to ~1 per minute for filmstrip
	let filmstripCaptures = $derived.by(() => {
		if (captures.length <= 60) return captures;
		const interval = Math.ceil(captures.length / (captures.length / 12));
		return captures.filter((_, i) => i % interval === 0 || i === captures.length - 1);
	});
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex h-screen flex-col bg-[#0A0A0A] text-[#FAFAFA]">
	<!-- Header -->
	<div class="flex items-center justify-between border-b border-[#262626] px-6 py-3">
		<h1 class="text-lg font-semibold">Timeline</h1>
		<div class="flex items-center gap-3">
			<input
				type="date"
				bind:value={selectedDate}
				class="rounded-md border border-[#262626] bg-[#141414] px-3 py-1.5 text-sm text-[#FAFAFA]"
			/>
			<button
				onclick={() => {
					selectedDate = new Date().toISOString().split('T')[0];
				}}
				class="rounded-md bg-[#1C1C1C] px-3 py-1.5 text-sm text-[#A3A3A3] hover:text-[#FAFAFA]"
			>
				Jump to Now
			</button>
			<span class="text-sm text-[#525252]">{captures.length} captures</span>
		</div>
	</div>

	<!-- Stage -->
	<div class="flex-1 overflow-hidden">
		{#if activeCapture}
			<Stage capture={activeCapture} {ocrText} />
		{:else}
			<div class="flex h-full items-center justify-center text-[#525252]">
				{#if captures.length === 0}
					No captures for this day
				{:else}
					Select a capture from the filmstrip
				{/if}
			</div>
		{/if}
	</div>

	<!-- Filmstrip -->
	<div class="border-t border-[#262626]">
		<Filmstrip
			captures={filmstripCaptures}
			activeCaptureId={activeCapture?.id ?? null}
			onselect={selectCapture}
		/>
	</div>
</div>
