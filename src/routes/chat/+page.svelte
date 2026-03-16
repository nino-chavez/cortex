<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';

	interface Citation {
		capture_id: number;
		timestamp: string;
		app_name: string;
		snippet: string;
	}

	interface ChatResponse {
		text: string;
		citations: Citation[];
	}

	interface Message {
		role: 'user' | 'assistant';
		text: string;
		citations?: Citation[];
	}

	let messages = $state<Message[]>([]);
	let input = $state('');
	let loading = $state(false);
	let messagesEl: HTMLDivElement | undefined = $state();

	async function send() {
		const query = input.trim();
		if (!query || loading) return;

		messages.push({ role: 'user', text: query });
		input = '';
		loading = true;

		try {
			const response = await invoke<ChatResponse>('chat_message', { message: query });
			messages.push({
				role: 'assistant',
				text: response.text,
				citations: response.citations
			});
		} catch (e) {
			messages.push({
				role: 'assistant',
				text: `Error: ${e}. Make sure Ollama is running with \`ollama serve\` and you've pulled llama3.1 with \`ollama pull llama3.1\`.`
			});
		} finally {
			loading = false;
		}

		// Scroll to bottom
		requestAnimationFrame(() => {
			messagesEl?.scrollTo({ top: messagesEl.scrollHeight, behavior: 'smooth' });
		});
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			send();
		}
	}

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleTimeString('en-US', {
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="flex h-screen flex-col bg-[#0A0A0A] text-[#FAFAFA]">
	<!-- Header -->
	<div class="border-b border-[#262626] px-6 py-3">
		<h1 class="text-lg font-semibold">Chat</h1>
		<p class="text-xs text-[#525252]">Ask questions about your capture history</p>
	</div>

	<!-- Messages -->
	<div bind:this={messagesEl} class="flex-1 overflow-y-auto px-6 py-4">
		{#if messages.length === 0}
			<div class="flex h-full flex-col items-center justify-center gap-4 text-[#525252]">
				<p class="text-lg">What would you like to recall?</p>
				<div class="flex flex-wrap justify-center gap-2">
					{#each ['What did I work on this morning?', 'Show me anything about the API', 'What meetings did I have?'] as suggestion}
						<button
							onclick={() => {
								input = suggestion;
								send();
							}}
							class="rounded-lg border border-[#262626] bg-[#141414] px-3 py-2 text-sm hover:border-[#404040]"
						>
							{suggestion}
						</button>
					{/each}
				</div>
			</div>
		{:else}
			{#each messages as msg}
				<div class="mb-4 {msg.role === 'user' ? 'text-right' : ''}">
					<div
						class="inline-block max-w-[80%] rounded-xl px-4 py-3 text-sm {msg.role === 'user'
							? 'bg-blue-600 text-white'
							: 'bg-[#141414] text-[#D4D4D4]'}"
					>
						<p class="whitespace-pre-wrap">{msg.text}</p>

						{#if msg.citations && msg.citations.length > 0}
							<div class="mt-3 flex flex-wrap gap-1.5 border-t border-[#262626] pt-2">
								{#each msg.citations as cite}
									<a
										href="/timeline?capture={cite.capture_id}"
										class="inline-flex items-center gap-1 rounded bg-[#1C1C1C] px-2 py-0.5 text-[10px] text-[#A3A3A3] hover:text-[#FAFAFA]"
									>
										{formatTime(cite.timestamp)} - {cite.app_name}
									</a>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			{/each}

			{#if loading}
				<div class="mb-4">
					<div class="inline-block rounded-xl bg-[#141414] px-4 py-3 text-sm text-[#525252]">
						Thinking...
					</div>
				</div>
			{/if}
		{/if}
	</div>

	<!-- Input -->
	<div class="border-t border-[#262626] p-4">
		<div class="flex items-center gap-3 rounded-xl border border-[#262626] bg-[#141414] px-4 py-2">
			<input
				bind:value={input}
				onkeydown={handleKeydown}
				type="text"
				placeholder="Ask about your history..."
				class="flex-1 bg-transparent text-sm text-[#FAFAFA] placeholder-[#525252] outline-none"
				disabled={loading}
			/>
			<button
				onclick={send}
				disabled={loading || !input.trim()}
				class="rounded-lg bg-blue-600 px-4 py-1.5 text-sm font-medium text-white disabled:opacity-50"
			>
				Send
			</button>
		</div>
	</div>
</div>
