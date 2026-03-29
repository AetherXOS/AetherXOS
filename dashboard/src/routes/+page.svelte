<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { Globe, Activity } from 'lucide-svelte';
	import { m } from '$lib/paraglide/messages';

	/**
	 * Initialization shell for dashboard boot.
	 */
	onMount(() => {
		setTimeout(() => {
			goto('/executive');
		}, 1500);
	});
</script>

<div
	class="bg-base-100 relative flex h-screen w-full flex-col items-center justify-center overflow-hidden"
>
	<!-- Animated Pulse Background -->
	<div
		class="absolute inset-0 animate-pulse bg-[radial-gradient(circle_at_center,var(--p)_0%,transparent_50%)] opacity-[0.05]"
	></div>

	<div class="relative z-10 flex flex-col items-center gap-12 text-center">
		<!-- High-End Logo -->
		<div
			class="bg-primary text-primary-content shadow-3xl shadow-primary/40 flex h-32 w-32 rotate-12 items-center justify-center rounded-[3rem] transition-all hover:rotate-0"
		>
			<Globe size={64} strokeWidth={3} />
		</div>

		<div class="space-y-4">
			<h1 class="text-6xl leading-none font-black tracking-tighter uppercase italic">
				{m.brand_name_primary()}<span class="text-primary not-italic">{m.brand_name_suffix()}</span>
			</h1>
			<div
				class="flex items-center justify-center gap-4 text-[10px] font-black tracking-[0.5em] uppercase italic opacity-30"
			>
				<Activity size={14} />
				{m.splash_tagline()}
				<Activity size={14} />
			</div>
		</div>

		<div class="relative h-1.5 w-64 overflow-hidden rounded-full bg-white/5">
			<div
				class="bg-primary absolute inset-y-0 left-0 w-1/3 animate-[loading_1.5s_ease-in-out_infinite]"
			></div>
		</div>
	</div>
</div>

<style>
	@keyframes loading {
		0% {
			left: -40%;
			width: 40%;
		}
		100% {
			left: 100%;
			width: 40%;
		}
	}
</style>
