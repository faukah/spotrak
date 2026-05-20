<script lang="ts">
	import { THEMES, type ChartConfig } from "./chart-utils.js";

	let { id, config }: { id: string; config: ChartConfig } = $props();

	const colorConfig = $derived(
		config ? Object.entries(config).filter(([, itemConfig]) => itemConfig.theme || itemConfig.color) : null
	);

	const contents = $derived.by(() => {
		if (!colorConfig || !colorConfig.length) return;

		const themeBlocks = [];
		for (const [themeName, prefix] of Object.entries(THEMES)) {
			let content = `${prefix} [data-chart=${id}] {\n`;
			const declarations = colorConfig.map(([key, itemConfig]) => {
				const theme = themeName as keyof typeof itemConfig.theme;
				const resolvedColor = itemConfig.theme?.[theme] || itemConfig.color;
				return resolvedColor ? `\t--color-${key}: ${resolvedColor};` : null;
			});

			content += declarations.join("\n") + "\n}";

			themeBlocks.push(content);
		}

		return themeBlocks.join("\n");
	});
</script>

{#if contents}
	{#key id}
		<svelte:element this={"style"}>
			{contents}
		</svelte:element>
	{/key}
{/if}
