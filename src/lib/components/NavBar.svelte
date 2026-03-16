<script lang="ts">
	import { page } from '$app/state';

	const links = [
		{ href: '/', label: 'Home' },
		{ href: '/timeline', label: 'Timeline' },
		{ href: '/chat', label: 'Chat' },
		{ href: '/settings', label: 'Settings' }
	];
</script>

<nav class="flex items-center gap-1 border-b border-[#262626] bg-[#0A0A0A] px-4 py-2">
	<span class="mr-4 text-sm font-bold text-[#FAFAFA]">Cortex</span>
	{#each links as link}
		<a
			href={link.href}
			class="rounded-md px-3 py-1 text-sm transition-colors {page.url.pathname === link.href
				? 'bg-[#1C1C1C] text-[#FAFAFA]'
				: 'text-[#525252] hover:text-[#A3A3A3]'}"
		>
			{link.label}
		</a>
	{/each}
</nav>
