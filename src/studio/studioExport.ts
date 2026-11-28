import { invoke } from "@tauri-apps/api/core";
import type { ExportFormat, ExportRequest } from "./ExportDialog";
import { exportMime } from "./studioHelpers";

/**
 * Seçilen kapsam ve biçimde dışa aktarma dosyasını çekirdekten oluşturur (R3.4).
 * Hata çağırana fırlatılır; çağıran kullanıcıya gösterir.
 */
export async function createExportFile(
  request: ExportRequest,
  format: ExportFormat,
): Promise<{ blob: Blob; fileName: string }> {
  let blob: Blob;
  const ext = format === "meetin" ? "mee" : format;
  if (request.scope === "library") {
    if (format === "meetin") {
      const bytes = await invoke<number[]>("export_mee_archive");
      blob = new Blob([new Uint8Array(bytes)], { type: "application/zip" });
    } else {
      const text = await invoke<string>("export_library", { format });
      blob = new Blob([text], { type: exportMime(format) });
    }
  } else if (format === "meetin") {
    const bytes = await invoke<number[]>("export_phrases_subset_mee", {
      phraseIds: request.ids,
    });
    blob = new Blob([new Uint8Array(bytes)], { type: "application/zip" });
  } else {
    const text = await invoke<string>("export_phrases_subset", {
      phraseIds: request.ids,
      format,
    });
    blob = new Blob([text], { type: exportMime(format) });
  }
  const fileName = `${request.scope === "library" ? "meetin-yedek" : request.fileName}.${ext}`;
  return { blob, fileName };
}

/** Dosyayı tarayıcı indirmesi olarak kaydettirir. */
export function downloadBlob(blob: Blob, fileName: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}
