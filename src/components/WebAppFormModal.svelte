<script lang="ts">
  import { onMount } from "svelte";
  import { _, locale } from "svelte-i18n";
  import { HelpCircle, Globe, Puzzle, X, Search } from "lucide-svelte";
  import { api } from "../lib/api";
  import Modal from "./Modal.svelte";
  import {
    CATEGORIES,
    CATEGORY_FREEDESKTOP_ID,
    LAUNCH_PARAMS,
    composeExtraParams,
    isChromiumFamily,
    parseExtraParams,
    supportsExtensions,
    type Browser,
    type ExtensionChanges,
    type ExtensionResult,
    type WebApp,
    type WebAppExtension,
    type WebAppInput,
  } from "../lib/webapps";

  // Passing an existing webapp switches this into edit mode. The category
  // selector shows the one the webapp has; the backend keeps it as it is when
  // none is sent, which is what happens until the user picks another one.
  export let webapp: WebApp | null = null;
  export let browsers: Browser[];
  export let onSave: (input: WebAppInput, extensions: ExtensionChanges) => Promise<void>;
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
  // The category the webapp already has, when it is one of the listed ones. A
  // value written by hand in the .desktop file (several categories, or one that
  // is not listed) stays selectable as it is, so editing never replaces it.
  const CURRENT = "current";
  const currentCategory = webapp?.category ?? "";
  const listed = CATEGORIES.find((id) => CATEGORY_FREEDESKTOP_ID[id] === currentCategory);
  const extraCategory = isEdit && currentCategory !== "" && !listed ? currentCategory : null;
  let category: (typeof CATEGORIES)[number] | typeof CURRENT = listed ?? (extraCategory !== null ? CURRENT : "network");
  let categoryChosen = false;
  let browserId = webapp?.browserId ?? browsers[0]?.id ?? "";
  let showNavbar = webapp?.showNavbar ?? false;
  let isIncognito = webapp?.isIncognito ?? false;
  // The launch parameters are one text, in the field, and for the browsers
  // based on Chrome the checkboxes are another way to edit it: ticking a box
  // writes its parameter in the field, and writing one in the field ticks its
  // box. What is not one of the boxes stays in the field as it was written.
  const initialParams = parseExtraParams(webapp?.extraParams ?? "");
  let paramChecked: Record<string, boolean> = { ...initialParams.checked };
  let paramValues: Record<string, string> = { ...initialParams.values };
  let extraParams = webapp?.extraParams ?? "";
  let helpOpen = false;

  function paramsFromText() {
    const parsed = parseExtraParams(extraParams);
    paramChecked = { ...parsed.checked };
    paramValues = { ...parsed.values };
  }

  function paramsToText() {
    extraParams = composeExtraParams(paramChecked, paramValues, parseExtraParams(extraParams).other);
  }
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

  // The extensions: the ones the webapp has, and the ones asked for here. Nothing
  // is downloaded or removed until the form is saved, because a new webapp has
  // no profile to put them in yet.
  type ExtensionItem = { id: string | null; slug: string | null; path: string | null; name: string; isNew: boolean };
  let items: ExtensionItem[] = [];
  let removed: string[] = [];
  let saving = false;

  // The search of extensions, by name, in addons.mozilla.org: the user never
  // has to leave the program to look for one.
  let query = "";
  let searching = false;
  let searched = false;
  let searchFailed = false;
  let results: ExtensionResult[] = [];

  // What is already in the list is not offered again.
  $: found = results.filter((result) => !items.some((item) => item.slug === result.slug));

  async function loadExtensions() {
    if (!webapp) return;
    try {
      const current = await api.invoke<WebAppExtension[]>("list_webapp_extensions", { webappId: webapp.id });
      items = current.map((extension) => ({ id: extension.id, slug: null, path: null, name: extension.name, isNew: false }));
    } catch (error) {
      console.error("Failed to read the extensions", error);
    }
  }

  async function search() {
    if (query.trim() === "" || searching) return;
    searching = true;
    searchFailed = false;
    try {
      results = await api.invoke<ExtensionResult[]>("search_extensions", { query, lang: $locale ?? "en" });
    } catch (error) {
      console.error("The search of extensions failed", error);
      results = [];
      searchFailed = true;
    } finally {
      searching = false;
      searched = true;
    }
  }

  function addResult(result: ExtensionResult) {
    items = [...items, { id: null, slug: result.slug, path: null, name: result.name || result.slug, isNew: true }];
  }

  // For the browsers based on Chrome: the store of the browser, in a normal
  // window with the profile of this webapp.
  let storeError = "";

  async function openStore() {
    if (!webapp) return;
    storeError = "";
    try {
      await api.invoke("open_extension_store", { id: webapp.id });
    } catch (error) {
      console.error("Failed to open the extension store", error);
      storeError = String(error);
    }
  }

  async function addFile() {
    const path = await api.invoke<string | null>("pick_extension_file");
    if (!path) return;
    const name = (path.split("/").pop() ?? path).replace(/\.xpi$/i, "");
    items = [...items, { id: null, slug: null, path, name, isNew: true }];
  }

  function removeItem(item: ExtensionItem) {
    items = items.filter((candidate) => candidate !== item);
    if (!item.isNew && item.id) removed = [...removed, item.id];
  }

  onMount(() => {
    void loadExtensions();
    if (initialUrl) onUrlBlur();
  });

  $: canSave = name.trim() !== "" && url.trim() !== "" && browserId !== "";

  async function submit() {
    if (!canSave || saving) return;
    saving = true;
    // Extensions only apply to Firefox-based browsers: for any other one
    // whatever was chosen is left out.
    const withExtensions = supportsExtensions(browserId);
    const changes: ExtensionChanges = {
      addSlugs: withExtensions ? items.filter((item) => item.isNew && item.slug).map((item) => item.slug as string) : [],
      addFiles: withExtensions ? items.filter((item) => item.isNew && item.path).map((item) => item.path as string) : [],
      remove: withExtensions ? removed : [],
    };
    try {
      await save(changes);
    } finally {
      saving = false;
    }
  }

  async function save(changes: ExtensionChanges) {
    await onSave({
      name: name.trim(),
      url: url.trim(),
      iconPath: iconPath || "applications-internet",
      // An edit that did not touch the selector sends nothing: the backend
      // keeps the categories exactly as they were.
      category: category === CURRENT || (isEdit && !categoryChosen) ? "" : CATEGORY_FREEDESKTOP_ID[category],
      browserId,
      showNavbar,
      extraParams: extraParams.trim(),
      isIncognito,
    }, changes);
  }
