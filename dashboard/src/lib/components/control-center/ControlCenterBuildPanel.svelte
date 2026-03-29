<script lang="ts">
	import type {
		AgentHost,
		AgentJobDetail,
		AgentJobSummary,
		BuildFeatureRecommendation,
		ComplianceReport,
		ConfigDriftReport,
		ConfigProfileExport,
		ConfigOverrideTemplate,
		CrashSummary,
		JobStreamTimelineEvent
	} from '$lib/types';
	import type { ComposeGoal, DriftApplyMode, OverrideTemplateMode } from './types';
	import BuildArtifactsSection from './BuildArtifactsSection.svelte';
	import BuildDriftSection from './BuildDriftSection.svelte';
	import BuildJobInspectorSection from './BuildJobInspectorSection.svelte';
	import BuildProfileTransportSection from './BuildProfileTransportSection.svelte';
	import BuildRecommendationSection from './BuildRecommendationSection.svelte';

	interface Props {
		composeGoal: ComposeGoal;
		composeMinimal: boolean;
		driftApplyMode: DriftApplyMode;
		overrideTemplateMode: OverrideTemplateMode;
		hosts: AgentHost[];
		selectedJobHostId: string;
		selectedJobId: string;
		selectedJobTail: number;
		jobs: AgentJobSummary[];
		recommendation: BuildFeatureRecommendation | null;
		driftReport: ConfigDriftReport | null;
		exportedProfile: ConfigProfileExport | null;
		overrideTemplate: ConfigOverrideTemplate | null;
		selectedJobDetail: AgentJobDetail | null;
		complianceReport: ComplianceReport | null;
		crashSummary: CrashSummary | null;
		jobStreamEnabled: boolean;
		jobStreamConnected: boolean;
		jobStreamStatus: 'idle' | 'connecting' | 'streaming' | 'fallback' | 'error';
		jobAutoRefreshEnabled: boolean;
		jobAutoRefreshMs: number;
		streamEvents: JobStreamTimelineEvent[];
		streamReconnectCount: number;
		buildBusy: boolean;
		jobDetailBusy: boolean;
		buildMessage: string;
		exportProfileName: string;
		importProfileText: string;
		onComposeGoalChange: (value: ComposeGoal) => void;
		onComposeMinimalChange: (checked: boolean) => void;
		onDriftApplyModeChange: (value: DriftApplyMode) => void;
		onOverrideTemplateModeChange: (value: OverrideTemplateMode) => void;
		onExportProfileNameChange: (value: string) => void;
		onImportProfileTextChange: (value: string) => void;
		onRefreshInsights: () => void;
		onRefreshRecommendation: () => void;
		onApplyCompose: () => void;
		onApplyDriftFix: () => void;
		onExportProfile: () => void;
		onImportProfile: () => void;
		onRefreshTemplate: () => void;
		onRefreshHosts: () => void;
		onSelectedJobHostChange: (value: string) => void;
		onSelectedJobChange: (value: string) => void;
		onSelectedJobTailChange: (value: number) => void;
		onFetchJobDetail: (id: string, tail: number) => void;
		onJobStreamEnabledChange: (checked: boolean) => void;
		onJobAutoRefreshEnabledChange: (checked: boolean) => void;
		onJobAutoRefreshMsChange: (value: number) => void;
		onCancelJob: () => void;
	}

	let {
		composeGoal,
		composeMinimal,
		driftApplyMode,
		overrideTemplateMode,
		hosts,
		selectedJobHostId,
		selectedJobId,
		selectedJobTail,
		jobs,
		recommendation,
		driftReport,
		exportedProfile,
		overrideTemplate,
		selectedJobDetail,
		complianceReport,
		crashSummary,
		jobStreamEnabled,
		jobStreamConnected,
		jobStreamStatus,
		jobAutoRefreshEnabled,
		jobAutoRefreshMs,
		streamEvents,
		streamReconnectCount,
		buildBusy,
		jobDetailBusy,
		buildMessage,
		exportProfileName,
		importProfileText,
		onComposeGoalChange,
		onComposeMinimalChange,
		onDriftApplyModeChange,
		onOverrideTemplateModeChange,
		onExportProfileNameChange,
		onImportProfileTextChange,
		onRefreshInsights,
		onRefreshRecommendation,
		onApplyCompose,
		onApplyDriftFix,
		onExportProfile,
		onImportProfile,
		onRefreshTemplate,
		onRefreshHosts,
		onSelectedJobHostChange,
		onSelectedJobChange,
		onSelectedJobTailChange,
		onFetchJobDetail,
		onJobStreamEnabledChange,
		onJobAutoRefreshEnabledChange,
		onJobAutoRefreshMsChange,
		onCancelJob
	}: Props = $props();

	const isoCount = $derived.by(
		() => (complianceReport?.artifactRows ?? []).filter((row) => row.path.toLowerCase().includes('.iso')).length
	);
