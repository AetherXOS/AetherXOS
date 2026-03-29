<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { LauncherStatus } from '$lib/types';

	interface Props {
		launcherStatus: LauncherStatus;
	}

	let { launcherStatus }: Props = $props();
</script>

<div class="card space-y-3 rounded-3xl border border-white/5 bg-base-200/80 p-5 text-sm sm:p-6">
	<div>
		{m.control_launcher_status()}: <span class="font-black">{launcherStatus.status}</span>
	</div>
	<div>{m.control_launcher_pid()}: {launcherStatus.pid ?? '-'}</div>
	<div class="opacity-60">{m.control_launcher_updated_at()}: {launcherStatus.updatedAt ?? '-'}</div>
</div>
