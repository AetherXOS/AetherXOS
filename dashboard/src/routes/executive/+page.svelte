<script lang="ts">
	import { resolve } from '$app/paths';
	import ExecutiveArchitecturePanel from '$lib/components/executive/ExecutiveArchitecturePanel.svelte';
	import ExecutiveGuidedFlowPanel from '$lib/components/executive/ExecutiveGuidedFlowPanel.svelte';
	import MetricBox from '$lib/components/MetricBox.svelte';
	import { m } from '$lib/paraglide/messages';
	import { appState } from '$lib/state.svelte';
	import { formatUptime } from '$lib/utils/format';
	import {
		Activity,
		AlertCircle,
		CheckCircle,
		Compass,
		Globe,
		Settings,
		Terminal,
		Wifi
	} from 'lucide-svelte';

	// Status health map for sidebar subsystems
	const subsystems = $derived.by(() => {
		const ph = appState.pluginHealth;
		const base = [
			{ id: 'agent', label: 'Agent', ok: appState.isConnected },
			{ id: 'launcher', label: 'Launcher', ok: appState.launcherStatus.status === 'running' },
			{ id: 'stream', label: 'Live stream', ok: appState.streamConnected }
		];
		const plugins = Object.entries(ph).slice(0, 4).map(([k, v]) => ({
			id: k,
			label: k,
			ok: v.status === 'ok' || v.status === 'healthy'
		}));
		return [...base, ...plugins];
	});

	const guidanceActions = $derived.by(() => {
		type GuidanceHref =
			| '/settings#connection'
			| '/settings#authentication'
			| '/settings#verify'
			| '/operations'
			| '/control-center'
			| '/deep-debug';
		type GuidanceAction = { id: string; label: string; href: GuidanceHref; icon: typeof Settings };

		if (!appState.isConnected) {
			return [
				{ id: 'setup', label: 'Configure connection first', href: '/settings#connection', icon: Settings },
				{ id: 'auth', label: 'Verify authentication tokens', href: '/settings#authentication', icon: Settings },
				{ id: 'test', label: 'Run connectivity verification', href: '/settings#verify', icon: Wifi }
			] as GuidanceAction[];
		}

		return [
			{ id: 'ops', label: 'Review incidents in Operations', href: '/operations', icon: Compass },
			{ id: 'jobs', label: 'Manage jobs in Control Center', href: '/control-center', icon: Terminal },
			{ id: 'debug', label: 'Investigate traces in Deep Debug', href: '/deep-debug', icon: Activity }
		] as GuidanceAction[];
	});

	const executiveLayers = [
		{
			id: 'obs',
			title: m.exec_layer_obs_title(),
			detail: m.exec_layer_obs_detail(),
			subsections: [m.exec_layer_obs_sub1(), m.exec_layer_obs_sub2(), m.exec_layer_obs_sub3()]
		},
		{
			id: 'health',
			title: m.exec_layer_health_title(),
			detail: m.exec_layer_health_detail(),
			subsections: [
				m.exec_layer_health_sub1(),
				m.exec_layer_health_sub2(),
				m.exec_layer_health_sub3(),
				m.exec_layer_health_sub4()
			]
		},
		{
			id: 'audit',
			title: m.exec_layer_audit_title(),
			detail: m.exec_layer_audit_detail(),
			subsections: [m.exec_layer_audit_sub1(), m.exec_layer_audit_sub2(), m.exec_layer_audit_sub3()]
		}
	];

	const executiveDefinitions = [
		{ term: m.exec_term_latency(), meaning: m.exec_term_latency_meaning() },
		{ term: m.exec_term_stream(), meaning: m.exec_term_stream_meaning() },
		{ term: m.exec_term_subsystem(), meaning: m.exec_term_subsystem_meaning() }
	];

	const hasCriticalIncidents = $derived(
		appState.incidents.some((incident) => incident.severity === 'critical' && incident.status !== 'resolved')
	);
</script>

