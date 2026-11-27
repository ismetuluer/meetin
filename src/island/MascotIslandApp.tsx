import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { t, useLanguage } from "../i18n";
import { MeetinSheep } from "../mascot/MeetinSheep";
import {
  ANIM_DURATIONS,
  computePupilOffset,
  pickAutoAnim,
  playBaaSound,
  playThrowSound,
  playWallImpactSound,
  type MascotAnim,
} from "../mascot/mascotMachine";
import { createShakeDetector } from "../mascot/shakeDetector";
import {
  checkBasketScore,
  classifyThrowSpeed,
  createVelocityTracker,
  stepThrowPhysics,
  type HoopRimRect,
  type MonitorBounds,
  type ThrowState,
} from "../mascot/throwPhysics";
import type {
  AbbrSuggestMode,
  AppSettingsDto,
  EngineStatus,
  FolderRecord,
  PhraseRecord,
  SuggestionItem,
} from "../studio/types";
import { normalizeTurkishAbbr, suggestAbbreviation } from "./abbrSuggest";
import { createPhraseSearchIndex } from "./searchEngine";
import "./island.css";

type IslandMode = "mascot" | "pill" | "suggest" | "search" | "add" | "fill" | "menu";

interface FormFieldDto {
  name: string;
  options: string[] | null;
}

interface FillIslandPayload {
  abbr: string;
  fields: FormFieldDto[];
  preview: string;
}

interface PillNotice {
  ok: boolean;
  text: string;
}

