<script lang="ts">
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { getVersion } from "@tauri-apps/api/app";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "./lib/api";
  import { theme, toggleTheme } from "./lib/theme";
  import { SUPPORTED_LOCALES, LOCALE_NAMES, setLocale, type SupportedLocale } from "./i18n";
  import Modal from "./components/Modal.svelte";
  import AboutDialog from "./components/AboutDialog.svelte";
  import WebAppRow from "./components/WebAppRow.svelte";
  import WebAppFormModal from "./components/WebAppFormModal.svelte";
  import type { Browser, WebApp, WebAppInput } from "./lib/webapps";
  import { Plus, Minus, Moon, Sun, Globe } from "lucide-svelte";

  let webapps: WebApp[] = [];
  let browsers: Browser[] = [];
  let selectedId: string | null = null;
  let loading = true;

  let formTarget: { mode: "add"; initialUrl?: string } | { mode: "edit"; webapp: WebApp } | null = null;
  let deleteTarget: WebApp | null = null;
  let aboutOpen = false;

  let envInfo = { desktopEnvironment: "unknown", displayProtocol: "unknown" };
  let appVersion = "";

  async function loadWebapps() {
    try {
      webapps = await api.invoke<WebApp[]>("list_webapps");
    } catch (error) {
      console.error("Failed to load webapps", error);
    }
  }

  async function refreshBrowsers() {
    browsers = await api.invoke<Browser[]>("list_browsers");
  }

  onMount(async () => {
    try {
      envInfo = await api.invoke<{ desktopEnvironment: string; displayProtocol: string }>("get_environment_info");
    } catch (error) {
      console.error("Failed to read environment info", error);
    }
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = "";
    }
    await Promise.all([loadWebapps(), refreshBrowsers()]);
    loading = false;
  });

  function openAdd() {
    selectedId = null;
    refreshBrowsers();
    formTarget = { mode: "add" };
  }

  function openEdit(w: WebApp) {
    refreshBrowsers();
    formTarget = { mode: "edit", webapp: w };
  }

  async function saveWebapp(input: WebAppInput) {
    if (formTarget?.mode === "edit") {
      await api.invoke("update_webapp", { id: formTarget.webapp.id, input });
    } else {
      await api.invoke("create_webapp", { input });
    }
    formTarget = null;
    await loadWebapps();
  }

  async function runWebapp(w: WebApp) {
    try {
      await api.invoke("run_webapp", { id: w.id });
    } catch (error) {
      console.error("Failed to run webapp", error);
    }
  }

  function askDelete(w: WebApp) {
    deleteTarget = w;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    await api.invoke("delete_webapp", { id: deleteTarget.id });
    deleteTarget = null;
    selectedId = null;
    await loadWebapps();
  }

  function onLanguageChange(e: Event) {
    setLocale((e.currentTarget as HTMLSelectElement).value as SupportedLocale);
  }

  // === Global keyboard shortcuts ===
  // F1 -> About, Ctrl+Q -> quit, Ctrl+N -> add, Delete -> remove selected
  // (Ctrl+Q/N and Delete are ignored while a dialog is open, and Delete is
  // also ignored while focus is in a text field, so it keeps editing text
  // as expected there instead of deleting the selected row).
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "F1") {
      e.preventDefault();
      aboutOpen = true;
      return;
    }
    if (e.ctrlKey && (e.key === "q" || e.key === "Q")) {
      e.preventDefault();
      getCurrentWindow().close();
      return;
    }
    if (formTarget || deleteTarget || aboutOpen) return;

    if (e.ctrlKey && (e.key === "n" || e.key === "N")) {
      e.preventDefault();
      openAdd();
      return;
    }

    const target = e.target as HTMLElement | null;
    const typing = !!target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);
    if (!typing && e.key === "Delete" && selectedId) {
      e.preventDefault();
      const w = webapps.find((x) => x.id === selectedId);
      if (w) askDelete(w);
    }
  }

  // === Drag and drop a link from the browser to create a WebApp ===
  // Requires "dragDropEnabled": false on the window in tauri.conf.json —
  // Tauri intercepts drag-and-drop at the window level by default, which
  // stops these DOM events from ever firing.
  let dragDepth = 0;
  $: isDraggingOver = dragDepth > 0;

  function extractDroppedUrl(dataTransfer: DataTransfer): string | null {
    const raw = dataTransfer.getData("text/uri-list") || dataTransfer.getData("text/plain");
    const candidate = raw
      .split("\n")
      .map((line) => line.trim())
      .find((line) => line && !line.startsWith("#"));
    if (!candidate) return null;
    try {
      const parsed = new URL(candidate);
      return parsed.protocol === "http:" || parsed.protocol === "https:" ? candidate : null;
    } catch {
      return null;
    }
  }

  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    dragDepth++;
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function handleDragLeave() {
    dragDepth = Math.max(0, dragDepth - 1);
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragDepth = 0;
    if (formTarget || deleteTarget || aboutOpen || !e.dataTransfer) return;
    const url = extractDroppedUrl(e.dataTransfer);
    if (!url) return;
    selectedId = null;
    refreshBrowsers();
    formTarget = { mode: "add", initialUrl: url };
  }
