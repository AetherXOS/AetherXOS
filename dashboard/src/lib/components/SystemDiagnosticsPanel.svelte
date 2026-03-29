<script lang="ts">
	import { resolve } from '$app/paths';
	import { appState } from '$lib/state.svelte';
	import { AlertTriangle, CheckCircle, Cpu, Link2Off, RadioTower, ShieldAlert } from 'lucide-svelte';

	type Diagnostic = {
		id: string;
		severity: 'info' | 'warning' | 'error' | 'success';
		title: string;
		detail: string;
		actionLabel?: string;
		actionHref?:
			| '/settings#connection'
			| '/settings#authentication'
			| '/settings#verify'
			| '/operations'
			| '/executive'
			| '/control-center';
	};

	const diagnostics = $derived.by((): Diagnostic[] => {
		const cards: Diagnostic[] = [];

		if (!appState.agentUrl) {
			cards.push({
				id: 'missing-url',
				severity: 'error',
				title: 'Agent endpoint missing',
				detail: 'Set a valid Agent URL in Settings > Connection.',
				actionLabel: 'Open connection settings',
				actionHref: '/settings#connection'
			});
		}

		if (!appState.agentToken && !appState.launcherToken) {
			cards.push({
				id: 'missing-token',
				severity: 'warning',
				title: 'No authentication token configured',
				detail: 'If your agent requires auth, add at least one token in Settings > Authentication.',
				actionLabel: 'Open authentication settings',
				actionHref: '/settings#authentication'
			});
		}

		if (!appState.isConnected) {
			cards.push({
				id: 'offline',
				severity: 'error',
				title: 'Agent is offline',
				detail: 'Live metrics and incidents are unavailable until connectivity is restored.',
				actionLabel: 'Run connection verification',
				actionHref: '/settings#verify'
			});
		}

		if (appState.isConnected && !appState.streamConnected) {
			cards.push({
				id: 'stream',
				severity: 'warning',
				title: 'Live stream is disconnected',
				detail: 'System is connected but real-time streaming is not active. Dashboard may lag.',
				actionLabel: 'Check operations runbook',
				actionHref: '/operations'
			});
		}

		if (appState.apiCircuitOpen) {
			cards.push({
				id: 'circuit',
				severity: 'error',
				title: 'API circuit breaker is open',
				detail: 'Requests are being blocked after repeated failures. Validate endpoint and retry flow.',
				actionLabel: 'Review executive diagnostics',
				actionHref: '/executive'
			});
		}

		if (appState.isConnected && appState.metrics.cpu >= 90) {
			cards.push({
				id: 'cpu-hot',
				severity: 'warning',
				title: 'CPU pressure is very high',
				detail: 'CPU is above 90%. Consider reducing job load or scaling execution cadence.',
				actionLabel: 'Open control center',
				actionHref: '/control-center'
			});
		}

		if (cards.length === 0) {
			cards.push({
				id: 'healthy',
				severity: 'success',
				title: 'No active diagnostic issue detected',
				detail: 'Configuration, connectivity, and runtime signals look healthy.'
			});
		}

		return cards.slice(0, 4);
	});

	const toneClass: Record<Diagnostic['severity'], string> = {
		info: 'border-info/30 bg-info/10',
		warning: 'border-warning/30 bg-warning/10',
		error: 'border-error/30 bg-error/10',
		success: 'border-success/30 bg-success/10'
	};
</script>

<section class="card border border-white/5 bg-base-200/70 p-5 sm:p-6 space-y-4">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="text-sm font-black tracking-widest uppercase opacity-60">Automatic Diagnostics</div>
			<div class="text-xs opacity-50">Detected issues and direct recovery actions.</div>
		</div>
		<div class="badge badge-outline">{diagnostics.length} card(s)</div>
	</div>

	<div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
		{#each diagnostics as d (d.id)}
			<div class="rounded-xl border p-4 text-xs space-y-2 {toneClass[d.severity]}">
				<div class="flex items-center gap-2 font-black uppercase tracking-wide">
					{#if d.severity === 'error'}
						<ShieldAlert size={13} />
					{:else if d.severity === 'warning'}
						<AlertTriangle size={13} />
					{:else if d.severity === 'success'}
						<CheckCircle size={13} />
					{:else}
						<RadioTower size={13} />
					{/if}
					{d.title}
				</div>
				<div class="opacity-75">{d.detail}</div>
				{#if d.actionHref && d.actionLabel}
					<a href={resolve(d.actionHref)} class="btn btn-xs btn-outline mt-1">{d.actionLabel}</a>
				{/if}
			</div>
		{/each}
	</div>

	<div class="flex flex-wrap items-center gap-3 text-[11px] opacity-50">
		<span class="inline-flex items-center gap-1"><Link2Off size={12} /> Connectivity</span>
		<span class="inline-flex items-center gap-1"><ShieldAlert size={12} /> Reliability</span>
		<span class="inline-flex items-center gap-1"><Cpu size={12} /> Resource pressure</span>
	</div>
</section>
