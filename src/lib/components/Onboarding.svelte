<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	interface PermissionStatus {
		screen_recording: boolean;
		accessibility: boolean;
	}

	let { oncomplete }: { oncomplete: () => void } = $props();

	let step = $state(0);
	let permissions = $state<PermissionStatus>({ screen_recording: false, accessibility: false });
	let checking = $state(false);

	const steps = [
		{
			title: 'Welcome to Cortex',
			description: 'Your local-first AI memory. Everything runs on your Mac — no cloud, no accounts, no telemetry.',
		},
		{
			title: 'Screen Recording',
			description: 'Cortex needs Screen Recording permission to capture screenshots. Click "Grant" to open System Settings, then enable Cortex.',
			permission: 'screen_recording' as const,
		},
		{
			title: 'Accessibility',
			description: 'Accessibility permission lets Cortex detect which app and window you\'re using. Without it, captures will show "Unknown" for app names.',
			permission: 'accessibility' as const,
		},
		{
			title: 'You\'re all set',
			description: 'Cortex is ready. Click the tray icon to start capturing, press Cmd+Shift+Space to search, or explore the Timeline.',
		},
	];

	async function checkPermissions() {
		checking = true;
		permissions = await invoke<PermissionStatus>('check_permissions');
		checking = false;
	}

	async function grantPermission(type: string) {
		if (type === 'screen_recording') {
			// This triggers the system dialog
			await invoke('check_permissions');
		}
		// Poll for grant
		for (let i = 0; i < 30; i++) {
			await new Promise((r) => setTimeout(r, 1000));
			await checkPermissions();
			if (type === 'screen_recording' && permissions.screen_recording) break;
			if (type === 'accessibility' && permissions.accessibility) break;
		}
	}

	function next() {
		if (step < steps.length - 1) {
			step++;
		} else {
			oncomplete();
		}
	}

	$effect(() => {
		checkPermissions();
	});
</script>

<div class="flex h-screen items-center justify-center bg-[#0A0A0A] text-[#FAFAFA]">
	<div class="w-[480px] rounded-2xl border border-[#262626] bg-[#141414] p-8">
		<!-- Progress -->
		<div class="mb-6 flex gap-1">
			{#each steps as _, i}
				<div class="h-1 flex-1 rounded-full {i <= step ? 'bg-blue-500' : 'bg-[#262626]'}"></div>
			{/each}
		</div>

		<!-- Content -->
		<h2 class="text-xl font-bold">{steps[step].title}</h2>
		<p class="mt-2 text-sm leading-relaxed text-[#A3A3A3]">{steps[step].description}</p>

		<!-- Permission check -->
		{#if steps[step].permission}
			{@const perm = steps[step].permission}
			<div class="mt-4 rounded-lg border border-[#262626] bg-[#0A0A0A] p-4">
				<div class="flex items-center justify-between">
					<span class="text-sm">{perm === 'screen_recording' ? 'Screen Recording' : 'Accessibility'}</span>
					{#if permissions[perm]}
						<span class="rounded bg-green-500/10 px-2 py-0.5 text-xs text-green-400">Granted</span>
					{:else}
						<button
							onclick={() => grantPermission(perm)}
							class="rounded bg-blue-600 px-3 py-1 text-xs font-medium text-white"
							disabled={checking}
						>
							{checking ? 'Checking...' : 'Grant'}
						</button>
					{/if}
				</div>
			</div>
		{/if}

		<!-- Navigation -->
		<div class="mt-6 flex justify-between">
			{#if step > 0}
				<button
					onclick={() => step--}
					class="rounded-lg px-4 py-2 text-sm text-[#525252] hover:text-[#A3A3A3]"
				>
					Back
				</button>
			{:else}
				<div></div>
			{/if}

			<button
				onclick={next}
				class="rounded-lg bg-blue-600 px-6 py-2 text-sm font-medium text-white"
			>
				{step === steps.length - 1 ? 'Get Started' : 'Next'}
			</button>
		</div>
	</div>
</div>
