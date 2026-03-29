import { m } from '$lib/paraglide/messages';
import { appState } from '$lib/state.svelte';

export type WorkflowStage = 'idle' | 'running' | 'done' | 'failed';

export type AutoAction = {
	id: string;
	tone: 'success' | 'warning' | 'error' | 'info';
	title: string;
	detail: string;
	cta: string;
	precheck: () => boolean;
	run: () => Promise<void>;
	verify: () => boolean;
};

type WorkflowStep = {
	id: 'precheck' | 'execute' | 'verify' | 'audit';
	label: string;
	state: WorkflowStage;
};

export class AutonomyRunner {
	busyId = $state('');
	workflow = $state<WorkflowStep[]>([
		{ id: 'precheck', label: m.cc_stage_precheck(), state: 'idle' },
		{ id: 'execute', label: m.cc_stage_execute(), state: 'idle' },
		{ id: 'verify', label: m.cc_stage_verify(), state: 'idle' },
		{ id: 'audit', label: m.cc_stage_audit(), state: 'idle' }
	]);
	message = $state('');

	setStage(id: WorkflowStep['id'], stage: WorkflowStage): void {
		this.workflow = this.workflow.map((step) => (step.id === id ? { ...step, state: stage } : step));
	}

	reset(): void {
		this.workflow = this.workflow.map((step) => ({ ...step, state: 'idle' as WorkflowStage }));
		this.message = '';
	}

	runById(actionId: string, actions: AutoAction[]): void {
		const target = actions.find((item) => item.id === actionId);
		if (target) void this.run(target);
	}

	async run(action: AutoAction): Promise<void> {
		this.reset();
		this.busyId = action.id;
		try {
			this.setStage('precheck', 'running');
			if (!action.precheck()) {
				this.setStage('precheck', 'failed');
				this.message = m.cc_auto_precheck_failed();
				return;
			}
			this.setStage('precheck', 'done');

			this.setStage('execute', 'running');
			await action.run();
			this.setStage('execute', 'done');

			this.setStage('verify', 'running');
			if (!action.verify()) {
				this.setStage('verify', 'failed');
				this.message = m.cc_auto_verify_failed();
				return;
			}
			this.setStage('verify', 'done');

			this.setStage('audit', 'running');
			appState.addAudit(`AUTONOMY_WORKFLOW_OK:${action.id}`);
			this.setStage('audit', 'done');
			this.message = m.cc_auto_success();
		} catch {
			appState.addAudit(`AUTONOMY_WORKFLOW_FAIL:${action.id}`, 'failure');
			this.message = m.cc_auto_failed();
		} finally {
			this.busyId = '';
		}
	}
}

export function createAutonomyRunner(): AutonomyRunner {
	return new AutonomyRunner();
}
