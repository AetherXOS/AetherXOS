<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { Terminal } from 'lucide-svelte';

	type DebugLine = {
		id: string;
		time: string;
		level: 'audit' | 'error' | 'critical';
		msg: string;
	};

	const {
		lines,
		isConnected,
		hasFilter
	}: {
		lines: DebugLine[];
		isConnected: boolean;
		hasFilter: boolean;
	} = $props();
</script>

<div class="card h-125 overflow-y-auto border border-white/10 bg-black p-10 font-mono text-xs">
	<div class="mb-4 flex items-center justify-between border-b border-white/10 pb-4 opacity-60">
		<div class="text-[10px] tracking-widest uppercase">{m.debug_live_trace()}</div>
		<div class="flex items-center gap-2">
			<Terminal size={14} />
			{isConnected ? m.debug_agent_online() : m.debug_agent_offline()}
		</div>
	</div>
	{#if lines.length === 0}
		<div class="opacity-50">
			{hasFilter ? m.debug_no_matches() : m.debug_empty()}
		</div>
	{:else}
		{#each lines as log (log.id)}
			<div class="flex gap-4 opacity-70">
				<span>{log.time}</span>
				<span
					class={log.level === 'critical'
						? 'text-error'
						: log.level === 'error'
							? 'text-warning'
							: 'text-primary'}
				>
					[{log.level.toUpperCase()}]
				</span>
				<span>{log.msg}</span>
			</div>
		{/each}
	{/if}
</div>
