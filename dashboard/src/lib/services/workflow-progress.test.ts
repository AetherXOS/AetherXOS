import { describe, expect, it, beforeEach } from 'vitest';
import {
	getWorkflowProgressPercent,
	getVisitedRoutes,
	markRouteVisited,
	setTaskDone,
	isTaskDone
} from './workflow-progress';

const storage = new Map<string, string>();

const localStorageMock = {
	getItem(key: string): string | null {
		return storage.has(key) ? (storage.get(key) ?? null) : null;
	},
	setItem(key: string, value: string): void {
		storage.set(key, value);
	},
	removeItem(key: string): void {
		storage.delete(key);
	},
	clear(): void {
		storage.clear();
	}
};

Object.defineProperty(globalThis, 'window', {
	value: { localStorage: localStorageMock },
	writable: true,
	configurable: true
});

describe('workflow-progress', () => {
	beforeEach(() => {
		localStorageMock.clear();
	});

	it('tracks visited routes and computes percentage', () => {
		markRouteVisited('/settings');
		markRouteVisited('/executive');
		const visited = getVisitedRoutes();
		expect(visited['/settings']).toBe(true);
		expect(visited['/executive']).toBe(true);
		expect(getWorkflowProgressPercent()).toBe(40);
	});

	it('tracks custom task completion', () => {
		expect(isTaskDone('recovery-pack')).toBe(false);
		setTaskDone('recovery-pack', true);
		expect(isTaskDone('recovery-pack')).toBe(true);
	});
});
