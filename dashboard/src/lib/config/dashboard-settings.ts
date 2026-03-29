import type { LocaleCode, ThemeMode } from '$lib/types';

export const STORAGE_KEY = 'dashboard.settings.v2';
export const SETTINGS_PROFILE_VERSION = 2;
export const DEFAULT_AGENT_URL = 'http://127.0.0.1:7401';
export const DEFAULT_LOCAL_AGENT_TOKEN = 'hypercore-local-dev-token';
export const SYNC_INTERVAL_MIN_MS = 1000;
export const SYNC_INTERVAL_MAX_MS = 60000;
export const DEFAULT_SYNC_INTERVAL_MS = 5000;

export function clampSyncInterval(value: number): number {
	return Math.min(SYNC_INTERVAL_MAX_MS, Math.max(SYNC_INTERVAL_MIN_MS, Math.round(value)));
}

export function normalizeAgentUrl(raw: string): string {
	const value = String(raw || '').trim();
	if (!value) {
		return DEFAULT_AGENT_URL;
	}
	return value.replace(/\/+$/, '');
}

export function isThemeMode(value: unknown): value is ThemeMode {
	return value === 'dark' || value === 'light';
}

export function isLocaleCode(value: unknown): value is LocaleCode {
	return value === 'en' || value === 'tr';
}
