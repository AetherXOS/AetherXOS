<script lang="ts">
	import { SYNC_INTERVAL_MAX_MS, SYNC_INTERVAL_MIN_MS } from '$lib/config/dashboard-settings';
	import { m } from '$lib/paraglide/messages';
	import { CheckCircle, Globe, Server, Timer, WifiOff, Zap } from 'lucide-svelte';

	interface Props {
		localUrl: string;
		localSyncIntervalMs: number;
		isConnected: boolean;
		isTesting: boolean;
		testResult: 'idle' | 'ok' | 'fail';
		onUrlChange: (v: string) => void;
		onIntervalChange: (v: number) => void;
		onTestConnection: () => void;
		onApplyLocalDefaults: () => void;
	}

	let {
		localUrl,
		localSyncIntervalMs,
		isConnected,
		isTesting,
		testResult,
		onUrlChange,
		onIntervalChange,
		onTestConnection,
		onApplyLocalDefaults
	}: Props = $props();
</script>

<section id="connection" class="card rounded-2xl border border-white/5 bg-base-200/80 p-5 sm:p-6 space-y-5 scroll-mt-28">
	<div class="flex items-center gap-3 border-b border-white/5 pb-4">
		<div class="bg-primary/15 flex h-9 w-9 items-center justify-center rounded-xl">
			<Server size={18} class="text-primary" />
		</div>
		<div>
			<div class="text-sm font-black uppercase tracking-wider">Connection</div>
			<div class="text-xs opacity-50">Agent endpoint and sync settings</div>
		</div>
		<div class="ml-auto flex items-center gap-2">
			{#if isConnected}
				<div class="badge badge-success gap-1"><CheckCircle size={11} /> Connected</div>
			{:else}
				<div class="badge badge-error gap-1"><WifiOff size={11} /> Offline</div>
			{/if}
		</div>
	</div>

	<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
		<div id="connection-endpoint" class="form-control gap-2 scroll-mt-28">
			<label class="flex items-center gap-2 text-xs font-bold uppercase opacity-60">
				<Globe size={13} />{m.settings_endpoint()}
			</label>
			<input class="input bg-base-100" value={localUrl} oninput={(e) => onUrlChange((e.currentTarget as HTMLInputElement).value)} placeholder="http://127.0.0.1:7401" />
		</div>
		<div id="connection-sync" class="form-control gap-2 scroll-mt-28">
			<label class="flex items-center gap-2 text-xs font-bold uppercase opacity-60">
				<Timer size={13} />{m.settings_sync_interval()} (ms)
			</label>
			<input class="input bg-base-100" type="number" min={SYNC_INTERVAL_MIN_MS} max={SYNC_INTERVAL_MAX_MS} step="500" value={localSyncIntervalMs} oninput={(e) => onIntervalChange(Number((e.currentTarget as HTMLInputElement).value))} />
		</div>
	</div>

	<div id="connection-health" class="flex flex-wrap items-center gap-3 pt-1 scroll-mt-28">
		<button class="btn btn-sm btn-outline gap-2" onclick={onTestConnection} disabled={isTesting}>
			<Zap size={14} class={isTesting ? 'animate-pulse' : ''} />
			{isTesting ? 'Testing…' : 'Test connection'}
		</button>
		<button class="btn btn-sm btn-ghost gap-2" onclick={onApplyLocalDefaults}>
			<Server size={14} />
			Use local defaults
		</button>
		{#if testResult === 'ok'}
			<span class="flex items-center gap-1 text-xs text-success"><CheckCircle size={13} /> Reachable</span>
		{:else if testResult === 'fail'}
			<span class="flex items-center gap-1 text-xs text-error"><WifiOff size={13} /> Unreachable</span>
		{/if}
	</div>
</section>
