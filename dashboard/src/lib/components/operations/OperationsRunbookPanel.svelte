<script lang="ts">
	import { resolve } from '$app/paths';
	import { AlertTriangle, CheckCircle, Compass, RefreshCw, Settings, ShieldAlert } from 'lucide-svelte';

	interface RunbookState {
		offline: boolean;
		hasActive: boolean;
		openCount: number;
		investigatingCount: number;
		resolvedCount: number;
	}

	interface Labels {
		badgeOffline: string;
		badgeAttention: string;
		badgeStable: string;
		detectTitle: string;
		detectDetail: string;
		detectOfflineNote: string;
		detectOnlineNote: string;
		triageTitle: string;
		triageDetail: string;
		resolveTitle: string;
		resolveDetail: string;
		configureConnection: string;
		refreshFeed: string;
		focusCritical: string;
		focusInvestigating: string;
	}

	interface Props {
		runbook: RunbookState;
		isRefreshing: boolean;
		labels: Labels;
		onRefresh: () => void;
		onFocusCritical: () => void;
		onFocusInvestigating: () => void;
	}

	let { runbook, isRefreshing, labels, onRefresh, onFocusCritical, onFocusInvestigating }: Props =
		$props();
</script>

<section class="card bg-base-200 space-y-5 border border-white/5 p-6">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="text-sm font-black tracking-widest uppercase opacity-60">Operations Runbook</div>
			<div class="text-xs opacity-50">Follow this sequence: detect, triage, resolve.</div>
		</div>
		<div class="badge {runbook.offline ? 'badge-warning' : runbook.hasActive ? 'badge-error' : 'badge-success'}">
			{runbook.offline ? labels.badgeOffline : runbook.hasActive ? labels.badgeAttention : labels.badgeStable}
		</div>
	</div>

	<div class="grid grid-cols-1 gap-3 lg:grid-cols-3 text-xs">
		<div class="rounded-xl border border-white/10 bg-base-100/50 p-4 space-y-1">
			<div class="flex items-center gap-2 font-black uppercase tracking-wide"><AlertTriangle size={13} class="text-warning" /> {labels.detectTitle}</div>
			<div class="opacity-70">{labels.detectDetail}</div>
			<div class="opacity-50">{runbook.offline ? labels.detectOfflineNote : labels.detectOnlineNote}</div>
		</div>
		<div class="rounded-xl border border-white/10 bg-base-100/50 p-4 space-y-1">
			<div class="flex items-center gap-2 font-black uppercase tracking-wide"><ShieldAlert size={13} class="text-error" /> {labels.triageTitle}</div>
			<div class="opacity-70">Open: {runbook.openCount} · Investigating: {runbook.investigatingCount}</div>
			<div class="opacity-50">{labels.triageDetail}</div>
		</div>
		<div class="rounded-xl border border-white/10 bg-base-100/50 p-4 space-y-1">
			<div class="flex items-center gap-2 font-black uppercase tracking-wide"><CheckCircle size={13} class="text-success" /> {labels.resolveTitle}</div>
			<div class="opacity-70">Resolved: {runbook.resolvedCount}</div>
			<div class="opacity-50">{labels.resolveDetail}</div>
		</div>
	</div>

	<div class="flex flex-wrap gap-2">
		{#if runbook.offline}
			<a href={resolve('/settings#connection')} class="btn btn-sm btn-warning gap-2">
				<Settings size={14} />
				{labels.configureConnection}
			</a>
		{/if}
		<button class="btn btn-sm btn-outline gap-2" onclick={onRefresh} disabled={isRefreshing}>
			<RefreshCw size={14} class={isRefreshing ? 'animate-spin' : ''} />
			{labels.refreshFeed}
		</button>
		<button class="btn btn-sm btn-outline gap-2" onclick={onFocusCritical} disabled={runbook.offline}>
			<ShieldAlert size={14} />
			{labels.focusCritical}
		</button>
		<button class="btn btn-sm btn-outline gap-2" onclick={onFocusInvestigating} disabled={runbook.offline}>
			<Compass size={14} />
			{labels.focusInvestigating}
		</button>
	</div>
</section>
