<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import Onboarding from '$lib/components/Onboarding.svelte';

	interface CortexConfig {
		general: { capture_interval_secs: number; hotkey: string };
		retention: { screenshots_days: number; audio_days: number; keep_text_forever: boolean };
		privacy: { excluded_apps: string[] };
		audio: { system_audio_enabled: boolean; microphone_enabled: boolean };
	}

	interface StorageStats {
		total_bytes: number;
		screenshots_bytes: number;
		capture_count: number;
	}

	let showOnboarding = $state(false);
	let stats = $state<StorageStats | null>(null);

	$effect(() => {
		// Check if this is first run (no captures yet)
		invoke<StorageStats>('get_storage_stats').then((s) => {
			stats = s;
			if (s.capture_count === 0 && s.total_bytes < 1000) {
				showOnboarding = true;
			}
		});
	});

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
		return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
	}
</script>

{#if showOnboarding}
	<Onboarding oncomplete={() => (showOnboarding = false)} />
{:else}
	<div class="flex h-screen flex-col items-center justify-center bg-[#0A0A0A] text-[#FAFAFA]">
		<div class="text-center">
			<h1 class="text-4xl font-bold tracking-tight">Cortex</h1>
			<p class="mt-2 text-[#A3A3A3]">Local-first AI memory</p>
		</div>

		<nav class="mt-8 flex gap-4">
			<a
				href="/timeline"
				class="rounded-lg border border-[#262626] bg-[#141414] px-6 py-3 text-sm font-medium text-[#A3A3A3] transition-colors hover:border-[#404040] hover:text-[#FAFAFA]"
			>
				Timeline
			</a>
			<a
				href="/chat"
				class="rounded-lg border border-[#262626] bg-[#141414] px-6 py-3 text-sm font-medium text-[#A3A3A3] transition-colors hover:border-[#404040] hover:text-[#FAFAFA]"
			>
				Chat
			</a>
			<a
				href="/settings"
				class="rounded-lg border border-[#262626] bg-[#141414] px-6 py-3 text-sm font-medium text-[#A3A3A3] transition-colors hover:border-[#404040] hover:text-[#FAFAFA]"
			>
				Settings
			</a>
		</nav>

		{#if stats}
			<div class="mt-6 flex gap-6 text-xs text-[#525252]">
				<span>{stats.capture_count.toLocaleString()} captures</span>
				<span>{formatBytes(stats.total_bytes)} stored</span>
			</div>
		{/if}

		<p class="mt-4 text-xs text-[#525252]">Press Cmd+Shift+Space to search from anywhere</p>
	</div>
{/if}
