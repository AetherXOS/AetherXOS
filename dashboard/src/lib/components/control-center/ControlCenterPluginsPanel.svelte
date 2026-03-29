<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { PluginHealthMap } from '$lib/types';
	import { formatUptime } from '$lib/utils/format';
	import { ShieldCheck } from 'lucide-svelte';

	interface Props {
		pluginRows: [string, PluginHealthMap[string]][];
		onRefresh: () => void;
	}

	let { pluginRows, onRefresh }: Props = $props();
</script>

<div class="card space-y-5 rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="flex items-center gap-2">
			<ShieldCheck size={18} class="text-primary" />
			<h2 class="text-lg font-black">{m.control_plugin_health()}</h2>
		</div>
		<button class="btn btn-outline btn-sm" onclick={onRefresh}>
			{m.control_refresh_plugins()}
		</button>
	</div>
	{#if pluginRows.length === 0}
		<p class="text-sm opacity-50">{m.control_no_plugins()}</p>
	{:else}
		<div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
			{#each pluginRows as [name, health] (name)}
				<div class="rounded-2xl border border-white/10 bg-base-100/30 p-4">
					<div class="flex items-center justify-between gap-4">
						<div class="text-xs font-black tracking-wide uppercase">{name}</div>
						<div class="badge {health.status === 'online' || health.status === 'ok' ? 'badge-success' : health.status === 'degraded' ? 'badge-warning' : 'badge-error'}">
							{health.status}
						</div>
					</div>
					{#if health.version}
						<div class="mt-2 text-[11px] opacity-50">{m.control_plugin_version()}: v{health.version}</div>
					{/if}
					{#if typeof health.uptime === 'number'}
						<div class="mt-1 text-[11px] opacity-50">{m.control_plugin_uptime()}: {formatUptime(health.uptime)}</div>
					{/if}
					{#if health.error}
						<div class="mt-2 text-[11px] text-error opacity-80">{m.control_plugin_error()}: {health.error}</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
