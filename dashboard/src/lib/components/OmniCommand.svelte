<script lang="ts">
	import { Search, Zap, Command, LayoutGrid, Terminal, Cpu, RefreshCw, ShieldAlert } from 'lucide-svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { AgentRepo, LauncherRepo } from '$lib/api';
	import { m } from '$lib/paraglide/messages';
	import { orchestrator } from '$lib/services/orchestrator';

	/**
	 * Global command palette for rapid control-plane navigation.
	 * Commands are $derived so they re-evaluate when the locale changes.
	 */
	let isOpen = $state(false);
	let query = $state('');
	let searchInput = $state<HTMLInputElement | null>(null);

	type NavRoute = '/executive' | '/operations' | '/control-center';

	interface PaletteCommand {
		label: string;
		icon: typeof LayoutGrid;
		slug?: NavRoute;
		action?: () => void;
	}

	const commands = $derived<PaletteCommand[]>([
		{ label: m.cmd_goto_executive(), slug: '/executive', icon: LayoutGrid },
		{ label: m.cmd_goto_operations(), slug: '/operations', icon: Cpu },
		{ label: m.cmd_goto_hub(), slug: '/control-center', icon: Terminal },
		{
			label: 'Recover connectivity pack',
			action: async () => {
				await LauncherRepo.restartAgent().catch(() => undefined);
				await orchestrator.sync(true);
			},
			icon: RefreshCw
		},
		{
			label: 'Focus critical incidents',
			action: () => void goto(resolve('/operations?severity=critical&status=open')),
			icon: ShieldAlert
		},
		{
			label: m.cmd_run_syscall_audit(),
			action: () => void AgentRepo.executeBlueprint('SYSCALL'),
			icon: Zap
		}
	]);

	const filtered = $derived(
		commands.filter((c) => c.label.toLowerCase().includes(query.toLowerCase()))
	);

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
			e.preventDefault();
			isOpen = !isOpen;
		}
		if (e.key === 'Escape') isOpen = false;
	}

	async function execute(c: PaletteCommand) {
		if (c.slug) void goto(resolve(c.slug));
		await c.action?.();
		isOpen = false;
		query = '';
	}

	$effect(() => {
		if (isOpen && searchInput) {
			searchInput.focus();
		}
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
	<div
		class="animate-in fade-in fixed inset-0 z-100 flex items-start justify-center bg-black/60 px-6 pt-[15vh] backdrop-blur-md duration-300"
	>
		<div
			class="bg-base-200 scale-in-center w-full max-w-2xl overflow-hidden rounded-2xl border border-white/10 shadow-[0_0_100px_rgba(0,0,0,0.5)]"
		>
			<div class="flex items-center gap-6 border-b border-white/5 p-8">
				<Search size={24} class="text-primary opacity-40" />
				<input
					type="text"
					bind:value={query}
					bind:this={searchInput}
					placeholder={m.cmd_placeholder()}
					class="w-full border-none bg-transparent text-xl font-black tracking-widest uppercase outline-none placeholder:opacity-10"
				/>
				<div
					class="badge badge-outline border-white/10 px-3 py-2 text-[9px] font-black tracking-widest uppercase opacity-20"
				>
					{m.cmd_esc_close()}
				</div>
			</div>

			<div class="max-h-100 overflow-y-auto p-4">
				{#each filtered as cmd (cmd.label)}
					{@const Icon = cmd.icon}
					<button
						class="hover:bg-primary hover:text-primary-content group flex w-full items-center gap-6 rounded-3xl p-6 text-left transition-all"
						onclick={async () => await execute(cmd)}
					>
						<div class="rounded-2xl bg-white/5 p-3 group-hover:bg-white/10">
							<Icon size={20} />
						</div>
						<span class="flex-1 text-xs font-black tracking-widest uppercase italic"
							>{cmd.label}</span
						>
						<div class="opacity-0 transition-opacity group-hover:opacity-100">
							<Command size={14} />
						</div>
					</button>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.scale-in-center {
		animation: scale-in-center 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94) both;
	}
	@keyframes scale-in-center {
		0% {
			transform: scale(0.95);
			opacity: 0;
		}
		100% {
			transform: scale(1);
			opacity: 1;
		}
	}
</style>
