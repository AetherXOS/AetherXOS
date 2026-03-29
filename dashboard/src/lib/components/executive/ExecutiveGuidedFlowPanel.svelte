<script lang="ts">
	import { resolve } from '$app/paths';

	type GuidanceHref =
		| '/settings#connection'
		| '/settings#authentication'
		| '/settings#verify'
		| '/operations'
		| '/control-center'
		| '/deep-debug';

	interface GuidanceAction {
		id: string;
		label: string;
		href: GuidanceHref;
		icon: any;
	}

	interface Props {
		isConnected: boolean;
		actions: GuidanceAction[];
	}

	let { isConnected, actions }: Props = $props();
</script>

<section class="rounded-3xl border border-white/5 bg-base-200/70 p-6 sm:p-8 space-y-4">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="text-sm font-black tracking-widest uppercase opacity-60">Guided Flow</div>
			<div class="text-xs opacity-50">Use this path to reduce complexity and move step by step.</div>
		</div>
		<div class="badge {isConnected ? 'badge-success' : 'badge-warning'}">{isConnected ? 'ready' : 'setup required'}</div>
	</div>
	<div class="grid grid-cols-1 gap-3 md:grid-cols-3">
		{#each actions as action (action.id)}
			{@const Icon = action.icon}
			<a href={resolve(action.href)} class="rounded-xl border border-white/10 bg-base-100/40 px-4 py-4 text-xs transition-all hover:border-primary/30 hover:bg-base-100/70">
				<div class="flex items-center gap-2 font-black uppercase tracking-wide">
					<Icon size={13} class="text-primary" />
					Step
				</div>
				<div class="mt-1 opacity-70">{action.label}</div>
			</a>
		{/each}
	</div>
</section>
