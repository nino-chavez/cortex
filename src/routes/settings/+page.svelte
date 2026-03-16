<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	interface CortexConfig {
		general: { capture_interval_secs: number; hotkey: string };
		retention: { screenshots_days: number; audio_days: number; keep_text_forever: boolean };
		privacy: { excluded_apps: string[] };
		audio: { system_audio_enabled: boolean; microphone_enabled: boolean };
	}

	interface StorageStats {
		total_bytes: number;
		screenshots_bytes: number;
		audio_bytes: number;
		database_bytes: number;
		capture_count: number;
	}

	let config = $state<CortexConfig | null>(null);
	let stats = $state<StorageStats | null>(null);
	let saving = $state(false);
	let newExcludedApp = $state('');

	$effect(() => {
		loadSettings();
	});

	async function loadSettings() {
		config = await invoke<CortexConfig>('get_settings');
		stats = await invoke<StorageStats>('get_storage_stats');
	}

	async function save() {
		if (!config) return;
		saving = true;
		await invoke('update_settings', { settings: config });
		saving = false;
	}

	async function cleanup() {
		const result = await invoke<{ deleted_screenshots: number; deleted_audio: number }>('run_cleanup');
		alert(`Cleaned up ${result.deleted_screenshots} screenshots and ${result.deleted_audio} audio files.`);
		stats = await invoke<StorageStats>('get_storage_stats');
	}

	function addExcludedApp() {
		if (!config || !newExcludedApp.trim()) return;
		config.privacy.excluded_apps = [...config.privacy.excluded_apps, newExcludedApp.trim()];
		newExcludedApp = '';
		save();
	}

	function removeExcludedApp(app: string) {
		if (!config) return;
		config.privacy.excluded_apps = config.privacy.excluded_apps.filter((a) => a !== app);
		save();
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
		return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
	}
</script>