</script>

<svelte:window
  on:keydown={handleKeydown}
  on:dragenter={handleDragEnter}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
/>

<div class="relative flex flex-col h-screen overflow-hidden">
  {#if isDraggingOver}
    <div class="absolute inset-0 z-40 border-2 border-dashed border-primary bg-primary/10 flex items-center justify-center pointer-events-none">
      <p class="text-[0.95rem] font-semibold text-primary">{$_("webapp.dropHint")}</p>
    </div>
  {/if}

  <div class="p-4 pb-0 shrink-0">
    <div
      class="relative rounded border border-border overflow-hidden flex flex-col items-start justify-end p-4 min-h-[110px]"
      style="background: linear-gradient(rgba(20, 20, 22, 0.25), rgba(20, 20, 22, 0.82)), url('/assets/hero/webapp-manager-background.jpg'); background-size: cover; background-position: center;"
    >
      <div class="absolute top-3 right-3">
        <select
          value={$locale}
          on:change={onLanguageChange}
          class="appearance-none bg-primary/20 backdrop-blur-sm border border-primary/30 text-primary rounded px-2 py-1 text-[0.7rem] focus:outline-none"
          title={$_("sidebar.language")}
        >
          {#each SUPPORTED_LOCALES as code}
            <option value={code}>{LOCALE_NAMES[code]}</option>
          {/each}
        </select>
      </div>

      <div class="absolute bottom-3 right-3 flex items-center gap-1.5">
        <button
          class="p-1.5 rounded bg-primary/20 backdrop-blur-sm border border-primary/30 text-primary hover:bg-primary/30"
          title={$_("webapp.addTooltip")}
          on:click={openAdd}
        >
          <Plus size={14} />
        </button>
        <button
          class="p-1.5 rounded bg-primary/20 backdrop-blur-sm border border-primary/30 text-primary hover:bg-primary/30 disabled:opacity-40 disabled:pointer-events-none"
          title={$_("webapp.removeTooltip")}
          disabled={!selectedId}
          on:click={() => {
            const w = webapps.find((x) => x.id === selectedId);
            if (w) askDelete(w);
          }}
        >
          <Minus size={14} />
        </button>
      </div>
      <p class="text-white font-semibold text-[0.95rem]">{$_("hero.title")}</p>
      <p class="text-[0.75rem] text-white/85 mt-0.5">{$_("hero.description")}</p>
    </div>
  </div>

  <div class="flex-1 overflow-y-auto content-scroll p-4">
    {#if !loading && webapps.length === 0}
      <div class="h-full flex flex-col items-center justify-center gap-2 text-center">
        <Globe size={64} class="text-text-muted" />
        <p class="text-[0.9rem] text-text-bright">{$_("webapp.emptyTitle")}</p>
        <p class="text-[0.8rem] text-text-muted">{$_("webapp.emptyHint")}</p>
      </div>
    {:else}
      <div class="flex flex-col gap-2">
        {#each webapps as w (w.id)}
          <div
            role="button"
            tabindex="0"
            class="rounded {selectedId === w.id ? 'ring-1 ring-primary' : ''}"
            on:click={() => (selectedId = w.id)}
            on:keydown={(e) => e.key === "Enter" && (selectedId = w.id)}
          >
            <WebAppRow webapp={w} onRun={runWebapp} onEdit={openEdit} onDelete={askDelete} />
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <footer class="flex items-center justify-between px-4 py-1.5 border-t border-border text-[0.7rem] text-text-muted shrink-0">
    <div class="flex items-center gap-1.5">
      <button
        class="p-1 -m-1 rounded text-text-muted hover:bg-surface hover:text-text-bright"
        on:click={toggleTheme}
        title={$theme === "dark" ? $_("sidebar.switchToLight") : $_("sidebar.switchToDark")}
      >
        <svelte:component this={$theme === "dark" ? Moon : Sun} size={13} />
      </button>
      <span>
        {$_(`environment.${envInfo.desktopEnvironment}`)} · {$_(`environment.${envInfo.displayProtocol}`)}
      </span>
    </div>
    <button
      class="text-text-muted hover:text-text-bright hover:underline"
      on:click={() => (aboutOpen = true)}
      title={$_("sidebar.aboutTooltip")}
    >
      {appVersion}
    </button>
  </footer>
</div>

{#if formTarget}
  <WebAppFormModal
    webapp={formTarget.mode === "edit" ? formTarget.webapp : null}
    initialUrl={formTarget.mode === "add" ? (formTarget.initialUrl ?? "") : ""}
    {browsers}
    onSave={saveWebapp}
    onClose={() => (formTarget = null)}
  />
{/if}

{#if deleteTarget}
  <Modal
    variant="danger"
    title={$_("webapp.deleteConfirmTitle", { values: { name: deleteTarget.name } })}
    confirmLabel={$_("webapp.delete")}
    onClose={() => (deleteTarget = null)}
    onConfirm={confirmDelete}
  >
    <p>{$_("webapp.deleteConfirmBody")}</p>
  </Modal>
{/if}

{#if aboutOpen}
  <AboutDialog onClose={() => (aboutOpen = false)} />
{/if}
