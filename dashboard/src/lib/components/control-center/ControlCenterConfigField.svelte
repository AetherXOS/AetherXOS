<script lang="ts">
	import type { ConfigFieldSpec } from '$lib/types';
	import { Undo2 } from 'lucide-svelte';

	interface Props {
		field: ConfigFieldSpec;
		value: string;
		changed: boolean;
		disabled: boolean;
		onTextChange: (path: string, value: string) => void;
		onBoolChange: (path: string, checked: boolean) => void;
		onRevert: (field: ConfigFieldSpec) => void;
	}

	let { field, value, changed, disabled, onTextChange, onBoolChange, onRevert }: Props = $props();
</script>

<div class="rounded-xl border border-white/10 bg-base-100/30 p-3">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div>
			<div class="text-sm font-semibold">{field.label}</div>
			<div class="text-[11px] opacity-55">{field.path}</div>
		</div>
		<div class="flex items-center gap-2">
			<div class="badge badge-ghost text-[10px]">{field.type}</div>
			{#if changed}
				<div class="badge badge-warning text-[10px]">changed</div>
				<button class="btn btn-ghost btn-xs" onclick={() => onRevert(field)}>
					<Undo2 size={12} />
					revert
				</button>
			{/if}
		</div>
	</div>

	<div class="mt-3">
		{#if field.type === 'bool'}
			<label class="label cursor-pointer justify-start gap-3 py-0">
				<input
					type="checkbox"
					class="checkbox"
					checked={value === 'true'}
					onchange={(event) => onBoolChange(field.path, (event.currentTarget as HTMLInputElement).checked)}
					disabled={disabled || field.readonly}
				/>
				<span class="label-text">Enabled</span>
			</label>
		{:else if field.choices && field.choices.length > 0}
			<select
				class="select select-sm bg-base-100 w-full"
				value={value}
				onchange={(event) => onTextChange(field.path, (event.currentTarget as HTMLSelectElement).value)}
				disabled={disabled || field.readonly}
			>
				{#each field.choices as choice (choice)}
					<option value={choice}>{choice}</option>
				{/each}
			</select>
		{:else}
			<input
				class="input input-sm bg-base-100 w-full"
				type={field.type === 'int' || field.type === 'float' ? 'number' : 'text'}
				step={field.type === 'float' ? '0.01' : '1'}
				min={field.min}
				max={field.max}
				value={value}
				oninput={(event) => onTextChange(field.path, (event.currentTarget as HTMLInputElement).value)}
				disabled={disabled || field.readonly}
			/>
		{/if}
	</div>

	{#if field.help}
		<div class="mt-2 text-[11px] opacity-60">{field.help}</div>
	{/if}
</div>
