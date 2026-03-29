<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { Download, Upload } from 'lucide-svelte';

	interface Props {
		profileMessage: string;
		profileFileInput: HTMLInputElement | null;
		onExportProfile: () => void;
		onImportProfile: (event: Event) => void;
	}

	let { profileMessage, profileFileInput, onExportProfile, onImportProfile }: Props = $props();
</script>

<section id="profile" class="card rounded-2xl border border-white/5 bg-base-200/80 p-5 sm:p-6 space-y-5 scroll-mt-28">
	<div class="flex items-center gap-3 border-b border-white/5 pb-4">
		<div class="bg-success/15 flex h-9 w-9 items-center justify-center rounded-xl">
			<Download size={18} class="text-success" />
		</div>
		<div>
			<div class="text-sm font-black uppercase tracking-wider">Profile</div>
			<div class="text-xs opacity-50">Export or import an entire settings profile as JSON</div>
		</div>
	</div>

	<div id="profile-import" class="flex flex-wrap gap-3 scroll-mt-28">
		<button class="btn btn-sm btn-outline gap-2" onclick={() => profileFileInput?.click()}>
			<Upload size={14} />{m.settings_import()}
		</button>
	</div>
	<div id="profile-export" class="flex flex-wrap gap-3 scroll-mt-28">
		<button class="btn btn-sm btn-outline gap-2" onclick={onExportProfile}>
			<Download size={14} />{m.settings_export()}
		</button>
	</div>

	{#if profileMessage}
		<div class="alert alert-info py-3 text-sm"><span>{profileMessage}</span></div>
	{/if}
</section>

<!-- Hidden file input rendered by parent, wired via profileFileInput binding -->
