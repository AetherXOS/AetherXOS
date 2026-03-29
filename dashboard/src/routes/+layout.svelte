<script lang="ts">
	import '../app.css';
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import ConnectionBanner from '$lib/components/ConnectionBanner.svelte';
	import GuidedOnboardingTour from '$lib/components/GuidedOnboardingTour.svelte';
	import NowNextBlockerBar from '$lib/components/NowNextBlockerBar.svelte';
	import OmniCommand from '$lib/components/OmniCommand.svelte';
	import { m } from '$lib/paraglide/messages';
	import { setLocale } from '$lib/paraglide/runtime';
	import { orchestrator } from '$lib/services/orchestrator';
	import { appState } from '$lib/state.svelte';
	import {
		Activity,
		Cpu,
		Globe,
		LayoutDashboard,
		SearchCode,
		Settings,
		Terminal
	} from 'lucide-svelte';
	import { onMount } from 'svelte';
	import { markRouteVisited } from '$lib/services/workflow-progress';

	let { children } = $props();

	type NavHref = '/executive' | '/operations' | '/control-center' | '/deep-debug' | '/settings';

	const navigation = $derived.by(
		(): Array<{ href: NavHref; label: string; icon: typeof LayoutDashboard }> => [
			{ href: '/executive', label: m.nav_executive(), icon: LayoutDashboard },
			{ href: '/operations', label: m.nav_operations(), icon: Cpu },
			{ href: '/control-center', label: m.nav_control_center(), icon: Terminal },
			{ href: '/deep-debug', label: m.nav_deep_debug(), icon: SearchCode },
			{ href: '/settings', label: m.nav_settings(), icon: Settings }
		]
	);

	const syncLabel = $derived(appState.isConnected ? m.layout_synced() : m.layout_dropped());
	const liveLabel = $derived(
		appState.liveMode === 'streaming' ? m.layout_live() : m.layout_polling()
	);

	$effect(() => {
		if (typeof window !== 'undefined') {
			void setLocale(appState.lang, { reload: false });
			markRouteVisited($page.url.pathname);
		}
	});

	onMount(() => {
		void setLocale(appState.lang, { reload: false });
		orchestrator.initialize();
		return () => orchestrator.dispose();
	});
</script>

<!-- Global Command Palette -->
<OmniCommand />
<GuidedOnboardingTour />

<div
	class="drawer lg:drawer-open bg-base-100 text-base-content selection:bg-primary/30 min-h-screen antialiased"
