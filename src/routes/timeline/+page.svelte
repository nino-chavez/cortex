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

	let zoomLevel = $state(1); // 1 = 1/min, 2 = 1/30s, 3 = all (5s)

	// Smart downsampling based on zoom level
	let filmstripCaptures = $derived.by(() => {
		if (captures.length === 0) return [];

		if (zoomLevel >= 3 || captures.length <= 60) return captures;

		// Calculate target count based on zoom
		const targetPerMinute = zoomLevel === 1 ? 1 : 2; // 1/min or 2/min
		const totalMinutes = Math.max(1, Math.ceil(captures.length / 12)); // ~12 captures per min at 5s
		const targetCount = totalMinutes * targetPerMinute;
		const step = Math.max(1, Math.floor(captures.length / targetCount));

		return captures.filter((_, i) => i % step === 0 || i === captures.length - 1);
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
			<div class="flex items-center gap-1 rounded-md border border-[#262626] bg-[#141414] px-1">
				<button
					onclick={() => (zoomLevel = Math.max(1, zoomLevel - 1))}
					class="px-1.5 py-0.5 text-sm text-[#525252] hover:text-[#FAFAFA]"
					disabled={zoomLevel <= 1}
				>-</button>
				<span class="px-1 text-xs text-[#A3A3A3]">
					{zoomLevel === 1 ? '1/min' : zoomLevel === 2 ? '1/30s' : 'All'}
				</span>
				<button
					onclick={() => (zoomLevel = Math.min(3, zoomLevel + 1))}
					class="px-1.5 py-0.5 text-sm text-[#525252] hover:text-[#FAFAFA]"
					disabled={zoomLevel >= 3}
				>+</button>
			</div>
			<span class="text-sm text-[#525252]">{filmstripCaptures.length}/{captures.length}</span>
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
