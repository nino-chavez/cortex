<script lang="ts">
	let { value = $bindable('') }: { value: string } = $props();

	let inputEl: HTMLInputElement | undefined = $state();

	// Auto-focus on mount
	$effect(() => {
		inputEl?.focus();
	});
</script>

<div class="flex items-center gap-3">
	<!-- Search icon -->
	<svg class="h-5 w-5 shrink-0 text-[#525252]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
		<path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
	</svg>

	<input
		bind:this={inputEl}
		bind:value
		type="text"
		placeholder="Search your history..."
		class="h-10 w-full bg-transparent text-lg text-[#FAFAFA] placeholder-[#525252] outline-none"
	/>
</div>