>
	<input id="main-drawer" type="checkbox" class="drawer-toggle" />

	<div class="drawer-content bg-base-300/30 flex flex-col overflow-x-hidden">
		<!-- Navbar -->
		<header
			class="navbar bg-base-100/60 sticky top-0 z-40 min-h-20 border-b border-white/5 px-4 backdrop-blur-3xl sm:px-6 lg:px-12"
		>
			<div class="flex flex-1 items-center gap-4 sm:gap-6">
				<label for="main-drawer" class="btn btn-ghost rounded-2xl lg:hidden"
					><Activity size={24} /></label
				>
				<div class="flex flex-col">
					<div class="text-primary text-[10px] font-black tracking-[0.5em] uppercase italic">
						{m.layout_runtime_architecture()}
					</div>
					<div class="mt-1 text-[9px] font-bold tracking-[0.2em] uppercase opacity-20">
						{m.layout_control_plane()}
					</div>
				</div>
			</div>

			<div class="flex flex-none items-center gap-4 sm:gap-6">
				<div
					class="hidden items-center gap-4 rounded-full border border-white/5 bg-white/5 px-6 py-3 shadow-inner xl:flex"
				>
					<div
						class="h-2 w-2 rounded-full {appState.isConnected
							? 'bg-success shadow-[0_0_10px_oklch(var(--s))]'
							: 'bg-error animate-pulse'}"
					></div>
					<span class="text-[10px] font-black tracking-[0.3em] uppercase opacity-40"
						>{m.layout_cluster_link()}_{syncLabel}_{liveLabel}</span
					>
				</div>

				{#if appState.apiCircuitOpen}
					<div
						class="bg-error/20 border-error/30 hidden items-center gap-3 rounded-full border px-4 py-2 xl:flex"
					>
						<span class="text-error text-[9px] font-black tracking-[0.2em] uppercase"
							>{m.layout_api_circuit_open()}</span
						>
					</div>
				{/if}

				<button
					class="kbd kbd-sm hidden border-white/5 text-[9px] font-black opacity-20 transition-opacity hover:opacity-100 md:flex"
					>{m.layout_command_shortcut()}</button
				>

				<div class="dropdown dropdown-end">
					<button class="btn btn-ghost btn-circle avatar online">
						<div
							class="ring-primary/20 ring-offset-base-100 bg-base-200 w-12 rounded-3xl ring-2 ring-offset-4"
						>
							<img src="https://api.dicebear.com/7.x/bottts-neutral/svg?seed=Runtime" alt="S" />
						</div>
					</button>
				</div>
			</div>
		</header>

		<main class="mx-auto w-full max-w-540 flex-1 overflow-x-hidden p-4 sm:p-6 lg:p-12">
			<div class="space-y-6">
				<NowNextBlockerBar />
				<ConnectionBanner />
				<svelte:boundary>
					{@render children()}
					{#snippet failed(error, reset)}
						<div class="rounded-2xl border border-error/25 bg-error/10 p-5 text-sm">
							<div class="font-black uppercase tracking-wide text-error">UI Error Boundary</div>
							<div class="mt-1 opacity-70">{String(error)}</div>
							<button class="btn btn-sm btn-outline mt-3" onclick={reset}>Retry render</button>
						</div>
					{/snippet}
				</svelte:boundary>
			</div>
		</main>
	</div>

	<!-- Industrial Sidebar -->
	<aside class="drawer-side z-50">
		<label for="main-drawer" class="drawer-overlay"></label>
		<div class="menu bg-base-100 flex min-h-screen w-80 flex-col gap-12 border-r border-white/5 p-8 lg:w-96 lg:p-10">
			<!-- Branding -->
			<a href={resolve('/')} class="group flex items-center gap-6 px-4 transition-all">
				<div
					class="bg-primary text-primary-content shadow-3xl shadow-primary/40 flex h-16 w-16 items-center justify-center rounded-4xl transition-all group-hover:scale-110 group-hover:rotate-12"
				>
					<Globe size={32} strokeWidth={3} />
				</div>
				<div class="flex flex-col">
					<span class="text-3xl leading-none font-black tracking-tighter uppercase italic"
						>{m.brand_name_primary()}<span class="text-primary not-italic underline decoration-white/10"
							>{m.brand_name_suffix()}</span
						></span
					>
					<span class="mt-2 text-[10px] font-black tracking-[0.4em] uppercase opacity-20"
						>{m.layout_aether_pulse()}</span
					>
				</div>
			</a>

			<!-- Navigation -->
			<nav class="flex flex-1 flex-col gap-4">
				{#each navigation as item (item.href)}
					{@const Icon = item.icon}
					{@const active = $page.url.pathname.startsWith(item.href)}
					<a
						href={resolve(item.href)}
						class="group relative flex items-center gap-6 overflow-hidden rounded-2xl px-8 py-6 text-[11px] font-black tracking-[0.2em] uppercase italic transition-all
            {active
							? 'bg-primary text-primary-content shadow-3xl shadow-primary/30'
							: 'opacity-30 hover:bg-white/5 hover:opacity-100'}"
					>
						{#if active}
							<div class="absolute inset-x-0 bottom-0 h-1 bg-white opacity-20"></div>
						{/if}
						<div
							class={active
								? 'text-primary-content'
								: 'group-hover:text-primary opacity-40 transition-all'}
						>
							<Icon size={22} strokeWidth={2.5} />
						</div>
						<span>{item.label}</span>
					</a>
				{/each}
			</nav>

			<!-- Telemetry Health Summary -->
			<div
				class="mt-auto rounded-[3.5rem] border border-white/5 bg-linear-to-br from-white/5 to-transparent p-1"
			>
				<div class="bg-base-200/50 space-y-6 rounded-[3.3rem] p-8">
					<div
						class="flex items-center justify-between text-[9px] font-black tracking-widest uppercase italic"
					>
						<span class="opacity-20">{m.layout_kernel_load()}</span>
						<span class="{appState.isConnected ? 'text-primary opacity-60' : 'text-warning opacity-60'}">{appState.isConnected ? appState.metrics.cpu.toFixed(1) + '%' : 'offline'}</span>
					</div>
					{#snippet cpuViz()}
						{@const filled = Math.round(appState.metrics.cpu / 10)}
						<div class="grid grid-cols-10 gap-1.5">
							{#each Array.from({ length: 10 }, (_, idx) => idx) as k (k)}
								<div class="h-8 rounded-full transition-all duration-700 {k < filled ? (filled >= 9 ? 'bg-error opacity-90' : filled >= 7 ? 'bg-warning opacity-80' : 'bg-primary opacity-70') : 'bg-white/8'}"></div>
							{/each}
						</div>
					{/snippet}
					{@render cpuViz()}
				</div>
			</div>
		</div>
	</aside>
</div>
