import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AgentRepo } from './index';
import { appState } from '$lib/state.svelte';

function createSseResponse(chunks: string[]): Response {
	const stream = new ReadableStream<Uint8Array>({
		start(controller) {
			const encoder = new TextEncoder();
			for (const chunk of chunks) {
				controller.enqueue(encoder.encode(chunk));
			}
			controller.close();
		}
	});
	return new Response(stream, {
		status: 200,
		headers: { 'Content-Type': 'text/event-stream' }
	});
}

describe('AgentRepo.streamJob', () => {
	beforeEach(() => {
		vi.restoreAllMocks();
		appState.agentUrl = 'http://127.0.0.1:7401';
		appState.agentToken = 'test-token';
	});

	it('parses snapshot/line/complete events from SSE', async () => {
		const fetchMock = vi
			.spyOn(globalThis, 'fetch')
			.mockResolvedValueOnce(
				createSseResponse([
					'data: {"type":"snapshot","id":"job-1","action":"doctor","status":"running","line_count":1}\n\n',
					'data: {"type":"line","index":1,"line":"hello"}\n\n',
					'data: {"type":"complete","id":"job-1","status":"done"}\n\n'
				])
			);

		const types: string[] = [];
		const messages: string[] = [];
		await AgentRepo.streamJob({
			id: 'job-1',
			onEvent: (event) => {
				types.push(event.type);
				if (event.message) messages.push(event.message);
			}
		});

		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(types).toContain('snapshot');
		expect(types).toContain('line');
		expect(types).toContain('complete');
		expect(messages).toContain('hello');
	});

	it('emits timeout when stream closes without completion', async () => {
		vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
			createSseResponse([
				'data: {"type":"snapshot","id":"job-2","action":"doctor","status":"running","line_count":0}\n\n'
			])
		);

		const types: string[] = [];
		await AgentRepo.streamJob({
			id: 'job-2',
			onEvent: (event) => {
				types.push(event.type);
			}
		});

		expect(types).toContain('snapshot');
		expect(types).toContain('timeout');
	});
});