</script>

<Modal
  variant="confirm"
  maxWidth={460}
  title={isEdit ? $_("webapp.editTitle") : $_("webapp.addTitle")}
  confirmLabel={$_("common.save")}
  confirmDisabled={!canSave || saving}
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

    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("webapp.category")}</span>
      <select
        bind:value={category}
        on:change={() => (categoryChosen = true)}
        class="appearance-none px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
      >
        {#if extraCategory !== null}
          <option value={CURRENT}>{extraCategory}</option>
        {/if}
        {#each CATEGORIES as cat}
          <option value={cat}>{$_(`webapp.categories.${cat}`)}</option>
        {/each}
      </select>
    </label>

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

    {#if browserId !== ""}
      {#if supportsExtensions(browserId)}
        <div class="flex flex-col gap-1.5">
          <span class="text-[0.78rem] text-text-muted">{$_("webapp.extensions.title")}</span>
          {#if items.length === 0}
            <p class="text-[0.75rem] text-text-muted">{$_("webapp.extensions.none")}</p>
          {/if}
          {#each items as item}
            <div class="flex items-center gap-2 rounded bg-surface-2 border border-border px-2.5 py-1.5">
              <Puzzle size={14} class="text-primary shrink-0" />
              <span class="flex-1 min-w-0 truncate text-[0.82rem] text-text-bright">{item.name}</span>
              <button
                class="p-1 rounded text-text-muted hover:bg-surface hover:text-danger shrink-0"
                title={$_("webapp.extensions.remove")}
                aria-label={$_("webapp.extensions.remove")}
                on:click={() => removeItem(item)}
              >
                <X size={14} />
              </button>
            </div>
          {/each}
          <div class="flex items-center gap-2">
            <input
              type="text"
              bind:value={query}
              on:keydown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  e.stopPropagation();
                  void search();
                }
              }}
              placeholder={$_("webapp.extensions.searchPlaceholder")}
              class="flex-1 min-w-0 px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
            />
            <button
              class="flex items-center gap-1.5 px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.78rem] disabled:opacity-50 shrink-0"
              disabled={query.trim() === "" || searching}
              on:click={search}
            >
              <Search size={13} />
              {searching ? $_("webapp.extensions.searching") : $_("webapp.extensions.search")}
            </button>
          </div>
          {#if searchFailed}
            <p class="text-[0.75rem] text-danger">{$_("webapp.extensions.searchFailed")}</p>
          {:else if searched && !searching && found.length === 0}
            <p class="text-[0.75rem] text-text-muted">{$_("webapp.extensions.noResults")}</p>
          {/if}
          {#if found.length > 0}
            <div class="flex flex-col gap-1 max-h-[210px] overflow-y-auto content-scroll overscroll-contain pr-1">
              {#each found as result (result.slug)}
                <div class="flex items-center gap-2.5 rounded border border-border px-2.5 py-1.5">
                  {#if result.iconUrl}
                    <img src={result.iconUrl} alt="" class="w-7 h-7 rounded object-contain shrink-0" on:error={(e) => (e.currentTarget.style.visibility = "hidden")} />
                  {:else}
                    <Puzzle size={20} class="text-text-muted shrink-0" />
                  {/if}
                  <div class="flex-1 min-w-0">
                    <p class="truncate text-[0.82rem] font-semibold text-text-bright">{result.name}</p>
                    <p class="truncate text-[0.7rem] text-text-muted">{result.summary}</p>
                    {#if result.users > 0}
                      <p class="text-[0.66rem] text-text-muted">{$_("webapp.extensions.users", { values: { count: result.users.toLocaleString($locale ?? "en") } })}</p>
                    {/if}
                  </div>
                  <button
                    class="px-3 py-1.5 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.74rem] font-semibold shrink-0"
                    on:click={() => addResult(result)}
                  >
                    {$_("webapp.extensions.add")}
                  </button>
                </div>
              {/each}
            </div>
          {/if}
          <button
            class="px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.78rem] self-start"
            on:click={addFile}
          >
            {$_("webapp.extensions.addFile")}
          </button>
        </div>
      {:else if isChromiumFamily(browserId)}
        <div class="flex flex-col gap-1.5">
          <span class="text-[0.78rem] text-text-muted">{$_("webapp.extensions.title")}</span>
          {#if isEdit}
            <button
              class="flex items-center gap-1.5 px-3 py-1.5 rounded border border-border text-text hover:bg-surface-2 hover:text-text-bright text-[0.78rem] self-start disabled:opacity-50"
              disabled={webapp?.browserId !== browserId}
              on:click={openStore}
            >
              <Puzzle size={13} />
              {$_("webapp.extensions.openStore")}
            </button>
            {#if webapp?.browserId !== browserId}
              <p class="text-[0.72rem] text-text-muted">{$_("webapp.extensions.storeSaveFirst")}</p>
            {:else}
              <p class="text-[0.72rem] text-text-muted">{$_("webapp.extensions.storeNote")}</p>
            {/if}
            {#if storeError}
              <p class="text-[0.75rem] text-danger break-words">{$_("webapp.extensions.storeFailed")} {storeError}</p>
            {/if}
          {:else}
            <p class="text-[0.72rem] text-text-muted">{$_("webapp.extensions.storeSaveFirst")}</p>
          {/if}
        </div>
      {:else}
        <p class="text-[0.72rem] text-text-muted">{$_("webapp.extensions.unsupported")}</p>
      {/if}
    {/if}

    {#if saving}
      <p class="text-[0.75rem] text-primary">{$_("webapp.extensions.saving")}</p>
    {/if}

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

    {#if isChromiumFamily(browserId)}
      <div class="flex flex-col gap-1.5">
        <span class="text-[0.78rem] text-text-muted">{$_("webapp.extraParams")}</span>
        <div class="flex flex-col gap-2 rounded border border-border bg-surface-2/40 px-3 py-2.5">
          {#each LAUNCH_PARAMS as param (param.key)}
            <div class="flex flex-col gap-1">
              <label class="flex items-start gap-2 cursor-pointer">
                <input type="checkbox" class="mt-[3px]" bind:checked={paramChecked[param.key]} on:change={paramsToText} />
                <span class="flex-1 min-w-0">
                  <span class="block font-mono text-[0.76rem] text-primary break-all">{param.flag}{param.value ? "…" : ""}</span>
                  <span class="block text-[0.7rem] text-text-muted">{$_(`webapp.paramsHelp.${param.key}`)}</span>
                </span>
              </label>
              {#if param.value && paramChecked[param.key]}
                <input
                  type="text"
                  bind:value={paramValues[param.key]}
                  on:input={paramsToText}
                  placeholder={param.value.placeholder}
                  class="ml-6 px-2.5 py-1 rounded bg-surface-2 border border-border text-text-bright text-[0.8rem] font-mono focus:outline-none focus:border-primary"
                />
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("webapp.extraParams")}</span>
      <div class="flex items-center gap-2">
        <input
          type="text"
          bind:value={extraParams}
          on:input={paramsFromText}
          placeholder={$_("webapp.extraParamsPlaceholder")}
          class="flex-1 min-w-0 px-3 py-1.5 rounded bg-surface-2 border border-border text-text-bright text-[0.85rem] focus:outline-none focus:border-primary"
        />
        {#if !isChromiumFamily(browserId)}
          <button
            class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-primary shrink-0"
            title={$_("webapp.paramsHelpTooltip")}
            on:click={() => (helpOpen = true)}
          >
            <HelpCircle size={17} />
          </button>
        {/if}
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
