export const API_RESILIENCE = {
	retryableStatuses: [408, 425, 429, 500, 502, 503, 504] as number[],
	maxRetryAttempts: 3,
	circuitOpenAfterFailures: 4,
	circuitOpenMs: 10_000,
	defaultTimeoutMs: 12_000,
	baseRetryDelayMs: 250,
	retryJitterMs: 120
};

export const LIVE_STREAM = {
	// Launcher stream endpoints are optional; when unavailable we stay in polling mode.
	paths: [] as const,
	initialReconnectDelayMs: 1500,
	maxReconnectDelayMs: 15_000
};

export const CONTROL_CENTER = {
	blueprintCacheKey: 'dashboard.blueprints.cache.v1',
	blueprintCacheTtlMs: 60_000
};
