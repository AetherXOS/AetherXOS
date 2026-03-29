<script lang="ts">
	import type { AgentConfigPayload, ConfigFieldSpec } from '$lib/types';
	import type { AutoPresetMode, ComposeGoal } from './types';
	import { Wrench } from 'lucide-svelte';
	import ControlCenterConfigField from './ControlCenterConfigField.svelte';

	type ConfigGroup = [string, ConfigFieldSpec[]];

	interface Props {
		configSnapshot: AgentConfigPayload | null;
		configGroups: ConfigGroup[];
		configSearch: string;
		showChangedOnly: boolean;
		autoPresetMode: AutoPresetMode;
		composeGoal: ComposeGoal;
		composeMinimal: boolean;
		configBusy: boolean;
		configMessage: string;
		pendingConfigCount: number;
		getDraftValue: (path: string) => string;
		isFieldChanged: (field: ConfigFieldSpec) => boolean;
		onSearchChange: (value: string) => void;
		onShowChangedOnlyChange: (checked: boolean) => void;
		onAutoPresetModeChange: (value: AutoPresetMode) => void;
		onComposeGoalChange: (value: ComposeGoal) => void;
		onComposeMinimalChange: (checked: boolean) => void;
		onRefreshConfig: () => void;
		onApplyConfigUpdates: () => void;
		onApplyAutoPreset: () => void;
		onApplyComposeProfile: () => void;
		onTextDraftChange: (path: string, value: string) => void;
		onBoolDraftChange: (path: string, checked: boolean) => void;
		onRevertField: (field: ConfigFieldSpec) => void;
	}

	let {
		configSnapshot,
		configGroups,
		configSearch,
		showChangedOnly,
		autoPresetMode,
		composeGoal,
		composeMinimal,
		configBusy,
		configMessage,
		pendingConfigCount,
		getDraftValue,
		isFieldChanged,
		onSearchChange,
		onShowChangedOnlyChange,
		onAutoPresetModeChange,
		onComposeGoalChange,
		onComposeMinimalChange,
		onRefreshConfig,
		onApplyConfigUpdates,
		onApplyAutoPreset,
		onApplyComposeProfile,
		onTextDraftChange,
		onBoolDraftChange,
		onRevertField
	}: Props = $props();
</script>

<section class="space-y-4">
	<div class="grid grid-cols-1 gap-3 xl:grid-cols-3">
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Snapshot</div>
			<div class="mt-3 text-sm font-semibold">{configSnapshot?.configPath ?? 'unavailable'}</div>
			<div class="mt-1 text-xs opacity-55">generated {configSnapshot?.generatedUtc ?? '-'}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Field inventory</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{configSnapshot?.fields.length ?? 0}</div>
			<div class="text-sm opacity-60">runtime-configurable settings discovered</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Pending</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{pendingConfigCount}</div>
			<div class="text-sm opacity-60">draft changes waiting to be applied</div>
		</div>
	</div>

	<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
		<div class="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
			<div class="flex items-center gap-3 text-sm font-black tracking-wider uppercase opacity-70">
				<Wrench size={16} />
				Runtime Config Center
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<input
					class="input input-sm bg-base-100 min-w-56"
					placeholder="Search path/label/group"
					value={configSearch}
					oninput={(event) => onSearchChange((event.currentTarget as HTMLInputElement).value)}
				/>
				<label class="label cursor-pointer gap-2 py-0">
					<span class="label-text text-xs opacity-70">changed only</span>
					<input
						class="checkbox checkbox-xs"
						type="checkbox"
						checked={showChangedOnly}
						onchange={(event) => onShowChangedOnlyChange((event.currentTarget as HTMLInputElement).checked)}
					/>
				</label>
				<button class="btn btn-outline btn-sm" onclick={onRefreshConfig} disabled={configBusy}>Refresh</button>
				<button class="btn btn-primary btn-sm" onclick={onApplyConfigUpdates} disabled={configBusy}>
					Apply Changed Fields ({pendingConfigCount})
				</button>
			</div>
		</div>

		<div class="mt-4 grid grid-cols-1 gap-3 lg:grid-cols-2">
			<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3">
				<div class="text-xs font-black tracking-wide uppercase opacity-60">Auto preset</div>
				<div class="mt-2 flex flex-wrap items-center gap-2">
					<select
						class="select select-sm bg-base-100"
						value={autoPresetMode}
						onchange={(event) =>
							onAutoPresetModeChange((event.currentTarget as HTMLSelectElement).value as AutoPresetMode)}
					>
						<option value="balanced">balanced</option>
						<option value="fast_dev">fast_dev</option>
						<option value="reliable_ci">reliable_ci</option>
					</select>
					<button class="btn btn-outline btn-sm" onclick={onApplyAutoPreset} disabled={configBusy}>Apply preset</button>
				</div>
			</div>

			<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3">
				<div class="text-xs font-black tracking-wide uppercase opacity-60">Build feature compose</div>
				<div class="mt-2 flex flex-wrap items-center gap-2">
					<select
						class="select select-sm bg-base-100"
						value={composeGoal}
						onchange={(event) =>
							onComposeGoalChange((event.currentTarget as HTMLSelectElement).value as ComposeGoal)}
					>
						<option value="boot_min">boot_min</option>
						<option value="linux_full">linux_full</option>
						<option value="release_hardening">release_hardening</option>
					</select>
					<label class="label cursor-pointer gap-2 py-0">
						<span class="label-text text-xs opacity-70">minimal</span>
						<input
							class="checkbox checkbox-xs"
							type="checkbox"
							checked={composeMinimal}
							onchange={(event) => onComposeMinimalChange((event.currentTarget as HTMLInputElement).checked)}
						/>
					</label>
					<button class="btn btn-outline btn-sm" onclick={onApplyComposeProfile} disabled={configBusy}>Apply compose</button>
				</div>
			</div>
		</div>

		{#if configMessage}
			<div class="alert alert-info mt-4 py-2 text-sm">
				<span>{configMessage}</span>
			</div>
		{/if}
	</div>

	{#if configSnapshot}
		<div class="grid grid-cols-1 gap-4">
			{#each configGroups as [groupName, fields] (groupName)}
				<div class="card rounded-2xl border border-white/8 bg-base-200/70 p-5">
					<div class="mb-4 flex items-center justify-between gap-3">
						<h3 class="text-base font-black tracking-tight">{groupName}</h3>
						<div class="badge badge-outline">{fields.length}</div>
					</div>
					<div class="space-y-3">
						{#each fields as field (field.path)}
							<ControlCenterConfigField
								field={field}
								value={getDraftValue(field.path)}
								changed={isFieldChanged(field)}
								disabled={configBusy}
								onTextChange={onTextDraftChange}
								onBoolChange={onBoolDraftChange}
								onRevert={onRevertField}
							/>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="card rounded-2xl border border-white/8 bg-base-200/70 p-5 text-sm opacity-70">
			Config snapshot is not available yet.
		</div>
	{/if}
</section>