</script>

<section class="space-y-4">
	<div class="grid grid-cols-1 gap-3 xl:grid-cols-4">
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Compose</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{recommendation?.selectedCount ?? 0}</div>
			<div class="text-sm opacity-60">selected cargo features</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Drift</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{driftReport?.goals.reduce((sum, row) => sum + row.missingCount, 0) ?? 0}</div>
			<div class="text-sm opacity-60">missing features</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">ISO</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{isoCount}</div>
			<div class="text-sm opacity-60">artifact manifest entries</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Logs</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{selectedJobDetail?.lineCount ?? 0}</div>
			<div class="text-sm opacity-60">selected job output lines</div>
		</div>
	</div>

	{#if buildMessage}
		<div class="alert alert-info py-2 text-sm"><span>{buildMessage}</span></div>
	{/if}

	<div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
		<BuildRecommendationSection
			composeGoal={composeGoal}
			composeMinimal={composeMinimal}
			recommendation={recommendation}
			busy={buildBusy}
			onComposeGoalChange={onComposeGoalChange}
			onComposeMinimalChange={onComposeMinimalChange}
			onRefreshRecommendation={onRefreshRecommendation}
			onApplyCompose={onApplyCompose}
		/>
		<BuildDriftSection
			driftApplyMode={driftApplyMode}
			driftReport={driftReport}
			busy={buildBusy}
			onDriftApplyModeChange={onDriftApplyModeChange}
			onRefreshInsights={onRefreshInsights}
			onApplyDriftFix={onApplyDriftFix}
		/>
	</div>

	<BuildProfileTransportSection
		exportProfileName={exportProfileName}
		importProfileText={importProfileText}
		exportedProfile={exportedProfile}
		overrideTemplate={overrideTemplate}
		overrideTemplateMode={overrideTemplateMode}
		busy={buildBusy}
		onExportProfileNameChange={onExportProfileNameChange}
		onImportProfileTextChange={onImportProfileTextChange}
		onExportProfile={onExportProfile}
		onImportProfile={onImportProfile}
		onOverrideTemplateModeChange={onOverrideTemplateModeChange}
		onRefreshTemplate={onRefreshTemplate}
	/>

	<BuildArtifactsSection
		complianceReport={complianceReport}
		crashSummary={crashSummary}
		busy={buildBusy}
		onRefreshInsights={onRefreshInsights}
	/>

	<BuildJobInspectorSection
		hosts={hosts}
		selectedJobHostId={selectedJobHostId}
		jobs={jobs}
		selectedJobId={selectedJobId}
		selectedJobTail={selectedJobTail}
		selectedJobDetail={selectedJobDetail}
		jobStreamEnabled={jobStreamEnabled}
		jobStreamConnected={jobStreamConnected}
		jobStreamStatus={jobStreamStatus}
		jobAutoRefreshEnabled={jobAutoRefreshEnabled}
		jobAutoRefreshMs={jobAutoRefreshMs}
		streamEvents={streamEvents}
		streamReconnectCount={streamReconnectCount}
		busy={buildBusy || jobDetailBusy}
		onRefreshHosts={onRefreshHosts}
		onSelectedJobHostChange={onSelectedJobHostChange}
		onSelectedJobChange={onSelectedJobChange}
		onSelectedJobTailChange={onSelectedJobTailChange}
		onFetchJobDetail={onFetchJobDetail}
		onJobStreamEnabledChange={onJobStreamEnabledChange}
		onJobAutoRefreshEnabledChange={onJobAutoRefreshEnabledChange}
		onJobAutoRefreshMsChange={onJobAutoRefreshMsChange}
		onCancelJob={onCancelJob}
	/>
</section>
