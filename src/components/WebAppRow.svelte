<script lang="ts">
  import { _ } from "svelte-i18n";
  import { Play, Pencil, Trash2, Globe } from "lucide-svelte";
  import type { WebApp } from "../lib/webapps";

  export let webapp: WebApp;
  export let onRun: (w: WebApp) => void;
  export let onEdit: (w: WebApp) => void;
  export let onDelete: (w: WebApp) => void;
</script>

<div class="border border-border rounded p-3 flex items-center gap-3">
  <div class="w-11 h-11 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden">
    {#if webapp.iconPreview}
      <img src={webapp.iconPreview} alt="" class="w-full h-full object-contain" />
    {:else}
      <Globe size={20} class="text-text-muted" />
    {/if}
  </div>
  <div class="min-w-0 flex-1">
    <div class="text-[0.88rem] font-semibold text-text-bright truncate">{webapp.name}</div>
    <div class="text-[0.72rem] text-text-muted truncate">{webapp.url}</div>
  </div>
  <div class="flex items-center gap-1 shrink-0">
    <button
      class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-primary"
      title={$_("webapp.run")}
      on:click={() => onRun(webapp)}
    >
      <Play size={15} />
    </button>
    <button
      class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-text-bright"
      title={$_("webapp.edit")}
      on:click={() => onEdit(webapp)}
    >
      <Pencil size={15} />
    </button>
    <button
      class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-danger"
      title={$_("webapp.delete")}
      on:click={() => onDelete(webapp)}
    >
      <Trash2 size={15} />
    </button>
  </div>
</div>
