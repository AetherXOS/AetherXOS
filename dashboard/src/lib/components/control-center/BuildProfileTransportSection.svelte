<script lang="ts">
	import type { ConfigProfileExport, ConfigOverrideTemplate } from '$lib/types';
	import type { OverrideTemplateMode } from './types';

	interface Props {
		exportProfileName: string;
		importProfileText: string;
		exportedProfile: ConfigProfileExport | null;
		overrideTemplate: ConfigOverrideTemplate | null;
		overrideTemplateMode: OverrideTemplateMode;
		busy: boolean;
		onExportProfileNameChange: (value: string) => void;
		onImportProfileTextChange: (value: string) => void;
		onExportProfile: () => void;
		onImportProfile: () => void;
		onOverrideTemplateModeChange: (value: OverrideTemplateMode) => void;
		onRefreshTemplate: () => void;
	}

	let {
		exportProfileName,
		importProfileText,
		exportedProfile,
		overrideTemplate,
		overrideTemplateMode,
		busy,
		onExportProfileNameChange,
		onImportProfileTextChange,
		onExportProfile,
		onImportProfile,
		onOverrideTemplateModeChange,
		onRefreshTemplate
	}: Props = $props();

	const exportedProfileText = $derived.by(() => (exportedProfile ? JSON.stringify(exportedProfile, null, 2) : ''));
	const overrideTemplateText = $derived.by(() => (overrideTemplate ? JSON.stringify(overrideTemplate, null, 2) : ''));
</script>

<div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
	<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
		<div class="text-sm font-black uppercase tracking-wider opacity-70">Config profile transport</div>
		<div class="mt-4 flex flex-wrap items-center gap-2">
			<input class="input input-sm bg-base-100 min-w-48" value={exportProfileName} placeholder="profile name" oninput={(event) => onExportProfileNameChange((event.currentTarget as HTMLInputElement).value)} />
			<button class="btn btn-outline btn-sm" onclick={onExportProfile} disabled={busy}>Export profile</button>
			<button class="btn btn-primary btn-sm" onclick={onImportProfile} disabled={busy}>Import JSON</button>
		</div>
		<div class="mt-3 text-xs opacity-55">Use this for repeatable build and config baselines across machines.</div>
		<textarea class="textarea mt-4 h-48 w-full bg-base-100 font-mono text-xs" readonly value={exportedProfileText}></textarea>
		<textarea class="textarea mt-4 h-48 w-full bg-base-100 font-mono text-xs" placeholder="Paste exported profile JSON here" value={importProfileText} oninput={(event) => onImportProfileTextChange((event.currentTarget as HTMLTextAreaElement).value)}></textarea>
	</div>

	<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
		<div class="flex items-center justify-between gap-3">
			<div>
				<div class="text-sm font-black uppercase tracking-wider opacity-70">Override template</div>
				<div class="text-xs opacity-55">export metadata for field-level override authoring</div>
			</div>
			<div class="flex items-center gap-2">
				<select class="select select-sm bg-base-100" value={overrideTemplateMode} onchange={(event) => onOverrideTemplateModeChange((event.currentTarget as HTMLSelectElement).value as OverrideTemplateMode)}>
					<option value="minimal">minimal</option>
					<option value="full">full</option>
				</select>
				<button class="btn btn-outline btn-sm" onclick={onRefreshTemplate} disabled={busy}>Refresh template</button>
			</div>
		</div>
		<textarea class="textarea mt-4 h-104 w-full bg-base-100 font-mono text-xs" readonly value={overrideTemplateText}></textarea>
	</div>
</div>
