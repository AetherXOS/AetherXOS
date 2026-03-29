<script lang="ts">
	import SystemDiagnosticsPanel from '$lib/components/SystemDiagnosticsPanel.svelte';
	import SettingsSetupNavigator from '$lib/components/settings/SettingsSetupNavigator.svelte';
	import SettingsConnectionSection from '$lib/components/settings/SettingsConnectionSection.svelte';
	import SettingsAuthSection from '$lib/components/settings/SettingsAuthSection.svelte';
	import SettingsInterfaceSection from '$lib/components/settings/SettingsInterfaceSection.svelte';
	import SettingsProfileSection from '$lib/components/settings/SettingsProfileSection.svelte';
	import SettingsVerifySection from '$lib/components/settings/SettingsVerifySection.svelte';
	import { m } from '$lib/paraglide/messages';
	import { orchestrator } from '$lib/services/orchestrator';
	import { DEFAULT_LOCAL_AGENT_TOKEN } from '$lib/config/dashboard-settings';
	import { migrateSettingsProfile, serializeSettingsProfile } from '$lib/settings/profile';
	import { appState } from '$lib/state.svelte';
	import type { DashboardSettingsDraft } from '$lib/types';
	import { MonitorCog, RefreshCw, Save } from 'lucide-svelte';

	appState.initializeSettings();

	let localUrl = $state(appState.agentUrl);
	let localToken = $state(appState.agentToken);
	let localLauncherToken = $state(appState.launcherToken);
	let localSyncIntervalMs = $state(appState.syncIntervalMs);
	let localTheme = $state(appState.theme);
	let localLang = $state(appState.lang);
	let isSaving = $state(false);
	let syncMessage = $state('');
	let profileMessage = $state('');
	let profileFileInput = $state<HTMLInputElement | null>(null);
	let isTesting = $state(false);
	let testResult = $state<'idle' | 'ok' | 'fail'>('idle');

	type SetupStep = { id: string; label: string; hint: string; done: boolean };

	const setupSteps = $derived.by((): SetupStep[] => [
		{ id: 'connection', label: 'Connection', hint: 'Set endpoint and sync interval', done: Boolean(localUrl) },
		{ id: 'authentication', label: 'Authentication', hint: 'Provide required tokens', done: Boolean(localToken || localLauncherToken) },
		{ id: 'verify', label: 'Verification', hint: 'Test and confirm connectivity', done: testResult === 'ok' || appState.isConnected },
		{ id: 'interface', label: 'Interface', hint: 'Choose theme and language', done: Boolean(localTheme && localLang) }
	]);

	const setupProgress = $derived(Math.round((setupSteps.filter((s) => s.done).length / setupSteps.length) * 100));

	const configMap = $derived.by(() => [
		{ id: 'connection', title: 'Connection', detail: 'Endpoint, sync cadence, reachability', subsections: [{ id: 'connection-endpoint', label: 'Endpoint' }, { id: 'connection-sync', label: 'Sync Policy' }, { id: 'connection-health', label: 'Health Check' }] },
		{ id: 'authentication', title: 'Authentication', detail: 'Agent and launcher tokens', subsections: [{ id: 'auth-agent', label: 'Agent Token' }, { id: 'auth-launcher', label: 'Launcher Token' }] },
		{ id: 'interface', title: 'Interface', detail: 'Theme and language', subsections: [{ id: 'ui-theme', label: 'Theme' }, { id: 'ui-language', label: 'Language' }] },
		{ id: 'profile', title: 'Profiles', detail: 'Import/export and portability', subsections: [{ id: 'profile-import', label: 'Import' }, { id: 'profile-export', label: 'Export' }] }
	]);

	function buildProfile(): DashboardSettingsDraft {
		return { agentUrl: localUrl, agentToken: localToken, launcherToken: localLauncherToken, syncIntervalMs: localSyncIntervalMs, theme: localTheme, lang: localLang };
	}

	function exportProfile() {
		profileMessage = '';
		const profile = serializeSettingsProfile(buildProfile());
		const blob = new Blob([JSON.stringify(profile, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = 'dashboard-profile.json';
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
		URL.revokeObjectURL(url);
		profileMessage = m.settings_export_success();
	}

	async function importProfile(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		profileMessage = '';
		try {
			const parsed = JSON.parse(await file.text());
			const profile = migrateSettingsProfile(parsed);
			localUrl = profile.agentUrl;
			localToken = profile.agentToken;
			localLauncherToken = profile.launcherToken;
			localSyncIntervalMs = profile.syncIntervalMs;
			localTheme = profile.theme;
			localLang = profile.lang;
			profileMessage = m.settings_import_success();
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			profileMessage = `${m.settings_import_failure()} ${message}`;
			appState.addAudit('SETTINGS_PROFILE_IMPORT_FAILED', 'failure');
		} finally {
			input.value = '';
		}
	}

	async function applyRegistrySync() {
		isSaving = true;
		syncMessage = '';
		try {
			appState.theme = localTheme;
			appState.lang = localLang;
			appState.applyConnectionSettings(buildProfile());
			orchestrator.dispose();
			await orchestrator.initialize();
			await orchestrator.sync(true);
			appState.addAudit('GLOBAL_REGISTRY_SYNC_COMPLETE');
			syncMessage = appState.isConnected ? m.settings_validation_success() : m.settings_validation_warn();
		} catch {
			appState.addAudit('GLOBAL_REGISTRY_SYNC_FAILED', 'failure');
			syncMessage = m.settings_validation_failure();
		} finally {
			isSaving = false;
		}
	}

	async function testConnection() {
		isTesting = true;
		testResult = 'idle';
		try {
			appState.applyConnectionSettings(buildProfile());
			await orchestrator.sync(true);
			testResult = appState.isConnected ? 'ok' : 'fail';
		} catch {
			testResult = 'fail';
		} finally {
			isTesting = false;
		}
	}

	function applyLocalDefaults() {
		localUrl = 'http://127.0.0.1:7401';
		localToken = DEFAULT_LOCAL_AGENT_TOKEN;
		localSyncIntervalMs = 2500;
		testResult = 'idle';
	}
</script>

<input bind:this={profileFileInput} type="file" class="hidden" accept="application/json" onchange={importProfile} />

<div class="space-y-8 pb-24">
	<header class="space-y-1">
		<h1 class="flex items-center gap-3 text-3xl font-black uppercase italic sm:text-4xl">
			<MonitorCog size={28} class="text-primary shrink-0" />
			{m.settings_title()}
		</h1>
		<p class="text-sm opacity-50">Configure agent connection, authentication, and interface preferences.</p>
	</header>

	<SettingsSetupNavigator {setupSteps} {setupProgress} {configMap} />

	<SystemDiagnosticsPanel />

	<SettingsConnectionSection
		{localUrl}
		{localSyncIntervalMs}
		isConnected={appState.isConnected}
		{isTesting}
		{testResult}
		onUrlChange={(v) => (localUrl = v)}
		onIntervalChange={(v) => (localSyncIntervalMs = v)}
		onTestConnection={testConnection}
		onApplyLocalDefaults={applyLocalDefaults}
	/>

	<SettingsAuthSection
		{localToken}
		{localLauncherToken}
		onTokenChange={(v) => (localToken = v)}
		onLauncherTokenChange={(v) => (localLauncherToken = v)}
	/>

	<SettingsInterfaceSection
		{localTheme}
		{localLang}
		onThemeChange={(v) => (localTheme = v)}
		onLangChange={(v) => (localLang = v)}
	/>

	<SettingsProfileSection
		{profileMessage}
		{profileFileInput}
		onExportProfile={exportProfile}
		onImportProfile={importProfile}
	/>

	<SettingsVerifySection
		{localUrl}
		{localToken}
		{localLauncherToken}
		isConnected={appState.isConnected}
		{isTesting}
		{isSaving}
		onTestConnection={testConnection}
		onSave={applyRegistrySync}
	/>

	{#if syncMessage}
		<div class="alert {appState.isConnected ? 'alert-success' : 'alert-warning'} text-sm">
			<span>{syncMessage}</span>
		</div>
	{/if}

	<div class="sticky bottom-0 z-30 -mx-4 border-t border-white/5 bg-base-100/80 px-4 py-4 backdrop-blur-xl sm:-mx-6 sm:px-6 lg:-mx-12 lg:px-12">
		<div class="mx-auto flex max-w-540 items-center justify-between gap-4">
			<div class="text-xs opacity-40">Changes apply on Save & Connect. Settings are persisted in local storage.</div>
			<button class="btn btn-primary gap-2 shrink-0" onclick={applyRegistrySync} disabled={isSaving}>
				{#if isSaving}
					<RefreshCw size={15} class="animate-spin" />
					{m.settings_saving()}
				{:else}
					<Save size={15} />
					{m.settings_save()}
				{/if}
			</button>
		</div>
	</div>
</div>