<div class="min-h-screen bg-[#0A0A0A] text-[#FAFAFA]">
	<div class="mx-auto max-w-2xl px-6 py-8">
		<h1 class="mb-8 text-2xl font-bold">Settings</h1>

		{#if config}
			<!-- General -->
			<section class="mb-8">
				<h2 class="mb-4 text-sm font-medium uppercase tracking-wide text-[#525252]">General</h2>
				<div class="space-y-4 rounded-xl border border-[#262626] bg-[#141414] p-4">
					<label class="flex items-center justify-between">
						<span class="text-sm text-[#A3A3A3]">Capture interval (seconds)</span>
						<input
							type="number"
							min="1"
							max="60"
							bind:value={config.general.capture_interval_secs}
							onchange={save}
							class="w-20 rounded border border-[#262626] bg-[#0A0A0A] px-2 py-1 text-right text-sm text-[#FAFAFA]"
						/>
					</label>
				</div>
			</section>

			<!-- Audio -->
			<section class="mb-8">
				<h2 class="mb-4 text-sm font-medium uppercase tracking-wide text-[#525252]">Audio</h2>
				<div class="space-y-4 rounded-xl border border-[#262626] bg-[#141414] p-4">
					<label class="flex items-center justify-between">
						<span class="text-sm text-[#A3A3A3]">System audio capture</span>
						<input type="checkbox" bind:checked={config.audio.system_audio_enabled} onchange={save} />
					</label>
					<label class="flex items-center justify-between">
						<span class="text-sm text-[#A3A3A3]">Microphone capture</span>
						<input type="checkbox" bind:checked={config.audio.microphone_enabled} onchange={save} />
					</label>
				</div>
			</section>

			<!-- Privacy -->
			<section class="mb-8">
				<h2 class="mb-4 text-sm font-medium uppercase tracking-wide text-[#525252]">Privacy</h2>
				<div class="space-y-4 rounded-xl border border-[#262626] bg-[#141414] p-4">
					<p class="text-xs text-[#525252]">Excluded apps will never be captured (use bundle IDs like com.1password).</p>
					<div class="flex gap-2">
						<input
							bind:value={newExcludedApp}
							placeholder="com.example.app"
							class="flex-1 rounded border border-[#262626] bg-[#0A0A0A] px-3 py-1.5 text-sm text-[#FAFAFA] placeholder-[#525252]"
							onkeydown={(e) => e.key === 'Enter' && addExcludedApp()}
						/>
						<button onclick={addExcludedApp} class="rounded bg-[#1C1C1C] px-3 py-1.5 text-sm text-[#A3A3A3] hover:text-[#FAFAFA]">Add</button>
					</div>
					{#each config.privacy.excluded_apps as app}
						<div class="flex items-center justify-between rounded bg-[#0A0A0A] px-3 py-1.5">
							<span class="text-sm text-[#A3A3A3]">{app}</span>
							<button onclick={() => removeExcludedApp(app)} class="text-xs text-red-400 hover:text-red-300">Remove</button>
						</div>
					{/each}
				</div>
			</section>

			<!-- Retention -->
			<section class="mb-8">
				<h2 class="mb-4 text-sm font-medium uppercase tracking-wide text-[#525252]">Retention</h2>
				<div class="space-y-4 rounded-xl border border-[#262626] bg-[#141414] p-4">
					<label class="flex items-center justify-between">
						<span class="text-sm text-[#A3A3A3]">Keep screenshots (days)</span>
						<input
							type="number"
							min="1"
							max="365"
							bind:value={config.retention.screenshots_days}
							onchange={save}
							class="w-20 rounded border border-[#262626] bg-[#0A0A0A] px-2 py-1 text-right text-sm text-[#FAFAFA]"
						/>
					</label>
					<label class="flex items-center justify-between">
						<span class="text-sm text-[#A3A3A3]">Keep audio (days)</span>
						<input
							type="number"
							min="1"
							max="365"
							bind:value={config.retention.audio_days}
							onchange={save}
							class="w-20 rounded border border-[#262626] bg-[#0A0A0A] px-2 py-1 text-right text-sm text-[#FAFAFA]"
						/>
					</label>
					<label class="flex items-center justify-between">
						<span class="text-sm text-[#A3A3A3]">Keep extracted text forever</span>
						<input type="checkbox" bind:checked={config.retention.keep_text_forever} onchange={save} />
					</label>
					<button onclick={cleanup} class="rounded bg-red-500/10 px-3 py-1.5 text-sm text-red-400 hover:bg-red-500/20">
						Run Cleanup Now
					</button>
				</div>
			</section>

			<!-- Storage -->
			{#if stats}
				<section class="mb-8">
					<h2 class="mb-4 text-sm font-medium uppercase tracking-wide text-[#525252]">Storage</h2>
					<div class="space-y-3 rounded-xl border border-[#262626] bg-[#141414] p-4">
						<div class="flex justify-between text-sm">
							<span class="text-[#A3A3A3]">Total</span>
							<span class="text-[#FAFAFA]">{formatBytes(stats.total_bytes)}</span>
						</div>
						<div class="flex justify-between text-sm">
							<span class="text-[#A3A3A3]">Screenshots</span>
							<span>{formatBytes(stats.screenshots_bytes)}</span>
						</div>
						<div class="flex justify-between text-sm">
							<span class="text-[#A3A3A3]">Audio</span>
							<span>{formatBytes(stats.audio_bytes)}</span>
						</div>
						<div class="flex justify-between text-sm">
							<span class="text-[#A3A3A3]">Database</span>
							<span>{formatBytes(stats.database_bytes)}</span>
						</div>
						<div class="flex justify-between text-sm">
							<span class="text-[#A3A3A3]">Captures</span>
							<span>{stats.capture_count.toLocaleString()}</span>
						</div>
					</div>
				</section>
			{/if}

			<!-- About -->
			<section class="mb-8">
				<h2 class="mb-4 text-sm font-medium uppercase tracking-wide text-[#525252]">About</h2>
				<div class="rounded-xl border border-[#262626] bg-[#141414] p-4 text-sm text-[#A3A3A3]">
					<p class="font-medium text-[#FAFAFA]">Cortex v0.1.0</p>
					<p class="mt-1">Local-first AI memory for macOS</p>
					<p class="mt-1">Signal X Studio</p>
					<p class="mt-2 text-xs text-[#525252]">All data stored locally at ~/.cortex/</p>
				</div>
			</section>
		{:else}
			<p class="text-[#525252]">Loading settings...</p>
		{/if}
	</div>
</div>
