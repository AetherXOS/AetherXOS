<script lang="ts">
	import type { ComplianceReport, CrashSummary } from '$lib/types';

	interface Props {
		complianceReport: ComplianceReport | null;
		crashSummary: CrashSummary | null;
		busy: boolean;
		onRefreshInsights: () => void;
	}

	let { complianceReport, crashSummary, busy, onRefreshInsights }: Props = $props();

	const isoRows = $derived.by(() =>
		(complianceReport?.artifactRows ?? []).filter((row) => row.path.toLowerCase().includes('.iso'))
	);
</script>

<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
	<div class="flex items-center justify-between gap-3">
		<div>
			<div class="text-sm font-black uppercase tracking-wider opacity-70">Artifact and ISO explorer</div>
			<div class="text-xs opacity-55">manifest integrity, ISO outputs, and crash artifact status</div>
		</div>
		<button class="btn btn-outline btn-sm" onclick={onRefreshInsights} disabled={busy}>Refresh</button>
	</div>

	<div class="mt-4 grid grid-cols-1 gap-3 xl:grid-cols-3">
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3 text-sm">
			<div class="opacity-55">Compliance checks</div>
			<div class="mt-1 text-2xl font-black">{complianceReport?.passCount ?? 0}/{complianceReport?.totalChecks ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3 text-sm">
			<div class="opacity-55">Artifact rows</div>
			<div class="mt-1 text-2xl font-black">{complianceReport?.artifactRows.length ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3 text-sm">
			<div class="opacity-55">ISO candidates</div>
			<div class="mt-1 text-2xl font-black">{isoRows.length}</div>
		</div>
	</div>

	<div class="mt-4 grid grid-cols-1 gap-4 xl:grid-cols-2">
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-4">
			<div class="mb-2 text-xs font-black uppercase tracking-wider opacity-60">ISO paths from artifact manifest</div>
			{#if isoRows.length === 0}
				<div class="text-sm opacity-60">No ISO artifacts currently indexed.</div>
			{:else}
				<div class="max-h-56 space-y-2 overflow-auto text-xs">
					{#each isoRows as row (row.path)}
						<div class="rounded-lg border border-white/10 p-2">
							<div class="font-semibold">{row.path}</div>
							<div class="opacity-55">exists: {String(row.exists)} | checksum: {String(row.checksumMatch ?? false)}</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-4">
			<div class="mb-2 text-xs font-black uppercase tracking-wider opacity-60">Crash and diagnostics summaries</div>
			<div class="max-h-56 space-y-2 overflow-auto text-xs">
				{#each crashSummary?.entries ?? [] as entry (entry.id)}
					<div class="rounded-lg border border-white/10 p-2">
						<div class="font-semibold">{entry.id}</div>
						<div class="opacity-65">{entry.path}</div>
						<div class="opacity-55">exists: {String(entry.exists)} | ok: {String(entry.ok)}</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
</div>
