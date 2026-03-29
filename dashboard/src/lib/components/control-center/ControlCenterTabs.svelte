<script lang="ts">
	import type { ControlTab } from './types';

	interface TabItem {
		id: ControlTab;
		label: string;
	}

	interface Props {
		activeTab: ControlTab;
		tabs: TabItem[];
		onSelect: (tab: ControlTab) => void;
	}

	let { activeTab, tabs, onSelect }: Props = $props();
</script>

<div class="join w-full rounded-3xl border border-white/5 bg-base-200/80 p-2">
	{#each tabs as tab (tab.id)}
		<button
			class="join-item btn flex-1 rounded-2xl {activeTab === tab.id ? 'btn-primary' : 'btn-ghost'}"
			onclick={() => onSelect(tab.id)}
		>
			{tab.label}
		</button>
	{/each}
</div>
