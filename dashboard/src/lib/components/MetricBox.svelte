<script lang="ts">
	import { TrendingUp, TrendingDown, Minus } from 'lucide-svelte';
	import { m } from '$lib/paraglide/messages';

	interface Props {
		label: string;
		value: string | number;
		unit?: string;
		trend?: number;
		status?: 'success' | 'warning' | 'error' | 'info' | 'primary';
	}

	let { label, value, unit, trend, status = 'primary' }: Props = $props();

	const gaugeWidth = $derived.by(() => {
		const n = Number(value);
		if (!Number.isFinite(n)) return 40;
		return Math.min(100, Math.max(8, Math.round(Math.abs(n))));
	});

	const statusClasses = {
		primary: 'text-primary border-primary/20 bg-primary/10',
		success: 'text-success border-success/20 bg-success/10',
		warning: 'text-warning border-warning/20 bg-warning/10',
		error: 'text-error border-error/20 bg-error/10',
		info: 'text-info border-info/20 bg-info/10'
	};
</script>

<div
	class="group bg-base-200 relative overflow-hidden rounded-2xl border border-white/5 p-8 shadow-xl shadow-black/20 transition-all hover:border-white/10"
>
	<div
		class="pointer-events-none absolute inset-0 bg-linear-to-br from-white/3 to-transparent opacity-0 transition-opacity group-hover:opacity-100"
	></div>

	<div class="relative z-10 space-y-8">
		<header class="flex items-center justify-between">
			<span
				class="text-[10px] font-black tracking-[0.3em] uppercase italic opacity-30 transition-opacity group-hover:opacity-100"
			>
				{label}
			</span>

			{#if trend !== undefined}
				<div
					class="flex items-center gap-2 rounded-2xl border border-white/5 bg-white/5 px-3 py-1.5"
				>
					{#if trend > 0}
						<TrendingUp size={12} class="text-success" />
						<span class="text-success text-[10px] font-black">+{trend}%</span>
					{:else if trend < 0}
						<TrendingDown size={12} class="text-error" />
						<span class="text-error text-[10px] font-black">{trend}%</span>
					{:else}
						<Minus size={12} class="opacity-20" />
						<span class="text-[10px] font-black opacity-30">0%</span>
					{/if}
				</div>
			{/if}
		</header>

		<main class="flex items-baseline gap-3">
			<h2 class="text-6xl leading-none font-black tracking-tighter italic tabular-nums">{value}</h2>
			{#if unit}
				<span
					class="-translate-y-2 text-sm font-bold tracking-[0.2em] uppercase italic underline decoration-white/10 opacity-20"
					>{unit}</span
				>
			{/if}
		</main>

		<footer class="flex items-center justify-between border-t border-white/5 pt-4">
			<div class="mr-6 h-1.5 flex-1 overflow-hidden rounded-full bg-white/5">
				<div
					class="h-full {statusClasses[status].split(' ')[0]} transition-all duration-1500"
					style="width: {gaugeWidth}%"
				></div>
			</div>
			<div
				class="badge badge-outline border-white/5 px-3 py-2 text-[8px] font-black tracking-widest uppercase opacity-20"
			>
				{m.metric_verified_io()}
			</div>
		</footer>
	</div>
</div>
