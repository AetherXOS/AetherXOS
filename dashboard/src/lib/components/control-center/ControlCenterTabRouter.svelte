<script lang="ts">
	import ControlCenterBlueprintsPanel from './ControlCenterBlueprintsPanel.svelte';
	import ControlCenterBuildPanel from './ControlCenterBuildPanel.svelte';
	import ControlCenterConfigPanel from './ControlCenterConfigPanel.svelte';
	import ControlCenterLauncherPanel from './ControlCenterLauncherPanel.svelte';
	import ControlCenterOperationsPanel from './ControlCenterOperationsPanel.svelte';
	import ControlCenterPluginsPanel from './ControlCenterPluginsPanel.svelte';
	import type { ControlCenterState } from './control-center.state.svelte';
	import type { ComposeGoal, DriftApplyMode, OverrideTemplateMode } from './types';
	import { appState } from '$lib/state.svelte';

	const { controller }: { controller: ControlCenterState } = $props();
</script>

{#if controller.activeTab === 'operations'}
	<ControlCenterOperationsPanel
		rows={controller.operationRows}
		categories={controller.operationCategories}
		operationSearch={controller.operationSearch}
		operationCategory={controller.operationCategory}
		operationPriority={controller.operationPriority}
		operationMessage={controller.operationMessage}
		operationsBusy={controller.operationsBusy}
		jobsBusy={controller.jobsBusy}
		jobs={controller.jobRows}
		onSearchChange={(value) => (controller.operationSearch = value)}
		onCategoryChange={(value) => (controller.operationCategory = value)}
		onPriorityChange={(value) => (controller.operationPriority = value)}
		onRefreshCatalog={controller.refreshCatalog}
		onRequestOperation={controller.requestOperation}
		onRefreshJobs={controller.refreshJobs}
		onCancelJob={controller.cancelJob}
	/>
{/if}

{#if controller.activeTab === 'build'}
	<ControlCenterBuildPanel
		composeGoal={controller.composeGoal}
		composeMinimal={controller.composeMinimal}
		driftApplyMode={controller.driftApplyMode}
		overrideTemplateMode={controller.overrideTemplateMode}
		hosts={controller.hostRows}
		selectedJobHostId={controller.selectedJobHostId}
		selectedJobId={controller.selectedJobId}
		selectedJobTail={controller.selectedJobTail}
		jobs={controller.jobRows}
		recommendation={controller.composeRecommendation}
		driftReport={controller.driftReport}
		exportedProfile={controller.exportedProfile}
		overrideTemplate={controller.overrideTemplate}
		selectedJobDetail={controller.selectedJobDetail}
		complianceReport={controller.complianceReport}
		crashSummary={controller.crashSummary}
		jobStreamEnabled={controller.jobStreamEnabled}
		jobStreamConnected={controller.jobStreamConnected}
		jobStreamStatus={controller.jobStreamStatus}
		jobAutoRefreshEnabled={controller.jobAutoRefreshEnabled}
		jobAutoRefreshMs={controller.jobAutoRefreshMs}
		streamEvents={controller.streamEvents}
		streamReconnectCount={controller.streamReconnectCount}
		buildBusy={controller.buildBusy}
		jobDetailBusy={controller.jobDetailBusy}
		buildMessage={controller.buildMessage}
		exportProfileName={controller.exportProfileName}
		importProfileText={controller.importProfileText}
		onComposeGoalChange={(value: ComposeGoal) => (controller.composeGoal = value)}
		onComposeMinimalChange={(checked: boolean) => (controller.composeMinimal = checked)}
		onDriftApplyModeChange={(value: DriftApplyMode) => (controller.driftApplyMode = value)}
		onOverrideTemplateModeChange={(value: OverrideTemplateMode) => {
			controller.overrideTemplateMode = value;
			void controller.refreshOverrideTemplate();
		}}
		onExportProfileNameChange={(value: string) => (controller.exportProfileName = value)}
		onImportProfileTextChange={(value: string) => (controller.importProfileText = value)}
		onRefreshInsights={controller.refreshBuildInsights}
		onRefreshRecommendation={controller.refreshComposeRecommendation}
		onApplyCompose={controller.applyComposeProfile}
		onApplyDriftFix={controller.applyDriftFix}
		onExportProfile={controller.exportProfile}
		onImportProfile={controller.importProfile}
		onRefreshTemplate={controller.refreshOverrideTemplate}
		onRefreshHosts={controller.refreshHosts}
		onSelectedJobHostChange={(value: string) => controller.selectJobHost(value)}
		onSelectedJobChange={(value: string) => controller.selectJob(value)}
		onSelectedJobTailChange={(value: number) => controller.setSelectedJobTail(value)}
		onFetchJobDetail={(id: string, tail: number) => void controller.fetchJobDetail(id, tail)}
		onJobStreamEnabledChange={(checked: boolean) => controller.setJobStreamEnabled(checked)}
		onJobAutoRefreshEnabledChange={(checked: boolean) =>
			controller.setJobAutoRefreshEnabled(checked)}
		onJobAutoRefreshMsChange={(value: number) => controller.setJobAutoRefreshMs(value)}
		onCancelJob={controller.cancelSelectedJob}
	/>
{/if}

{#if controller.activeTab === 'config'}
	<ControlCenterConfigPanel
		configSnapshot={controller.configSnapshot}
		configGroups={controller.configGroups}
		configSearch={controller.configSearch}
		showChangedOnly={controller.showChangedOnly}
		autoPresetMode={controller.autoPresetMode}
		composeGoal={controller.composeGoal}
		composeMinimal={controller.composeMinimal}
		configBusy={controller.configBusy}
		configMessage={controller.configMessage}
		pendingConfigCount={controller.pendingConfigCount}
		getDraftValue={controller.getDraftValue}
		isFieldChanged={controller.isFieldChanged}
		onSearchChange={(value) => (controller.configSearch = value)}
		onShowChangedOnlyChange={(checked) => (controller.showChangedOnly = checked)}
		onAutoPresetModeChange={(value) => (controller.autoPresetMode = value)}
		onComposeGoalChange={(value) => (controller.composeGoal = value)}
		onComposeMinimalChange={(checked) => (controller.composeMinimal = checked)}
		onRefreshConfig={controller.refreshConfig}
		onApplyConfigUpdates={controller.applyConfigUpdates}
		onApplyAutoPreset={controller.applyAutoPreset}
		onApplyComposeProfile={controller.applyComposeProfile}
		onTextDraftChange={controller.setDraftValue}
		onBoolDraftChange={controller.setDraftBool}
		onRevertField={controller.revertField}
	/>
{/if}

{#if controller.activeTab === 'blueprints'}
	<ControlCenterBlueprintsPanel
		blueprints={controller.blueprints}
		isExecuting={controller.isExecuting}
		categoryLabel={controller.categoryLabel}
		onExecute={controller.handleExecution}
	/>
{/if}

{#if controller.activeTab === 'plugins'}
	<ControlCenterPluginsPanel pluginRows={controller.pluginRows} onRefresh={controller.refreshPluginHealth} />
{/if}

{#if controller.activeTab === 'launcher'}
	<ControlCenterLauncherPanel launcherStatus={appState.launcherStatus} />
{/if}
