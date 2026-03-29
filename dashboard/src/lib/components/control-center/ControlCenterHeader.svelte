<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { LauncherStatus } from '$lib/types';
	import type { LauncherAction } from './types';
	import { Play, Power, RotateCcw } from 'lucide-svelte';

	interface Props {
		launcherStatus: LauncherStatus;
		liveMode: string;
		streamConnected: boolean;
		launcherBusy: boolean;
		latencyMs: number;
		retryCount: number;
		apiCircuitOpen: boolean;
		connectionQuality: 'stable' | 'recovering' | 'degraded' | 'offline';
		onLauncherAction: (action: LauncherAction) => void;
	}

	let {
		launcherStatus,
		liveMode,
		streamConnected,
		launcherBusy,
		latencyMs,
		retryCount,
		apiCircuitOpen,
		connectionQuality,
		onLauncherAction
	}: Props = $props();
</script>

<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
		<div class="space-y-1 text-xs font-black tracking-widest uppercase opacity-70 sm:text-sm">
			<div>
				{m.control_launcher_status()}:
				<span class="text-primary">{launcherStatus.status}</span>
			</div>
			<div>
				{m.control_live_mode()}: <span class="text-primary">{liveMode}</span>
				{streamConnected ? ` (${m.control_stream_connected()})` : ` (${m.control_polling_fallback()})`}
			</div>
			<div class="flex flex-wrap items-center gap-2 pt-1 text-[10px] font-bold tracking-wide">
				<span
					class="badge badge-sm {connectionQuality === 'stable'
						? 'badge-success'
						: connectionQuality === 'recovering'
							? 'badge-info'
							: connectionQuality === 'degraded'
								? 'badge-warning'
								: 'badge-error'}"
				>
					link: {connectionQuality}
				</span>
				<span class="badge badge-ghost badge-sm border-white/10">latency {Math.round(latencyMs)}ms</span>
				{#if retryCount > 0}
					<span class="badge badge-ghost badge-sm border-warning/25">retry x{retryCount}</span>
				{/if}
				{#if apiCircuitOpen}
					<span class="badge badge-warning badge-sm">circuit open</span>
				{/if}
			</div>
		</div>
		<div class="flex flex-wrap gap-2">
			<button class="btn btn-outline btn-sm" onclick={() => onLauncherAction('start')} disabled={launcherBusy}>
				<Play size={14} />
				{m.control_start_agent()}
			</button>
			<button class="btn btn-outline btn-sm" onclick={() => onLauncherAction('stop')} disabled={launcherBusy}>
				<Power size={14} />
				{m.control_stop_agent()}
			</button>
			<button class="btn btn-outline btn-sm" onclick={() => onLauncherAction('restart')} disabled={launcherBusy}>
				<RotateCcw size={14} />
				{m.control_restart_agent()}
			</button>
		</div>
	</div>
</div>
