export type WorkflowRoute = '/executive' | '/operations' | '/control-center' | '/deep-debug' | '/settings';

const VISITED_KEY = 'dashboard.workflow.visited.v1';
const TASKS_KEY = 'dashboard.workflow.tasks.v1';

function readJson<T>(key: string, fallback: T): T {
	if (typeof window === 'undefined') return fallback;
	try {
		const raw = window.localStorage.getItem(key);
		if (!raw) return fallback;
		return JSON.parse(raw) as T;
	} catch {
		return fallback;
	}
}

function writeJson<T>(key: string, value: T): void {
	if (typeof window === 'undefined') return;
	window.localStorage.setItem(key, JSON.stringify(value));
}

export function markRouteVisited(pathname: string): void {
	if (typeof window === 'undefined') return;
	const key = pathname as WorkflowRoute;
	const allowed: WorkflowRoute[] = [
		'/executive',
		'/operations',
		'/control-center',
		'/deep-debug',
		'/settings'
	];
	if (!allowed.includes(key)) return;

	const current = readJson<Record<string, boolean>>(VISITED_KEY, {});
	if (current[key]) return;
	current[key] = true;
	writeJson(VISITED_KEY, current);
}

export function getVisitedRoutes(): Record<string, boolean> {
	return readJson<Record<string, boolean>>(VISITED_KEY, {});
}

export function getWorkflowProgressPercent(): number {
	const visited = getVisitedRoutes();
	const targets: WorkflowRoute[] = [
		'/settings',
		'/executive',
		'/operations',
		'/control-center',
		'/deep-debug'
	];
	const done = targets.filter((route) => Boolean(visited[route])).length;
	return Math.round((done / targets.length) * 100);
}

export function setTaskDone(id: string, done: boolean): void {
	const tasks = readJson<Record<string, boolean>>(TASKS_KEY, {});
	tasks[id] = done;
	writeJson(TASKS_KEY, tasks);
}

export function isTaskDone(id: string): boolean {
	const tasks = readJson<Record<string, boolean>>(TASKS_KEY, {});
	return Boolean(tasks[id]);
}

export function getTaskMap(): Record<string, boolean> {
	return readJson<Record<string, boolean>>(TASKS_KEY, {});
}