<div class="animate-in fade-in slide-in-from-bottom-4 space-y-12 pb-24 duration-1000">
	<header class="bg-base-200 group relative overflow-hidden rounded-[3rem] border border-white/5 p-6 shadow-2xl sm:p-8 lg:rounded-[3.5rem] lg:p-16">
		<div
			class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,var(--p)_0%,transparent_70%)] opacity-[0.02]"
		></div>

		<div
			class="relative z-10 flex flex-col items-start justify-between gap-10 lg:flex-row lg:items-center"
		>
			<div class="space-y-4 sm:space-y-6">
				<div class="flex items-center gap-4">
					<div
						class="badge badge-primary shadow-primary/20 px-4 py-3 text-[10px] font-black tracking-widest uppercase shadow-xl"
					>
						{m.exec_node_id()}
					</div>
					<div class="flex items-center gap-2 font-mono text-xs opacity-30">
						<Globe size={14} />
						{appState.agentUrl}
					</div>
				</div>
				<h1 class="text-5xl leading-[0.82] font-black tracking-tighter uppercase italic sm:text-6xl lg:text-9xl">
					{m.exec_title_part1()}
					<span class="text-primary not-italic underline decoration-white/5"
						>{m.exec_title_part2()}</span
					>
				</h1>
			</div>

			<div class="flex flex-col items-end gap-2">
				<div class="text-[10px] font-black tracking-widest uppercase italic opacity-20">
					{m.exec_link_latency()}
				</div>
				<div
					class="text-8xl font-black tracking-tighter tabular-nums {appState.latencyMs > 100
						? 'text-error'
						: 'text-primary'}"
				>
					{appState.latencyMs}<span
						class="ml-2 text-xl uppercase not-italic underline decoration-white/10 opacity-20"
						>ms</span
					>
				</div>
			</div>
		</div>
	</header>

	<div class="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-4">
		<MetricBox
			label={m.exec_metric_cpu()}
			value={appState.isConnected ? appState.metrics.cpu.toFixed(1) : '—'}
			unit="%"
			status={appState.metrics.cpu > 80 ? 'error' : appState.metrics.cpu > 60 ? 'warning' : 'success'}
		/>
		<MetricBox
			label={m.exec_metric_memory()}
			value={appState.isConnected ? appState.metrics.memory.toFixed(1) : '—'}
			unit="%"
			status="primary"
		/>
		<MetricBox
			label={m.exec_metric_disk()}
			value={appState.isConnected ? appState.metrics.disk.toFixed(1) : '—'}
			unit="%"
			status="info"
		/>
		<MetricBox
			label={m.exec_metric_uptime()}
			value={appState.isConnected ? formatUptime(appState.metrics.uptime) : '—'}
			status="success"
		/>
	</div>

	<ExecutiveGuidedFlowPanel isConnected={appState.isConnected} actions={guidanceActions} />

	<ExecutiveArchitecturePanel
		layers={executiveLayers}
		definitions={executiveDefinitions}
		isConnected={appState.isConnected}
		hasCriticalIncidents={hasCriticalIncidents}
		labels={{
			title: m.exec_architecture_title(),
			subtitle: m.exec_architecture_subtitle(),
			layersLabel: m.exec_layers_label(),
			autonomyTitle: m.exec_autonomy_title(),
			autonomyIncidentsTitle: m.exec_autonomy_incidents_title(),
			autonomyIncidentsDetail: m.exec_autonomy_incidents_detail(),
			autonomyRecoverTitle: m.exec_autonomy_recover_title(),
			autonomyRecoverDetail: m.exec_autonomy_recover_detail(),
			autonomyCriticalTitle: m.exec_autonomy_critical_title(),
			autonomyCriticalDetail: m.exec_autonomy_critical_detail(),
			autonomyProactiveTitle: m.exec_autonomy_proactive_title(),
			autonomyProactiveDetail: m.exec_autonomy_proactive_detail(),
			definitionsTitle: m.exec_definitions_title()
		}}
	/>

	<div class="grid grid-cols-1 gap-12 xl:grid-cols-12">
		<div class="space-y-10 xl:col-span-8">
			<div class="flex items-center justify-between">
				<h2 class="flex items-center gap-3 text-sm font-black tracking-[0.5em] uppercase italic">
					<Activity size={20} class="text-primary" />
					{m.exec_sector_heatmap()}
				</h2>
			</div>
			{#if appState.isConnected}
				<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
					{#each subsystems as sub (sub.id)}
						<div class="bg-base-200 group flex items-center justify-between rounded-2xl border border-white/5 px-5 py-4 transition-all hover:border-white/10">
							<span class="text-xs font-bold uppercase tracking-widest opacity-60 group-hover:opacity-100">{sub.label}</span>
							{#if sub.ok}
								<CheckCircle size={16} class="text-success shrink-0" />
							{:else}
								<AlertCircle size={16} class="text-error shrink-0 animate-pulse" />
							{/if}
						</div>
					{/each}
				</div>
			{:else}
				<div class="rounded-2xl border border-white/5 bg-base-200/50 p-8 text-center text-sm opacity-40">
					Subsystem data unavailable — agent offline
				</div>
			{/if}
		</div>

		<aside class="space-y-8 xl:col-span-4">
			<h2 class="text-sm font-black tracking-[0.5em] uppercase italic opacity-40">
				{m.exec_audit_stream()}
			</h2>
			<div
				class="card relative flex h-150 flex-col overflow-hidden rounded-[3.5rem] border border-white/5 bg-black/40 p-6 shadow-2xl"
			>
				<div class="custom-scrollbar flex-1 space-y-6 overflow-y-auto pr-4">
					{#each appState.auditLogs as log (log.id)}
						<div
							class="hover:bg-primary/5 hover:border-primary/20 group flex gap-5 rounded-3xl border border-white/5 bg-white/5 p-6 transition-all"
						>
							<span
								class="mt-1 shrink-0 font-mono text-[10px] tracking-tighter uppercase italic opacity-30"
								>{log.timestamp.split('T')[1].split('.')[0]}</span
							>
							<div class="space-y-2">
								<div
									class="group-hover:text-primary text-[11px] leading-tight font-black tracking-tight uppercase transition-colors"
								>
									{log.action}
								</div>
								<div class="text-[9px] font-bold tracking-widest uppercase opacity-20">
									{log.operator} · {log.status}
								</div>
							</div>
						</div>
					{/each}
				</div>
			</div>
		</aside>
	</div>
</div>
