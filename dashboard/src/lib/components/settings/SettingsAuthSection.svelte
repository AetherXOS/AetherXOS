<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { Eye, EyeOff, Key } from 'lucide-svelte';

	interface Props {
		localToken: string;
		localLauncherToken: string;
		onTokenChange: (v: string) => void;
		onLauncherTokenChange: (v: string) => void;
	}

	let { localToken, localLauncherToken, onTokenChange, onLauncherTokenChange }: Props = $props();

	let showAgentToken = $state(false);
	let showLauncherToken = $state(false);
</script>

<section id="authentication" class="card rounded-2xl border border-white/5 bg-base-200/80 p-5 sm:p-6 space-y-5 scroll-mt-28">
	<div class="flex items-center gap-3 border-b border-white/5 pb-4">
		<div class="bg-warning/15 flex h-9 w-9 items-center justify-center rounded-xl">
			<Key size={18} class="text-warning" />
		</div>
		<div>
			<div class="text-sm font-black uppercase tracking-wider">Authentication</div>
			<div class="text-xs opacity-50">Tokens for agent and launcher access</div>
		</div>
	</div>

	<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
		<div id="auth-agent" class="form-control gap-2 scroll-mt-28">
			<label class="flex items-center gap-2 text-xs font-bold uppercase opacity-60">
				<Key size={13} />{m.settings_auth_key()}
			</label>
			<div class="flex gap-2">
				<input class="input bg-base-100 flex-1" type={showAgentToken ? 'text' : 'password'} value={localToken} oninput={(e) => onTokenChange((e.currentTarget as HTMLInputElement).value)} placeholder="leave blank if not required" />
				<button class="btn btn-square btn-sm btn-ghost" onclick={() => (showAgentToken = !showAgentToken)} title="Toggle visibility">
					{#if showAgentToken}<EyeOff size={15} />{:else}<Eye size={15} />{/if}
				</button>
			</div>
		</div>
		<div id="auth-launcher" class="form-control gap-2 scroll-mt-28">
			<label class="flex items-center gap-2 text-xs font-bold uppercase opacity-60">
				<Key size={13} />{m.settings_launcher_token()}
			</label>
			<div class="flex gap-2">
				<input class="input bg-base-100 flex-1" type={showLauncherToken ? 'text' : 'password'} value={localLauncherToken} oninput={(e) => onLauncherTokenChange((e.currentTarget as HTMLInputElement).value)} placeholder="leave blank if not required" />
				<button class="btn btn-square btn-sm btn-ghost" onclick={() => (showLauncherToken = !showLauncherToken)} title="Toggle visibility">
					{#if showLauncherToken}<EyeOff size={15} />{:else}<Eye size={15} />{/if}
				</button>
			</div>
		</div>
	</div>
</section>
