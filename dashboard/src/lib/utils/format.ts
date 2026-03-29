/**
 * @file Dashboard formatting utilities
 */

/**
 * Formats a Date into a standard industrial log timestamp.
 * Example: 13:42:01
 */
export function formatLogTimestamp(date: Date = new Date()): string {
	return date.toLocaleTimeString('en-GB', {
		hour12: false,
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit'
	});
}

/**
 * Formats bytes into a human readable string.
 */
export function formatBytes(bytes: number, decimals: number = 2): string {
	if (bytes === 0) return '0 Bytes';
	const k = 1024;
	const dm = decimals < 0 ? 0 : decimals;
	const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

/**
 * Formats seconds into a human-readable uptime string.
 * Examples: 45 → "45s", 125 → "2m 5s", 7321 → "2h 2m"
 */
export function formatUptime(seconds: number): string {
	const s = Math.floor(Math.max(0, seconds));
	if (s < 60) return `${s}s`;
	const m = Math.floor(s / 60);
	if (m < 60) {
		const rem = s % 60;
		return rem > 0 ? `${m}m ${rem}s` : `${m}m`;
	}
	const h = Math.floor(m / 60);
	const remM = m % 60;
	return remM > 0 ? `${h}h ${remM}m` : `${h}h`;
}

/**
 * Formats a latency value in ms to a human-readable string.
 * Examples: 0 → "<1ms", 250 → "250ms", 1200 → "1.20s"
 */
export function formatLatency(ms: number): string {
	if (ms < 1) return '<1ms';
	if (ms < 1000) return `${Math.round(ms)}ms`;
	return `${(ms / 1000).toFixed(2)}s`;
}

/**
 * Formats a numeric value as a percentage string.
 */
export function formatPercent(value: number, decimals = 1): string {
	return `${value.toFixed(decimals)}%`;
}
