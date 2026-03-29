<script lang="ts">
	import { CheckCircle, Save, WifiOff, Zap } from 'lucide-svelte';

	interface Props {
		localUrl: string;
		localToken: string;
		localLauncherToken: string;
		isConnected: boolean;
		isTesting: boolean;
		isSaving: boolean;
		onTestConnection: () => void;
		onSave: () => void;
	}

	let {
		localUrl,
		localToken,
		localLauncherToken,
		isConnected,
		isTesting,
		isSaving,
		onTestConnection,
		onSave
	}: Props = $props();
</script>

<section id="verify" class="card rounded-2xl border border-white/5 bg-base-200/80 p-5 sm:p-6 space-y-5 scroll-mt-28">
	<div class="flex items-center gap-3 border-b border-white/5 pb-4">
		<div class="bg-primary/15 flex h-9 w-9 items-center justify-center rounded-xl">
			<CheckCircle size={18} class="text-primary" />
		</div>
		<div>
			<div class="text-sm font-black uppercase tracking-wider">Verification</div>
			<div class="text-xs opacity-50">Final checks before daily operations</div>
		</div>
	</div>

	<div class="grid grid-cols-1 gap-3 md:grid-cols-3 text-xs">
		<div class="rounded-xl border border-white/10 bg-base-100/45 p-3">
			<div class="font-black uppercase tracking-wide opacity-60">1. Endpoint</div>
			<div class="mt-1 opacity-70">{localUrl || 'Not set'}</div>
		</div>
		<div class="rounded-xl border border-white/10 bg-base-100/45 p-3">
			<div class="font-black uppercase tracking-wide opacity-60">2. Auth</div>
			<div class="mt-1 opacity-70">{localToken || localLauncherToken ? 'Token configured' : 'No token configured'}</div>
		</div>
		<div class="rounded-xl border border-white/10 bg-base-100/45 p-3">
			<div class="font-black uppercase tracking-wide opacity-60">3. Live Status</div>
			<div class="mt-1 opacity-70">{isConnected ? 'Connected' : 'Offline'}</div>
		</div>
	</div>

	<div class="flex flex-wrap gap-3">
		<button class="btn btn-sm btn-outline gap-2" onclick={onTestConnection} disabled={isTesting}>
			<Zap size={14} class={isTesting ? 'animate-pulse' : ''} />
			Run connectivity check
		</button>
		<button class="btn btn-sm btn-primary gap-2" onclick={onSave} disabled={isSaving}>
			<Save size={14} />
			Save and sync now
		</button>
	</div>
</section>

{#if !isConnected}
	<div class="flex items-center gap-2 text-xs opacity-50 pl-1">
		<WifiOff size={12} />
		Not connected to agent.
	</div>
{/if}
