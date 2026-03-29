import { normalizeStreamEvent } from '$lib/api/contracts';
import { LIVE_STREAM } from '$lib/config/runtime-client';
import { appState } from '$lib/state.svelte';

export type StreamEventHandler = (payload: unknown) => void;

interface StreamHandlers {
	onAudit: StreamEventHandler;
	onIncident: StreamEventHandler;
	onStatus?: (status: { connected: boolean; path: string; reconnectDelayMs: number }) => void;
}

class LiveStreamService {
	private source: EventSource | null = null;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private pathIndex = 0;
	private reconnectDelayMs = LIVE_STREAM.initialReconnectDelayMs;
	private handlers: StreamHandlers | null = null;

	public start(handlers: StreamHandlers): void {
		if (typeof window === 'undefined') {
			return;
		}

		this.handlers = handlers;
		this.stop();
		this.connect();
	}

	public stop(): void {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}

		if (this.source) {
			this.source.close();
			this.source = null;
		}

		appState.streamConnected = false;
		appState.liveMode = 'polling';
	}

	private connect(): void {
		if (!this.handlers) {
			return;
		}
		if (LIVE_STREAM.paths.length === 0) {
			appState.streamConnected = false;
			appState.liveMode = 'polling';
			this.handlers.onStatus?.({
				connected: false,
				path: 'disabled',
				reconnectDelayMs: this.reconnectDelayMs
			});
			return;
		}

		const path = LIVE_STREAM.paths[this.pathIndex % LIVE_STREAM.paths.length];
		const tokenParam = appState.launcherToken
			? `?token=${encodeURIComponent(appState.launcherToken)}`
			: '';
		const source = new EventSource(`${path}${tokenParam}`);

		source.onopen = () => {
			appState.streamConnected = true;
			appState.liveMode = 'streaming';
			this.reconnectDelayMs = LIVE_STREAM.initialReconnectDelayMs;
			this.handlers?.onStatus?.({
				connected: true,
				path,
				reconnectDelayMs: this.reconnectDelayMs
			});
		};

		source.onmessage = (event) => {
			const normalized = normalizeStreamEvent(this.tryParse(event.data));
			if (!normalized) {
				return;
			}

			if (normalized.type === 'audit') {
				this.handlers?.onAudit(normalized);
				return;
			}

			this.handlers?.onIncident(normalized);
		};

		source.onerror = () => {
			source.close();
			if (this.source === source) {
				this.source = null;
			}

			appState.streamConnected = false;
			appState.liveMode = 'polling';
			this.handlers?.onStatus?.({
				connected: false,
				path,
				reconnectDelayMs: this.reconnectDelayMs
			});
			this.pathIndex += 1;
			this.scheduleReconnect();
		};

		this.source = source;
	}

	private scheduleReconnect(): void {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
		}

		this.reconnectTimer = setTimeout(() => {
			this.connect();
		}, this.reconnectDelayMs);

		this.reconnectDelayMs = Math.min(
			this.reconnectDelayMs * 2,
			LIVE_STREAM.maxReconnectDelayMs
		);
	}

	private tryParse(raw: string): Record<string, unknown> | null {
		try {
			const parsed = JSON.parse(raw);
			if (parsed && typeof parsed === 'object') {
				return parsed as Record<string, unknown>;
			}
			return null;
		} catch {
			return null;
		}
	}
}

export const liveStream = new LiveStreamService();
