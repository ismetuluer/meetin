import { t } from "../i18n";

interface StudioBannersProps {
  error: string | null;
  notice: string | null;
  availableUpdateVersion: string | null;
  showUpdateBanner: boolean;
  onDismissError: () => void;
  onOpenUpdate: () => void;
  onDismissUpdate: () => void;
}

/** Hata şeridi, kısa süreli işlem bildirimi ve güncelleme şeridi. */
export function StudioBanners({
  error,
  notice,
  availableUpdateVersion,
  showUpdateBanner,
  onDismissError,
  onOpenUpdate,
  onDismissUpdate,
}: StudioBannersProps) {
  return (
    <>
      {error && (
        <div className="banner-inline is-error" role="alert">
          <span>{error}</span>
          <button
            type="button"
            className="mini-icon-btn"
            title={t("studio.dismiss")}
            aria-label={t("studio.dismiss")}
            onClick={onDismissError}
          >
            ×
          </button>
        </div>
      )}
      {/* İşlem bildirimi düzeni kaydırmaz, kısa süre sonra kaybolur (R3.2) */}
      <div className="studio-toast-region" role="status" aria-live="polite">
        {notice && (
          <div key={notice} className="studio-toast">
            {notice}
          </div>
        )}
      </div>
      {availableUpdateVersion && showUpdateBanner && (
        <div
          className="banner-inline is-ok"
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 12,
          }}
        >
          <span>
            {t("studio.updateAvailableBanner", { ver: availableUpdateVersion })}
          </span>
          <div className="row-actions" style={{ alignItems: "center" }}>
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={onOpenUpdate}
            >
              {t("studio.openUpdateBtn")}
            </button>
            <button
              type="button"
              className="mini-icon-btn"
              onClick={onDismissUpdate}
            >
              ×
            </button>
          </div>
        </div>
      )}
    </>
  );
}
