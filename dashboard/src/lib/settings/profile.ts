import {
	DEFAULT_SYNC_INTERVAL_MS,
	SETTINGS_PROFILE_VERSION,
	SYNC_INTERVAL_MAX_MS,
	SYNC_INTERVAL_MIN_MS,
	clampSyncInterval,
	isLocaleCode,
	isThemeMode,
	normalizeAgentUrl
} from '$lib/config/dashboard-settings';
import type { DashboardSettingsDraft, DashboardSettingsProfileV2 } from '$lib/types';

export type SettingsProfileValidationErrorCode =
	| 'invalid_payload'
	| 'invalid_sync_interval'
	| 'invalid_theme'
	| 'invalid_lang';

export class SettingsProfileValidationError extends Error {
	public readonly code: SettingsProfileValidationErrorCode;

	constructor(code: SettingsProfileValidationErrorCode, message: string) {
		super(message);
		this.code = code;
	}
}

type RawSettingsProfile = Partial<DashboardSettingsDraft> & {
	exportedAt?: unknown;
	version?: unknown;
};

function toDraft(raw: Partial<DashboardSettingsDraft>): DashboardSettingsDraft {
	return {
		agentUrl: normalizeAgentUrl(String(raw.agentUrl ?? '')),
		agentToken: String(raw.agentToken ?? '').trim(),
		launcherToken: String(raw.launcherToken ?? '').trim(),
		syncIntervalMs: clampSyncInterval(Number(raw.syncIntervalMs ?? DEFAULT_SYNC_INTERVAL_MS)),
		theme: isThemeMode(raw.theme) ? raw.theme : 'dark',
		lang: isLocaleCode(raw.lang) ? raw.lang : 'en'
	};
}

export function parseSettingsProfile(raw: unknown): DashboardSettingsProfileV2 {
	if (!raw || typeof raw !== 'object') {
		throw new SettingsProfileValidationError('invalid_payload', 'Invalid profile payload');
	}

	const item = raw as RawSettingsProfile;
	const syncIntervalMs = Number(item.syncIntervalMs ?? NaN);
	if (!Number.isFinite(syncIntervalMs)) {
		throw new SettingsProfileValidationError(
			'invalid_sync_interval',
			'syncIntervalMs must be a number'
		);
	}

	if (syncIntervalMs < SYNC_INTERVAL_MIN_MS || syncIntervalMs > SYNC_INTERVAL_MAX_MS) {
		throw new SettingsProfileValidationError(
			'invalid_sync_interval',
			`syncIntervalMs must be between ${SYNC_INTERVAL_MIN_MS} and ${SYNC_INTERVAL_MAX_MS}`
		);
	}

	if (!isThemeMode(item.theme)) {
		throw new SettingsProfileValidationError('invalid_theme', 'theme must be dark or light');
	}

	if (!isLocaleCode(item.lang)) {
		throw new SettingsProfileValidationError('invalid_lang', 'lang must be en or tr');
	}

	const draft = toDraft({
		agentUrl: item.agentUrl,
		agentToken: item.agentToken,
		launcherToken: item.launcherToken,
		syncIntervalMs,
		theme: item.theme,
		lang: item.lang
	});

	return {
		version: SETTINGS_PROFILE_VERSION,
		...draft,
		exportedAt: typeof item.exportedAt === 'string' ? item.exportedAt : new Date().toISOString()
	};
}

export function migrateSettingsProfile(raw: unknown): DashboardSettingsProfileV2 {
	if (!raw || typeof raw !== 'object') {
		throw new SettingsProfileValidationError('invalid_payload', 'Invalid profile payload');
	}

	const item = raw as RawSettingsProfile;
	return parseSettingsProfile({
		version: SETTINGS_PROFILE_VERSION,
		agentUrl: item.agentUrl,
		agentToken: item.agentToken,
		launcherToken: item.launcherToken,
		syncIntervalMs: item.syncIntervalMs ?? DEFAULT_SYNC_INTERVAL_MS,
		theme: item.theme ?? 'dark',
		lang: item.lang ?? 'en',
		exportedAt: item.exportedAt
	});
}

export function serializeSettingsProfile(
	draft: DashboardSettingsDraft
): DashboardSettingsProfileV2 {
	return {
		version: SETTINGS_PROFILE_VERSION,
		...toDraft(draft),
		exportedAt: new Date().toISOString()
	};
}
