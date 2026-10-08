<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { HelpCircle, Globe } from "lucide-svelte";
  import { api } from "../lib/api";
  import Modal from "./Modal.svelte";
  import { CATEGORIES, CATEGORY_FREEDESKTOP_ID, type Browser, type WebApp, type WebAppInput } from "../lib/webapps";

  // Passing an existing webapp switches this into edit mode: the category
  // selector is hidden (2.x's edit dialog didn't have one either, and the
  // backend keeps whatever category a webapp already has when none is sent).
  export let webapp: WebApp | null = null;
  export let browsers: Browser[];
  export let onSave: (input: WebAppInput) => void;
  export let onClose: () => void;
  // Set when the form was opened by dropping a link from the browser onto
  // the window — prefills the URL and name and kicks off the same favicon
  // lookup a manual blur on the URL field would trigger.
  export let initialUrl = "";

  const isEdit = webapp !== null;

  function guessNameFromUrl(rawUrl: string): string {
    try {
      const host = new URL(rawUrl).hostname.replace(/^www\./, "");
      const label = host.split(".")[0];
      return label.charAt(0).toUpperCase() + label.slice(1);
    } catch {
      return "";
    }
  }

  let name = webapp?.name ?? (initialUrl ? guessNameFromUrl(initialUrl) : "");
  let url = webapp?.url ?? initialUrl;
  let iconPath = webapp?.iconPath ?? "";
  let iconPreview = webapp?.iconPreview ?? null;
  let category: (typeof CATEGORIES)[number] = "network";
  let browserId = webapp?.browserId ?? browsers[0]?.id ?? "";
  let showNavbar = webapp?.showNavbar ?? false;
  let isIncognito = webapp?.isIncognito ?? false;
  let extraParams = webapp?.extraParams ?? "";
  let helpOpen = false;
  let faviconBusy = false;
  let iconManuallyChosen = isEdit; // Don't overwrite an already-set icon on edit.

  async function pickIcon() {
    const path = await api.invoke<string | null>("pick_icon_file");
    if (!path) return;
    iconPath = path;
    iconManuallyChosen = true;
    iconPreview = await api.invoke<string | null>("read_icon_preview", { path });
  }

  async function onUrlBlur() {
    if (iconManuallyChosen || !url.trim()) return;
    faviconBusy = true;
    try {
      const path = await api.invoke<string | null>("download_favicon", { url: url.trim() });
      if (path) {
        iconPath = path;
        iconPreview = await api.invoke<string | null>("read_icon_preview", { path });
      }
    } catch (error) {
      console.error("Failed to fetch favicon", error);
    } finally {
      faviconBusy = false;
    }
  }

  onMount(() => {
    if (initialUrl) onUrlBlur();
  });

  $: canSave = name.trim() !== "" && url.trim() !== "" && browserId !== "";

  function submit() {
    if (!canSave) return;
    onSave({
      name: name.trim(),
      url: url.trim(),
      iconPath: iconPath || "applications-internet",
      category: isEdit ? "" : CATEGORY_FREEDESKTOP_ID[category],
      browserId,
      showNavbar,
      extraParams: extraParams.trim(),
      isIncognito,
    });
  }
</script>

<Modal
  variant="confirm"
  maxWidth={460}
  title={isEdit ? $_("webapp.editTitle") : $_("webapp.addTitle")}
  confirmLabel={$_("common.save")}
  confirmDisabled={!canSave}
  {onClose}
  onConfirm={submit}
>
  <div class="flex flex-col gap-3.5">
    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("webapp.name")}</span>
      <input
        type="text"
        bind:value={name}
        placeholder={$_("webapp.namePlaceholder")}
        class="px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
      />
    </label>

    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("webapp.url")}</span>
      <input
        type="text"
        bind:value={url}
        on:blur={onUrlBlur}
        placeholder={$_("webapp.urlPlaceholder")}
        class="px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
      />
    </label>

    <div class="flex items-center gap-3">
      <div class="w-11 h-11 rounded-lg bg-surface-2 grid place-items-center shrink-0 overflow-hidden border border-border">
        {#if iconPreview}
          <img src={iconPreview} alt="" class="w-full h-full object-contain" />
        {:else}
          <Globe size={18} class="text-text-muted" />
        {/if}
      </div>
      <div class="flex flex-col gap-0.5">
        <button
          class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.78rem] self-start"
          on:click={pickIcon}
        >
          {$_("webapp.selectIcon")}
        </button>
        {#if faviconBusy}
          <span class="text-[0.68rem] text-text-muted">{$_("webapp.downloadingFavicon")}</span>
        {/if}
      </div>
    </div>

    {#if !isEdit}
      <label class="flex flex-col gap-1">
        <span class="text-[0.78rem] text-text-muted">{$_("webapp.category")}</span>
        <select
          bind:value={category}
          class="appearance-none px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
        >
          {#each CATEGORIES as cat}
            <option value={cat}>{$_(`webapp.categories.${cat}`)}</option>
          {/each}
        </select>
      </label>
    {/if}

    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("webapp.browser")}</span>
      {#if browsers.length > 0}
        <select
          bind:value={browserId}
          class="appearance-none px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
        >
          {#each browsers as b}
            <option value={b.id}>{b.displayName}</option>
          {/each}
        </select>
      {:else}
        <p class="text-[0.78rem] text-warning">{$_("webapp.noBrowsers")}</p>
      {/if}
    </label>

    <div class="flex items-center gap-6">
      <label class="flex items-center gap-2 text-[0.82rem] text-text-bright cursor-pointer">
        <input type="checkbox" bind:checked={showNavbar} />
        {$_("webapp.navbar")}
      </label>
      <label class="flex items-center gap-2 text-[0.82rem] text-text-bright cursor-pointer">
        <input type="checkbox" bind:checked={isIncognito} />
        {$_("webapp.incognito")}
      </label>
    </div>

    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("webapp.extraParams")}</span>
      <div class="flex items-center gap-2">
        <input
          type="text"
          bind:value={extraParams}
          placeholder={$_("webapp.extraParamsPlaceholder")}
          class="flex-1 min-w-0 px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
        />
        <button
          class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-primary shrink-0"
          title={$_("webapp.paramsHelpTooltip")}
          on:click={() => (helpOpen = true)}
        >
          <HelpCircle size={17} />
        </button>
      </div>
    </label>
  </div>
</Modal>

{#if helpOpen}
  <Modal variant="info" maxWidth={480} title={$_("webapp.paramsHelpTitle")} onClose={() => (helpOpen = false)}>
    <dl class="flex flex-col gap-2.5 text-[0.8rem]">
      {#each ["startMaximized", "disableFeatures", "forceDarkMode", "userAgent", "proxyServer", "windowSize", "disableNotifications", "shmSize"] as key}
        <div>
          <dt class="font-mono text-primary">
            {#if key === "startMaximized"}--start-maximized
            {:else if key === "disableFeatures"}--disable-features=TabHoverCard
            {:else if key === "forceDarkMode"}--force-dark-mode
            {:else if key === "userAgent"}--user-agent="..."
            {:else if key === "proxyServer"}--proxy-server="IP:PORT"
            {:else if key === "windowSize"}--window-size=WIDTH,HEIGHT
            {:else if key === "disableNotifications"}--disable-notifications
            {:else}--shm-size=2gb{/if}
          </dt>
          <dd class="text-text-muted">{$_(`webapp.paramsHelp.${key}`)}</dd>
        </div>
      {/each}
    </dl>
  </Modal>
{/if}
