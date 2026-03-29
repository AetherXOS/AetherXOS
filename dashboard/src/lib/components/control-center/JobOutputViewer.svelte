<script lang="ts">
	interface OutputLine {
		raw: string;
		severity: 'error' | 'warn' | 'info' | 'normal';
	}

	interface Props {
		outputLines: OutputLine[];
	}

	let { outputLines }: Props = $props();

	let outputRef: HTMLDivElement | null = null;
	let outputAutoFollow = $state(true);
	let outputSearch = $state('');
	let outputWrap = $state(false);

	const filteredOutputLines = $derived.by(() => {
		const q = outputSearch.trim().toLowerCase();
		if (!q) return outputLines;
		return outputLines.filter((l) => l.raw.toLowerCase().includes(q));
	});

	const matchCount = $derived.by(() => {
		if (!outputSearch.trim()) return outputLines.length;
		return filteredOutputLines.length;
	});

	$effect(() => {
		const lines = filteredOutputLines;
		if (!lines.length) return;
		if (!outputAutoFollow || !outputRef) return;
		outputRef.scrollTop = outputRef.scrollHeight;
	});

	async function copyOutput(): Promise<void> {
		try {
			await navigator.clipboard.writeText(filteredOutputLines.map((l) => l.raw).join('\n'));
		} catch {
			// ignored
		}
	}
</script>

<!-- Output toolbar -->
<div class="mt-4 flex flex-wrap items-center justify-between gap-2 text-xs">
	<div class="flex items-center gap-2">
		<label class="label cursor-pointer gap-1.5 py-0">
			<span class="label-text text-xs opacity-70">auto follow</span>
			<input class="checkbox checkbox-xs" type="checkbox" checked={outputAutoFollow} onchange={(e) => (outputAutoFollow = (e.currentTarget as HTMLInputElement).checked)} />
		</label>
		<label class="label cursor-pointer gap-1.5 py-0">
			<span class="label-text text-xs opacity-70">wrap</span>
			<input class="checkbox checkbox-xs" type="checkbox" checked={outputWrap} onchange={(e) => (outputWrap = (e.currentTarget as HTMLInputElement).checked)} />
		</label>
	</div>
	<div class="flex items-center gap-2">
		<input class="input input-xs bg-base-100 w-40" type="text" placeholder="Search output…" bind:value={outputSearch} />
		<span class="opacity-50">{matchCount} line{matchCount === 1 ? '' : 's'}</span>
		<button class="btn btn-ghost btn-xs" onclick={copyOutput}>Copy</button>
	</div>
</div>

<!-- Output viewer -->
<div
	bind:this={outputRef}
	class="mt-2 max-h-136 overflow-auto rounded-2xl border border-white/8 bg-neutral text-neutral-content p-4 text-xs leading-6 font-mono"
>
	{#if filteredOutputLines.length === 0}
		<span class="opacity-40 italic">{outputSearch ? 'No matching lines.' : 'No job output loaded.'}</span>
	{:else}
		<div class={outputWrap ? 'whitespace-pre-wrap break-all' : 'whitespace-pre'}>
			{#each filteredOutputLines as line, i (i)}
				{#if line.severity === 'error'}
					<div class="text-error/90">{line.raw || '\u200b'}</div>
				{:else if line.severity === 'warn'}
					<div class="text-warning/90">{line.raw || '\u200b'}</div>
				{:else if line.severity === 'info'}
					<div class="text-success/80">{line.raw || '\u200b'}</div>
				{:else}
					<div>{line.raw || '\u200b'}</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