export function MascotIslandApp() {
  useLanguage();
  const [mode, setMode] = useState<IslandMode>("mascot");
  const [anim, setAnim] = useState<MascotAnim>("idle");
  const [paused, setPaused] = useState(false);
  const [isVisible, setIsVisible] = useState(true);
  const [isAdminWindow, setIsAdminWindow] = useState(false);
  const [soundEnabled, setSoundEnabled] = useState(true);
  const [soundBaa, setSoundBaa] = useState(false);
  const [soundThrow, setSoundThrow] = useState(true);
  const [soundImpact, setSoundImpact] = useState(true);
  const [mascotSize, setMascotSize] = useState<"small" | "normal" | "large">("normal");
  const [pupilOffset, setPupilOffset] = useState({ x: 0, y: 0 });
  const [flightAngleDeg, setFlightAngleDeg] = useState(0);
  const [basketballMode, setBasketballMode] = useState(false);
  const [pillNotice, setPillNotice] = useState<PillNotice | null>(null);
  const [suggestions, setSuggestions] = useState<SuggestionItem[]>([]);
  const modeRef = useRef<IslandMode>("mascot");
  useEffect(() => {
    modeRef.current = mode;
  }, [mode]);

  // Arama adası durumu
  const [phrases, setPhrases] = useState<PhraseRecord[]>([]);
  const [folders, setFolders] = useState<FolderRecord[]>([]);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isDraggingPanel, setIsDraggingPanel] = useState(false);

  // Ekleme adası durumu (§4.4)
  const [abbrSuggestMode, setAbbrSuggestMode] = useState<AbbrSuggestMode>("initials");
  const [addAbbr, setAddAbbr] = useState("");
  const [addAbbrManual, setAddAbbrManual] = useState(false);
  const [addContent, setAddContent] = useState("");
  const [addDescription, setAddDescription] = useState("");
  const [addDescriptionManual, setAddDescriptionManual] = useState(false);
  const [addFolderId, setAddFolderId] = useState<number | null>(() => {
    try {
      const saved = window.localStorage.getItem("meetin:last_folder_id");
      return saved ? Number(saved) : null;
    } catch {
      return null;
    }
  });
  const [showInlineNewFolder, setShowInlineNewFolder] = useState(false);
  const [inlineFolderName, setInlineFolderName] = useState("");
  const [addTagsText, setAddTagsText] = useState("");
  const [addError, setAddError] = useState<string | null>(null);

  // Alan doldurma adası durumu (§3.3, §5.2)
  const [fillPayload, setFillPayload] = useState<FillIslandPayload | null>(null);
  const [fillValues, setFillValues] = useState<Record<string, string>>({});

  const svgRef = useRef<SVGSVGElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const addAbbrInputRef = useRef<HTMLInputElement | null>(null);
  const fillFirstInputRef = useRef<HTMLInputElement | HTMLSelectElement | null>(null);
  const lastUsedFolderIdRef = useRef<number | null>(addFolderId);
  const animTimerRef = useRef<number | null>(null);
  const pillTimerRef = useRef<number | null>(null);
  const lastUserRef = useRef<number>(Date.now());
  const isDraggingRef = useRef(false);
  const shakeDetectorRef = useRef(createShakeDetector());
  const velocityTrackerRef = useRef(createVelocityTracker());
  const throwRafRef = useRef<number | null>(null);
  const basketStatsRef = useRef({ score: 0, streak: 0 });
  const basketballModeRef = useRef(basketballMode);
  basketballModeRef.current = basketballMode;
  const pausedRef = useRef(paused);
  pausedRef.current = paused;
  const soundEnabledRef = useRef(soundEnabled);
  soundEnabledRef.current = soundEnabled;
  const soundBaaRef = useRef(soundBaa);
  soundBaaRef.current = soundBaa;
  const soundThrowRef = useRef(soundThrow);
  soundThrowRef.current = soundThrow;
  const soundImpactRef = useRef(soundImpact);
  soundImpactRef.current = soundImpact;
  const lastImpactSoundRef = useRef(0);

  // İşaretçi hareket / basılı tutma / tıklama ayrımı (maskot)
  const pointerStateRef = useRef<{
    down: boolean;
    pointerId: number | null;
    startClientX: number;
    startClientY: number;
    startScreenX: number;
    startScreenY: number;
    startWinX: number;
    startWinY: number;
    hasWinPos: boolean;
    lastSentX: number;
    lastSentY: number;
    dragStarted: boolean;
    heldLong: boolean;
    holdTimer: number | null;
    clickTimer: number | null;
  }>({
    down: false,
    pointerId: null,
    startClientX: 0,
    startClientY: 0,
    startScreenX: 0,
    startScreenY: 0,
    startWinX: 0,
    startWinY: 0,
    hasWinPos: false,
    lastSentX: 0,
    lastSentY: 0,
    dragStarted: false,
    heldLong: false,
    holdTimer: null,
    clickTimer: null,
  });

  // Arama adası üst çubuk sürükleme durumu
  const panelDragRef = useRef<{
    down: boolean;
    pointerId: number | null;
    startScreenX: number;
    startScreenY: number;
    startWinX: number;
    startWinY: number;
    hasWinPos: boolean;
    lastSentX: number;
    lastSentY: number;
    dragStarted: boolean;
  }>({
    down: false,
    pointerId: null,
    startScreenX: 0,
    startScreenY: 0,
    startWinX: 0,
    startWinY: 0,
    hasWinPos: false,
    lastSentX: 0,
    lastSentY: 0,
    dragStarted: false,
  });

  const clearAnimTimer = useCallback(() => {
    if (animTimerRef.current !== null) {
      window.clearTimeout(animTimerRef.current);
      animTimerRef.current = null;
    }
  }, []);

  const cancelThrow = useCallback(() => {
    if (throwRafRef.current !== null) {
      window.cancelAnimationFrame(throwRafRef.current);
      throwRafRef.current = null;
    }
  }, []);

  const playAnim = useCallback(
    (next: MascotAnim, allowBaaSound = true) => {
      clearAnimTimer();
      setAnim(next);
      if (next === "baa" && allowBaaSound && soundEnabledRef.current && soundBaaRef.current) {
        playBaaSound();
      }
      const dur = ANIM_DURATIONS[next];
      if (dur !== null) {
        animTimerRef.current = window.setTimeout(() => {
          setAnim(pausedRef.current ? "sleep" : "idle");
        }, dur);
      }
    },
    [clearAnimTimer],
  );

  const showNoticePill = useCallback(
    (notice: PillNotice, nextAnim: MascotAnim) => {
      if (pillTimerRef.current !== null) {
        window.clearTimeout(pillTimerRef.current);
      }
      setPillNotice(notice);
      setMode("pill");
      playAnim(nextAnim);
      void invoke("set_mascot_mode", { mode: "pill" });

      pillTimerRef.current = window.setTimeout(() => {
        setMode("mascot");
        setPillNotice(null);
        void invoke("set_mascot_mode", { mode: "mascot" });
      }, 1200);
    },
    [playAnim],
  );

  const loadPhrases = useCallback(async () => {
    try {
      const [list, folderList] = await Promise.all([
        invoke<PhraseRecord[]>("list_phrases", { includeDeleted: false }),
        invoke<FolderRecord[]>("list_folders"),
      ]);
      setPhrases(list);
      setFolders(folderList);
    } catch {
      // Sessizce yok say
    }
  }, []);

  const openSearch = useCallback(async () => {
    cancelThrow();
    lastUserRef.current = Date.now();
    await loadPhrases();
    setQuery("");
    setSelectedIndex(0);
    setMode("search");
    await invoke("set_mascot_mode", { mode: "search" });
    window.setTimeout(() => searchInputRef.current?.focus(), 40);
  }, [loadPhrases]);

  const openAddIsland = useCallback(
    async (initialText: string) => {
      cancelThrow();
      lastUserRef.current = Date.now();
      const [settingsRes] = await Promise.all([
        invoke<AppSettingsDto>("get_app_settings").catch(() => null),
        loadPhrases(),
      ]);
      const effMode = settingsRes?.abbr_suggest_mode ?? abbrSuggestMode;
      if (settingsRes?.abbr_suggest_mode) {
        setAbbrSuggestMode(settingsRes.abbr_suggest_mode);
      }
      const cleaned = initialText.trim();
      const firstLine =
        cleaned
          .split(/\r?\n/)
          .map((l) => l.trim())
          .find(Boolean) ?? "";
      const initialFid = lastUsedFolderIdRef.current;
      const initialFolderName =
        initialFid !== null
          ? folders.find((f) => f.id === initialFid)?.name ?? ""
          : "";
      setAddContent(initialText);
      setAddAbbr(suggestAbbreviation(cleaned, effMode, initialFolderName ? [initialFolderName] : []));
      setAddAbbrManual(false);
      setAddDescription(firstLine.slice(0, 80));
      setAddDescriptionManual(false);
      setAddFolderId(initialFid);
      setShowInlineNewFolder(false);
      setInlineFolderName("");
      setAddTagsText(initialFolderName);
      setAddError(null);
      setMode("add");
      await invoke("set_mascot_mode", { mode: "add" });
      window.setTimeout(() => {
        addAbbrInputRef.current?.focus();
        addAbbrInputRef.current?.select();
      }, 45);
    },
    [abbrSuggestMode, loadPhrases],
  );

  const closeToMascot = useCallback(async () => {
    if (mode === "fill") {
      await invoke("cancel_fill_island");
      setFillPayload(null);
    }
    setMode("mascot");
    await invoke("set_mascot_mode", { mode: "mascot" });
  }, [mode]);

  // İlk durum yükleme ve Tauri olay dinleyicileri
  useEffect(() => {
    void invoke<EngineStatus>("get_engine_status").then((st) => {
      setPaused(st.paused);
      if (st.paused) setAnim("sleep");
    });
    void invoke<AppSettingsDto>("get_app_settings")
      .then((s) => {
        setSoundEnabled(s.mascot_sound);
        setSoundBaa(s.sound_baa ?? false);
        setSoundThrow(s.sound_throw ?? true);
        setSoundImpact(s.sound_impact ?? true);
        setMascotSize(s.mascot_size);
        setAbbrSuggestMode(s.abbr_suggest_mode ?? "initials");
      })
      .catch(() => {});
    void loadPhrases();

    const unlisteners = [
      listen<{
        sound: boolean;
        size: "small" | "normal" | "large";
        sound_baa?: boolean;
        sound_throw?: boolean;
        sound_impact?: boolean;
      }>("mascot-prefs-changed", (ev) => {
        setSoundEnabled(ev.payload.sound);
        setMascotSize(ev.payload.size);
        if (typeof ev.payload.sound_baa === "boolean") {
          setSoundBaa(ev.payload.sound_baa);
        }
        if (typeof ev.payload.sound_throw === "boolean") {
          setSoundThrow(ev.payload.sound_throw);
        }
        if (typeof ev.payload.sound_impact === "boolean") {
          setSoundImpact(ev.payload.sound_impact);
        }
      }),
      listen<void>("onboarding-wake-mascot", () => {
        playAnim("hop");
      }),
      listen<string>("phrase-expanded", () => {
        playAnim("thumbsup");
      }),
      listen<string>("phrase-created", () => {
        playAnim("thumbsup");
      }),
      listen<string>("engine-error", (ev) => {
        showNoticePill(
          { ok: false, text: ev.payload || t("island.notFound") },
          "sad",
        );
      }),
      listen<boolean>("engine-status-changed", (ev) => {
        setPaused(ev.payload);
        setAnim(ev.payload ? "sleep" : "idle");
      }),
      listen<boolean>("admin-window-changed", (ev) => {
        setIsAdminWindow(ev.payload);
      }),
      listen<boolean>("mascot-visibility-changed", (ev) => {
        setIsVisible(ev.payload);
      }),
      listen<void>("open-search-island", () => {
        void openSearch();
      }),
      listen<string>("open-add-island", (ev) => {
        void openAddIsland(ev.payload ?? "");
      }),
      listen<FillIslandPayload>("open-fill-island", (ev) => {
        cancelThrow();
        lastUserRef.current = Date.now();
        const payload = ev.payload;
        const initialVals: Record<string, string> = {};
        for (const f of payload.fields) {
          initialVals[f.name] = f.options?.[0] ?? "";
        }
        setFillPayload(payload);
        setFillValues(initialVals);
        setMode("fill");
        window.setTimeout(() => fillFirstInputRef.current?.focus(), 45);
      }),
      listen<number>("file-imported", (ev) => {
        showNoticePill(
          { ok: true, text: t("island.fileImportedOk", { count: ev.payload }) },
          "thumbsup",
        );
      }),
      listen<boolean>("basketball-mode-changed", (ev) => {
        setBasketballMode(ev.payload);
        if (!ev.payload) {
          basketStatsRef.current = { score: 0, streak: 0 };
        }
      }),
      listen<SuggestionItem[]>("typing-suggestions", (ev) => {
        const items = ev.payload ?? [];
        const curMode = modeRef.current;
        if (items.length > 0 && (curMode === "mascot" || curMode === "suggest")) {
          setSuggestions(items);
          if (curMode !== "suggest") {
            setMode("suggest");
            void invoke("set_mascot_mode", { mode: "suggest" });
          }
        } else if (items.length === 0 && curMode === "suggest") {
          setSuggestions([]);
          setMode("mascot");
          void invoke("set_mascot_mode", { mode: "mascot" });
        }
      }),
    ];
    return () => {
      cancelThrow();
      clearAnimTimer();
      if (pillTimerRef.current !== null) {
        window.clearTimeout(pillTimerRef.current);
      }
      for (const u of unlisteners) {
        void u.then((f) => f());
      }
    };
  }, [clearAnimTimer, loadPhrases, openAddIsland, openSearch, showNoticePill]);

  // "Kendi hâlinde" modu: 5 sn etkileşim yoksa 6 sn'de bir rastgele animasyon (§5.1.1)
  useEffect(() => {
    const id = window.setInterval(() => {
      if (
        !isVisible ||
        paused ||
        mode !== "mascot" ||
        anim !== "idle" ||
        Date.now() - lastUserRef.current < 5000
      ) {
        return;
      }
      playAnim(pickAutoAnim(), false);
    }, 6000);
    return () => window.clearInterval(id);
  }, [anim, isVisible, mode, paused, playAnim]);

  // Göz bebeği takibi (§5.1.1)
  useEffect(() => {
    function onPointerMove(e: PointerEvent) {
      if (paused || anim === "sleep" || !isVisible || !svgRef.current) return;
      const rect = svgRef.current.getBoundingClientRect();
      const cx = rect.left + rect.width * 0.5;
      const cy = rect.top + rect.height * 0.53;
      setPupilOffset(computePupilOffset(e.clientX - cx, e.clientY - cy));
    }
    window.addEventListener("pointermove", onPointerMove);
    return () => window.removeEventListener("pointermove", onPointerMove);
  }, [anim, isVisible, paused]);

  // Ada/menü açıkken odak nerede olursa olsun Esc ile kapat + odak kaybında kapat
  useEffect(() => {
    if (mode !== "search" && mode !== "add" && mode !== "fill" && mode !== "menu") return;

    function onWindowKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        void closeToMascot();
      }
    }
    function onWindowBlur() {
      if (isDraggingRef.current) return;
      void closeToMascot();
    }

    window.addEventListener("keydown", onWindowKeyDown);
    window.addEventListener("blur", onWindowBlur);
    return () => {
      window.removeEventListener("keydown", onWindowKeyDown);
      window.removeEventListener("blur", onWindowBlur);
    };
  }, [closeToMascot, mode]);

  // Bulanık arama indeksi
  const searchIndex = useMemo(
    () => createPhraseSearchIndex(phrases),
    [phrases],
  );

  const results = useMemo(
    () => searchIndex.search(query, 30),
    [searchIndex, query],
  );

  const selectedPhrase = results[selectedIndex] ?? null;

  const conflictingPhrase = useMemo(() => {
    const norm = normalizeTurkishAbbr(addAbbr);
    if (!norm) return null;
    return phrases.find((p) => normalizeTurkishAbbr(p.abbr) === norm) ?? null;
  }, [addAbbr, phrases]);

  async function handleFillFromClipboard() {
    try {
      const clip = await invoke<string | null>("read_clipboard_text");
      if (clip) {
        setAddContent(clip);
        if (!addAbbrManual || !addAbbr.trim()) {
          const tags = addTagsText
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean);
          setAddAbbr(suggestAbbreviation(clip, abbrSuggestMode, tags));
        }
        if (!addDescriptionManual || !addDescription.trim()) {
          const line =
            clip
              .split(/\r?\n/)
              .map((l) => l.trim())
              .find(Boolean) ?? "";
          setAddDescription(line.slice(0, 80));
        }
        setAddError(null);
      } else {
        setAddError(t("add.clipboardEmpty"));
      }
    } catch (err) {
      setAddError(String(err));
    }
  }

  async function handleCreateInlineFolder() {
    const trimmed = inlineFolderName.trim();
    if (!trimmed) return;
    try {
      const created = await invoke<FolderRecord>("create_folder", {
        name: trimmed,
        parentId: null,
        color: null,
      });
      setFolders((prev) => [...prev, created]);
      setAddFolderId(created.id);
      lastUsedFolderIdRef.current = created.id;
      setAddTagsText((prev) => {
        const list = prev.split(",").map((s) => s.trim()).filter(Boolean);
        return list.includes(created.name) ? prev : [...list, created.name].join(", ");
      });
      try {
        window.localStorage.setItem("meetin:last_folder_id", String(created.id));
      } catch {
        // yok say
      }
      setInlineFolderName("");
      setShowInlineNewFolder(false);
    } catch (err) {
      setAddError(String(err));
    }
  }

  async function handleSaveAdd() {
    const abbr = addAbbr.trim();
    if (!abbr || !addContent.trim()) {
      setAddError(t("error.requiredFields"));
      return;
    }
    const tags = addTagsText
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);

    try {
      await invoke<PhraseRecord>("create_phrase", {
        input: {
          abbr,
          content: addContent,
          content_html: null,
          description: addDescription.trim(),
          folder_id: addFolderId,
          folder_ids: addFolderId !== null ? [addFolderId] : [],
          case_mode: "keep_original",
          shortcut: null,
          tags,
        },
      });
      lastUsedFolderIdRef.current = addFolderId;
      try {
        if (addFolderId !== null) {
          window.localStorage.setItem("meetin:last_folder_id", String(addFolderId));
        } else {
          window.localStorage.removeItem("meetin:last_folder_id");
        }
      } catch {
        // yok say
      }
      await loadPhrases();
      await closeToMascot();
      playAnim("thumbsup");
    } catch (err) {
      setAddError(t("error.generic", { message: String(err) }));
    }
  }

  async function handleDetailedEdit() {
    await invoke("open_studio_with_draft", {
      abbr: addAbbr.trim(),
      content: addContent,
      description: addDescription.trim(),
    });
    await closeToMascot();
  }

  const confirmSelection = useCallback(
    async (phrase: PhraseRecord | null) => {
      if (!phrase) {
        playAnim("sad");
        return;
      }
      try {
        await invoke("paste_phrase_from_island", { phraseId: phrase.id });
      } catch (err) {
        showNoticePill(
          { ok: false, text: String(err) },
          "sad",
        );
      }
    },
    [playAnim, showNoticePill],
  );

  function handleSearchKeyDown(e: ReactKeyboardEvent<HTMLInputElement>) {
    if (e.key === "Escape") {
      e.preventDefault();
      void closeToMascot();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) =>
        results.length === 0 ? 0 : Math.min(i + 1, results.length - 1),
      );
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      void confirmSelection(selectedPhrase);
    }
  }

  // Maskot fare etkileşimleri: tek tık / çift tık / basılı tut / sürükle-fırlat / salla / sağ tık (§5.1)
  function startThrowLoop(initial: ThrowState) {
    cancelThrow();
    const isBasket = basketballModeRef.current;
    const gravity = isBasket ? 1150 : 0;

    clearAnimTimer();
    setAnim("super");
    if (soundEnabledRef.current && soundThrowRef.current) {
      playThrowSound();
    }
    setFlightAngleDeg((Math.atan2(initial.vy, initial.vx) * 180) / Math.PI);

    void Promise.all([
      invoke<{
        min_x: number;
        min_y: number;
        max_x: number;
        max_y: number;
      }>("get_mascot_monitor_bounds"),
      isBasket
        ? invoke<HoopRimRect | null>("get_hoop_rim_rect")
        : Promise.resolve(null),
    ])
      .then(([dto, initialRim]) => {
        const bounds: MonitorBounds = {
          minX: dto.min_x,
          minY: dto.min_y,
          maxX: dto.max_x,
          maxY: dto.max_y,
        };
        const rim = initialRim;
        const dpr = window.devicePixelRatio || 1;
        const halfW = 50 * dpr;
        let sim = initial;
        let lastTime = performance.now();
        let lastSavedX = Math.round(sim.x);
        let lastSavedY = Math.round(sim.y);
        let scoredThisThrow = false;

        const tick = (now: number) => {
          const dtSec = (now - lastTime) / 1000;
          lastTime = now;

          const prevCx = sim.x + halfW;
          const prevCy = sim.y + halfW;
          const step = stepThrowPhysics(sim, bounds, dtSec, { gravity });
          sim = step.state;
          if (step.bounced && soundEnabledRef.current && soundImpactRef.current) {
            const sinceLast = now - lastImpactSoundRef.current;
            if (sinceLast >= 90) {
              lastImpactSoundRef.current = now;
              playWallImpactSound(step.impactSpeed);
            }
          }
          const nextCx = sim.x + halfW;
          const nextCy = sim.y + halfW;

          const rx = Math.round(sim.x);
          const ry = Math.round(sim.y);
          if (rx !== lastSavedX || ry !== lastSavedY) {
            lastSavedX = rx;
            lastSavedY = ry;
            void invoke("move_mascot_window", { x: rx, y: ry });
          }

          if (!scoredThisThrow) {
            setFlightAngleDeg((Math.atan2(sim.vy, sim.vx) * 180) / Math.PI);
          }

          if (
            isBasket &&
            rim &&
            !scoredThisThrow &&
            checkBasketScore(prevCx, prevCy, nextCx, nextCy, rim)
          ) {
            scoredThisThrow = true;
            basketStatsRef.current.score += 1;
            basketStatsRef.current.streak += 1;
            const { score, streak } = basketStatsRef.current;
            void invoke("notify_basket_scored", { score, streak });
            playAnim("thumbsup");
            window.setTimeout(() => {
              void invoke("reposition_hoop_random");
            }, 850);
          }

          if (step.stopped) {
            throwRafRef.current = null;
            if (isBasket && !scoredThisThrow) {
              basketStatsRef.current.streak = 0;
            }
            if (!scoredThisThrow) {
              setAnim(pausedRef.current ? "sleep" : "idle");
            }
            void invoke("save_mascot_position", {
              x: lastSavedX,
              y: lastSavedY,
            });
            return;
          }

          throwRafRef.current = window.requestAnimationFrame(tick);
        };

        throwRafRef.current = window.requestAnimationFrame(tick);
      })
      .catch(() => {
        setAnim(pausedRef.current ? "sleep" : "idle");
        void invoke("save_mascot_position", {
          x: Math.round(initial.x),
          y: Math.round(initial.y),
        });
      });
  }

  function handleMascotPointerDown(e: ReactPointerEvent<HTMLButtonElement>) {
    if (e.button !== 0) return;
    cancelThrow();
    lastUserRef.current = Date.now();
    const ps = pointerStateRef.current;
    ps.down = true;
    ps.pointerId = e.pointerId;
    ps.startClientX = e.clientX;
    ps.startClientY = e.clientY;
    ps.startScreenX = e.screenX;
    ps.startScreenY = e.screenY;
    ps.hasWinPos = false;
    ps.dragStarted = false;
    ps.heldLong = false;
    shakeDetectorRef.current.reset();
    velocityTrackerRef.current.reset();

    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Yok say
    }

    void getCurrentWebviewWindow()
      .outerPosition()
      .then((pos) => {
        ps.startWinX = pos.x;
        ps.startWinY = pos.y;
        ps.lastSentX = pos.x;
        ps.lastSentY = pos.y;
        ps.hasWinPos = true;
        velocityTrackerRef.current.sample(pos.x, pos.y, performance.now());
      })
      .catch(() => {});

    clearAnimTimer();
    setAnim("squeeze");

    if (ps.holdTimer !== null) window.clearTimeout(ps.holdTimer);
    ps.holdTimer = window.setTimeout(() => {
      ps.heldLong = true;
    }, 450);
  }

  function handleMascotPointerMove(e: ReactPointerEvent<HTMLButtonElement>) {
    const ps = pointerStateRef.current;
    if (!ps.down) return;

    if (!ps.dragStarted) {
      const dist = Math.hypot(
        e.clientX - ps.startClientX,
        e.clientY - ps.startClientY,
      );
      if (dist <= 5) return;

      ps.dragStarted = true;
      isDraggingRef.current = true;
      if (ps.holdTimer !== null) {
        window.clearTimeout(ps.holdTimer);
        ps.holdTimer = null;
      }
      clearAnimTimer();
      setAnim("super");
    }

    if (!ps.hasWinPos) return;
    const now = performance.now();
    const dpr = window.devicePixelRatio || 1;
    const nextX = Math.round(ps.startWinX + (e.screenX - ps.startScreenX) * dpr);
    const nextY = Math.round(ps.startWinY + (e.screenY - ps.startScreenY) * dpr);
    const moveDx = nextX - ps.lastSentX;
    const moveDy = nextY - ps.lastSentY;
    if (moveDx !== 0 || moveDy !== 0) {
      if (Math.hypot(moveDx, moveDy) >= 2) {
        setFlightAngleDeg((Math.atan2(moveDy, moveDx) * 180) / Math.PI);
      }
      ps.lastSentX = nextX;
      ps.lastSentY = nextY;
      velocityTrackerRef.current.sample(nextX, nextY, now);
      void invoke("move_mascot_window", { x: nextX, y: nextY });
    }

    if (shakeDetectorRef.current.push(e.screenX, e.screenY, now)) {
      velocityTrackerRef.current.reset();
      playAnim("hop");
      void invoke<boolean>("toggle_studio_window");
    }
  }

  function finishMascotInteraction(
    target: HTMLButtonElement | null,
    treatAsClick: boolean,
  ) {
    const ps = pointerStateRef.current;
    if (!ps.down) return;
    ps.down = false;

    if (target && ps.pointerId !== null) {
      try {
        if (target.hasPointerCapture(ps.pointerId)) {
          target.releasePointerCapture(ps.pointerId);
        }
      } catch {
        // Yok say
      }
    }
    ps.pointerId = null;

    if (ps.holdTimer !== null) {
      window.clearTimeout(ps.holdTimer);
      ps.holdTimer = null;
    }

    if (ps.dragStarted) {
      ps.dragStarted = false;
      isDraggingRef.current = false;
      shakeDetectorRef.current.reset();

      const vel = velocityTrackerRef.current.getVelocity(performance.now());
      velocityTrackerRef.current.reset();
      const tier = treatAsClick ? classifyThrowSpeed(vel.speed) : "none";

      if (ps.hasWinPos && (tier !== "none" || basketballModeRef.current)) {
        const launchVx =
          tier === "none" ? vel.vx * 1.15 : vel.vx;
        const launchVy =
          tier === "none" ? vel.vy * 1.15 : vel.vy;
        if (Math.hypot(launchVx, launchVy) >= 140) {
          startThrowLoop({
            x: ps.lastSentX,
            y: ps.lastSentY,
            vx: launchVx,
            vy: launchVy,
          });
          return;
        }
      }

      if (ps.hasWinPos) {
        void invoke("save_mascot_position", {
          x: ps.lastSentX,
          y: ps.lastSentY,
        });
      }
      setAnim(paused ? "sleep" : "idle");
      return;
    }

    if (!treatAsClick) {
      setAnim(paused ? "sleep" : "idle");
      return;
    }

    if (ps.heldLong) {
      ps.heldLong = false;
      setAnim(paused ? "sleep" : "idle");
      void invoke("open_add_from_squeeze");
      return;
    }

    setAnim(paused ? "sleep" : "idle");

    // Tek tık ile çift tık ayrımı
    if (ps.clickTimer !== null) {
      window.clearTimeout(ps.clickTimer);
      ps.clickTimer = null;
      void invoke("open_studio_window");
    } else {
      ps.clickTimer = window.setTimeout(() => {
        ps.clickTimer = null;
        void openSearch();
      }, 220);
    }
  }

  function handleMascotPointerUp(e: ReactPointerEvent<HTMLButtonElement>) {
    if (e.button !== 0) return;
    finishMascotInteraction(e.currentTarget, true);
  }

  function handleMascotPointerCancel(e: ReactPointerEvent<HTMLButtonElement>) {
    finishMascotInteraction(e.currentTarget, false);
  }

  // Arama adası üst çubuğundan serbestçe taşıma
  function handlePanelPointerDown(e: ReactPointerEvent<HTMLDivElement>) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest("input, button, select, textarea")) {
      return;
    }

    const pd = panelDragRef.current;
    pd.down = true;
    pd.pointerId = e.pointerId;
    pd.startScreenX = e.screenX;
    pd.startScreenY = e.screenY;
    pd.hasWinPos = false;
    pd.dragStarted = false;

    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Yok say
    }

    void getCurrentWebviewWindow()
      .outerPosition()
      .then((pos) => {
        pd.startWinX = pos.x;
        pd.startWinY = pos.y;
        pd.lastSentX = pos.x;
        pd.lastSentY = pos.y;
        pd.hasWinPos = true;
      })
      .catch(() => {});
  }

  function handlePanelPointerMove(e: ReactPointerEvent<HTMLDivElement>) {
    const pd = panelDragRef.current;
    if (!pd.down || !pd.hasWinPos) return;
    const dx = e.screenX - pd.startScreenX;
    const dy = e.screenY - pd.startScreenY;
    if (!pd.dragStarted && Math.hypot(dx, dy) > 3) {
      pd.dragStarted = true;
      isDraggingRef.current = true;
      setIsDraggingPanel(true);
    }
    if (!pd.dragStarted) return;

    const dpr = window.devicePixelRatio || 1;
    const nextX = Math.round(pd.startWinX + dx * dpr);
    const nextY = Math.round(pd.startWinY + dy * dpr);
    if (nextX !== pd.lastSentX || nextY !== pd.lastSentY) {
      pd.lastSentX = nextX;
      pd.lastSentY = nextY;
      void invoke("move_mascot_window", { x: nextX, y: nextY });
    }
  }

  function handlePanelPointerUp(e: ReactPointerEvent<HTMLDivElement>) {
    const pd = panelDragRef.current;
    if (!pd.down) return;
    pd.down = false;
    if (pd.pointerId !== null) {
      try {
        if (e.currentTarget.hasPointerCapture(pd.pointerId)) {
          e.currentTarget.releasePointerCapture(pd.pointerId);
        }
      } catch {
        // Yok say
      }
    }
    pd.pointerId = null;

    if (pd.dragStarted) {
      pd.dragStarted = false;
      isDraggingRef.current = false;
      setIsDraggingPanel(false);
      if (pd.hasWinPos) {
        void invoke("save_mascot_position", {
          x: pd.lastSentX,
          y: pd.lastSentY,
        });
      }
    }
  }

  function handleMascotContextMenu(e: ReactMouseEvent) {
    e.preventDefault();
    lastUserRef.current = Date.now();
    setMode("menu");
    void invoke("set_mascot_mode", { mode: "menu" });
  }

  if (mode === "menu") {
    return (
      <div
        className="mascot-menu"
        onKeyDown={(e) => {
          if (e.key === "Escape") void closeToMascot();
        }}
      >
        <button
          type="button"
          className="mascot-menu-item"
          onClick={async () => {
            await invoke("toggle_pause");
            await closeToMascot();
          }}
        >
          {paused ? t("status.resume") : t("status.pause")}
        </button>
        <button
          type="button"
          className="mascot-menu-item"
          onClick={async () => {
            await invoke("open_studio_window");
            await closeToMascot();
          }}
        >
          {t("mascot.menuStudio")}
        </button>
        <button
          type="button"
          className="mascot-menu-item"
          onClick={async () => {
            const nextSound = !soundEnabled;
            setSoundEnabled(nextSound);
            await invoke("set_mascot_prefs", {
              sound: nextSound,
              size: mascotSize,
              soundBaa,
              soundThrow,
              soundImpact,
            });
            await closeToMascot();
          }}
        >
          {soundEnabled ? t("mascot.menuMute") : t("mascot.menuUnmute")}
        </button>
        <button
          type="button"
          className="mascot-menu-item"
          onClick={async () => {
            await invoke("toggle_basketball_game");
            await closeToMascot();
          }}
        >
          {basketballMode
            ? t("mascot.menuBasketStop")
            : t("mascot.menuBasketStart")}
        </button>
        <button
          type="button"
          className="mascot-menu-item"
          onClick={async () => {
            await closeToMascot();
            await invoke("toggle_mascot_visible");
          }}
        >
          {t("mascot.menuHide")}
        </button>
        <div className="mascot-menu-sep" />
        <button
          type="button"
          className="mascot-menu-item"
          onClick={() => void invoke("quit_app")}
        >
          {t("mascot.menuQuit")}
        </button>
      </div>
    );
  }

  if (mode === "suggest" && suggestions.length > 0) {
    return (
      <div className="mascot-stage">
        <div className="island-suggest-bar">
          <MeetinSheep
            ref={svgRef}
            anim="stare"
            size={42}
            pupilOffset={pupilOffset}
            pausedMotion={!isVisible}
          />
          <div className="suggest-pills-row">
            {suggestions.map((item, idx) => (
              <button
                key={item.phrase_id}
                type="button"
                className={`suggest-pill ${idx === 0 ? "is-primary" : ""}`}
                onClick={() =>
                  void invoke("accept_typing_suggestion", {
                    phraseId: item.phrase_id,
                  })
                }
                title={item.preview}
              >
                {idx === 0 && <span className="suggest-kbd">Tab</span>}
                <strong>{item.abbr}</strong>
              </button>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (mode === "pill" && pillNotice) {
    return (
      <div className="mascot-stage">
        <div className="island-pill">
          <MeetinSheep
            ref={svgRef}
            anim={anim}
            size={48}
            pupilOffset={pupilOffset}
            pausedMotion={!isVisible}
          />
          <span
            className={`island-pill-text ${pillNotice.ok ? "island-pill-ok" : "island-pill-err"}`}
          >
            {pillNotice.text}
          </span>
        </div>
      </div>
    );
  }

  if (mode === "search") {
    return (
      <div className="island-search-panel">
        <div
          className={`island-search-top ${isDraggingPanel ? "is-dragging" : ""}`}
          onPointerDown={handlePanelPointerDown}
          onPointerMove={handlePanelPointerMove}
          onPointerUp={handlePanelPointerUp}
          onPointerCancel={handlePanelPointerUp}
        >
          <span className="island-drag-grip" aria-hidden="true">
            ⋮⋮
          </span>
          <button
            type="button"
            className="island-mascot-close-btn"
            title={t("island.closeMascotTip")}
            aria-label={t("island.closeMascotTip")}
            onClick={() => void closeToMascot()}
          >
            <MeetinSheep
              ref={svgRef}
              anim={anim}
              size={48}
              pupilOffset={pupilOffset}
              pausedMotion={!isVisible}
            />
          </button>
          <input
            ref={searchInputRef}
            className="island-search-input"
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={handleSearchKeyDown}
            placeholder={t("island.searchPlaceholder")}
            autoFocus
          />
          <button
            type="button"
            className="island-kbd"
            onClick={() => void closeToMascot()}
          >
            Esc
          </button>
        </div>

        <div className="island-search-body">
          <div className="island-results">
            {results.length === 0 ? (
              <div className="island-empty">{t("island.noResults")}</div>
            ) : (
              results.map((item, idx) => (
                <button
                  key={item.id}
                  type="button"
                  className={`island-result-item ${idx === selectedIndex ? "is-selected" : ""}`}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  onClick={() => void confirmSelection(item)}
                >
                  <span className="island-result-abbr">{item.abbr}</span>
                  <span className="island-result-snippet">
                    {item.description || item.content}
                  </span>
                </button>
              ))
            )}
          </div>

          <div className="island-preview">
            {selectedPhrase ? (
              <>
                <div className="island-preview-title">
                  <span className="abbr-chip">{selectedPhrase.abbr}</span>
                  <span className="muted small">
                    {selectedPhrase.use_count}×
                  </span>
                </div>
                {selectedPhrase.description && (
                  <div className="island-preview-desc">
                    {selectedPhrase.description}
                  </div>
                )}
                <div className="island-preview-content">
                  {selectedPhrase.content}
                </div>
              </>
            ) : (
              <div className="island-empty">{t("island.previewEmpty")}</div>
            )}
          </div>
        </div>

        <div className="island-search-footer">
          <span>{t("island.navHint")}</span>
          <span>{t("island.pasteHint")}</span>
        </div>
      </div>
    );
  }

  if (mode === "add") {
    return (
      <div className="island-search-panel">
        <div
          className={`island-search-top ${isDraggingPanel ? "is-dragging" : ""}`}
          onPointerDown={handlePanelPointerDown}
          onPointerMove={handlePanelPointerMove}
          onPointerUp={handlePanelPointerUp}
          onPointerCancel={handlePanelPointerUp}
        >
          <span className="island-drag-grip" aria-hidden="true">
            ⋮⋮
          </span>
          <button
            type="button"
            className="island-mascot-close-btn"
            title={t("island.closeMascotTip")}
            aria-label={t("island.closeMascotTip")}
            onClick={() => void closeToMascot()}
          >
            <MeetinSheep
              ref={svgRef}
              anim={anim}
              size={44}
              pupilOffset={pupilOffset}
              pausedMotion={!isVisible}
            />
          </button>
          <span className="island-add-title">{t("add.title")}</span>
          <button
            type="button"
            className="btn island-mini-btn"
            onClick={() => void handleFillFromClipboard()}
          >
            {t("add.fromClipboard")}
          </button>
          <button
            type="button"
            className="island-kbd"
            onClick={() => void closeToMascot()}
          >
            Esc
          </button>
        </div>

        <form
          className="island-add-body"
          onSubmit={(e) => {
            e.preventDefault();
            void handleSaveAdd();
          }}
        >
          <div className="island-add-row">
            <label className="island-field">
              <span>{t("form.abbrLabel")}</span>
              <input
                ref={addAbbrInputRef}
                className="input"
                value={addAbbr}
                onChange={(e) => {
                  setAddAbbr(e.target.value);
                  setAddAbbrManual(true);
                }}
                placeholder={t("form.abbrPlaceholder")}
                autoFocus
              />
            </label>
            <label className="island-field">
              <span>{t("add.folderLabel")}</span>
              <select
                className="select"
                value={showInlineNewFolder ? "__new__" : (addFolderId ?? "")}
                onChange={(e) => {
                  const v = e.target.value;
                  if (v === "__new__") {
                    setShowInlineNewFolder(true);
                    return;
                  }
                  setShowInlineNewFolder(false);
                  const fid = v === "" ? null : Number(v);
                  setAddFolderId(fid);
                  lastUsedFolderIdRef.current = fid;
                  if (fid !== null) {
                    const fname = folders.find((f) => f.id === fid)?.name;
                    if (fname) {
                      setAddTagsText((prev) => {
                        const list = prev.split(",").map((s) => s.trim()).filter(Boolean);
                        return list.includes(fname) ? prev : [...list, fname].join(", ");
                      });
                    }
                  }
                }}
              >
                <option value="">{t("add.noFolder")}</option>
                {folders.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.name}
                  </option>
                ))}
                <option value="__new__">{t("add.newFolderOption")}</option>
              </select>
            </label>
          </div>

          {showInlineNewFolder && (
            <div className="island-inline-new-folder">
              <input
                className="input"
                autoFocus
                value={inlineFolderName}
                onChange={(e) => setInlineFolderName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void handleCreateInlineFolder();
                  } else if (e.key === "Escape") {
                    e.stopPropagation();
                    setShowInlineNewFolder(false);
                  }
                }}
                placeholder={t("add.newFolderPlaceholder")}
              />
              <button
                type="button"
                className="btn btn-primary island-mini-btn"
                onClick={() => void handleCreateInlineFolder()}
              >
                {t("add.createFolderBtn")}
              </button>
              <button
                type="button"
                className="btn island-mini-btn"
                onClick={() => setShowInlineNewFolder(false)}
              >
                ×
              </button>
            </div>
          )}

          {conflictingPhrase && (
            <div className="badge badge-warn">
              {t("add.conflictWarn", {
                abbr: conflictingPhrase.abbr,
                preview: conflictingPhrase.content.slice(0, 32),
              })}
            </div>
          )}

          <label className="island-field">
            <span>{t("add.contentPreview")}</span>
            <textarea
              className="textarea"
              rows={3}
              value={addContent}
              onChange={(e) => {
                const nextContent = e.target.value;
                setAddContent(nextContent);
                if (!addDescriptionManual) {
                  const firstLine =
                    nextContent
                      .split(/\r?\n/)
                      .map((l) => l.trim())
                      .find(Boolean) ?? "";
                  setAddDescription(firstLine.slice(0, 80));
                }
                if (!addAbbrManual) {
                  const tags = addTagsText
                    .split(",")
                    .map((s) => s.trim())
                    .filter(Boolean);
                  setAddAbbr(suggestAbbreviation(nextContent, abbrSuggestMode, tags));
                }
              }}
              placeholder={t("form.contentPlaceholder")}
            />
          </label>

          <div className="island-add-row">
            <label className="island-field">
              <span>{t("form.descriptionLabel")}</span>
              <input
                className="input"
                value={addDescription}
                onChange={(e) => {
                  setAddDescription(e.target.value);
                  setAddDescriptionManual(e.target.value.trim().length > 0);
                }}
                placeholder={t("form.descriptionPlaceholder")}
              />
            </label>
            <label className="island-field">
              <span>{t("form.tagsLabel")}</span>
              <input
                className="input"
                value={addTagsText}
                onChange={(e) => {
                  const nextTagsText = e.target.value;
                  setAddTagsText(nextTagsText);
                  if (!addAbbrManual && abbrSuggestMode === "tag") {
                    const tags = nextTagsText
                      .split(",")
                      .map((s) => s.trim())
                      .filter(Boolean);
                    setAddAbbr(suggestAbbreviation(addContent, "tag", tags));
                  }
                }}
                placeholder={t("form.tagsPlaceholder")}
              />
            </label>
          </div>

          {addError && <div className="error-banner">{addError}</div>}

          <div className="island-add-actions">
            <button
              type="button"
              className="btn island-mini-btn"
              onClick={() => void handleDetailedEdit()}
            >
              {t("add.detailedEdit")}
            </button>
            <div className="island-add-actions-right">
              <button
                type="button"
                className="btn island-mini-btn"
                onClick={() => void closeToMascot()}
              >
                {t("form.cancel")}
              </button>
              <button type="submit" className="btn btn-primary island-mini-btn">
                {t("form.save")}
              </button>
            </div>
          </div>
        </form>
      </div>
    );
  }

  if (mode === "fill" && fillPayload) {
    return (
      <div className="island-search-panel">
        <div
          className={`island-search-top ${isDraggingPanel ? "is-dragging" : ""}`}
          onPointerDown={handlePanelPointerDown}
          onPointerMove={handlePanelPointerMove}
          onPointerUp={handlePanelPointerUp}
          onPointerCancel={handlePanelPointerUp}
        >
          <span className="island-drag-grip" aria-hidden="true">
            ⋮⋮
          </span>
          <button
            type="button"
            className="island-mascot-close-btn"
            title={t("island.closeMascotTip")}
            aria-label={t("island.closeMascotTip")}
            onClick={() => void closeToMascot()}
          >
            <MeetinSheep
              ref={svgRef}
              anim={anim}
              size={44}
              pupilOffset={pupilOffset}
              pausedMotion={!isVisible}
            />
          </button>
          <span className="island-add-title">
            {t("fill.title", { abbr: fillPayload.abbr })}
          </span>
          <button
            type="button"
            className="island-kbd"
            onClick={() => void closeToMascot()}
          >
            Esc
          </button>
        </div>

        <form
          className="island-add-body"
          onSubmit={async (e) => {
            e.preventDefault();
            setMode("mascot");
            try {
              await invoke("submit_fill_island", { values: fillValues });
              setFillPayload(null);
            } catch {
              await closeToMascot();
            }
          }}
        >
          {fillPayload.fields.map((field, idx) => (
            <label key={field.name} className="island-field">
              <span>{field.name}</span>
              {field.options ? (
                <select
                  ref={
                    idx === 0
                      ? (el) => {
                          fillFirstInputRef.current = el;
                        }
                      : undefined
                  }
                  className="select"
                  value={fillValues[field.name] ?? ""}
                  onChange={(e) =>
                    setFillValues({
                      ...fillValues,
                      [field.name]: e.target.value,
                    })
                  }
                >
                  {field.options.map((opt) => (
                    <option key={opt} value={opt}>
                      {opt}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  ref={
                    idx === 0
                      ? (el) => {
                          fillFirstInputRef.current = el;
                        }
                      : undefined
                  }
                  className="input"
                  value={fillValues[field.name] ?? ""}
                  onChange={(e) =>
                    setFillValues({
                      ...fillValues,
                      [field.name]: e.target.value,
                    })
                  }
                />
              )}
            </label>
          ))}

          <div className="island-add-actions">
            <span className="muted small">{t("fill.hint")}</span>
            <div className="island-add-actions-right">
              <button
                type="button"
                className="btn island-mini-btn"
                onClick={() => void closeToMascot()}
              >
                {t("form.cancel")}
              </button>
              <button type="submit" className="btn btn-primary island-mini-btn">
                {t("fill.insert")}
              </button>
            </div>
          </div>
        </form>
      </div>
    );
  }

  return (
    <div className="mascot-stage">
      <button
        type="button"
        className="mascot-hit"
        aria-label={t("mascot.ariaLabel")}
        onPointerDown={handleMascotPointerDown}
        onPointerMove={handleMascotPointerMove}
        onPointerUp={handleMascotPointerUp}
        onPointerCancel={handleMascotPointerCancel}
        onLostPointerCapture={handleMascotPointerCancel}
        onContextMenu={handleMascotContextMenu}
      >
        <MeetinSheep
          ref={svgRef}
          anim={paused ? "sleep" : anim}
          size={mascotSize === "small" ? 62 : mascotSize === "large" ? 84 : 70}
          pupilOffset={pupilOffset}
          flightAngleDeg={flightAngleDeg}
          pausedMotion={!isVisible}
          showAdminBadge={isAdminWindow}
          adminTooltip={t("mascot.adminTooltip")}
        />
      </button>
    </div>
  );
}
