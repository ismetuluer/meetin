import { forwardRef } from "react";
import { getMascotFlags, type MascotAnim } from "./mascotMachine";
import "./mascot.css";

export interface MeetinSheepProps {
  anim: MascotAnim;
  size?: number;
  pupilOffset?: { x: number; y: number };
  flightAngleDeg?: number;
  pausedMotion?: boolean;
  showAdminBadge?: boolean;
  adminTooltip?: string;
  showUpdateDot?: boolean;
  baaText?: string;
}

/**
 * Meetin koyunu — Harman B (`docs/meetin-maskot.svg` ve `docs/meetin-maskot-animasyon.html` birebir SVG yapısı).
 * `thumbsup` (iki başparmak onay) ve `super` (pelerinli uçuş) durumlarını içerir.
 */
export const MeetinSheep = forwardRef<SVGSVGElement, MeetinSheepProps>(
  function MeetinSheep(
    {
      anim,
      size = 70,
      pupilOffset = { x: 0, y: 0 },
      flightAngleDeg = 0,
      pausedMotion = false,
      showAdminBadge = false,
      adminTooltip = "",
      showUpdateDot = false,
      baaText = "Mee!",
    },
    ref,
  ) {
    const f = getMascotFlags(anim);
    const svgClass = `sheep k-b2 a-${anim}${pausedMotion ? " is-paused-motion" : ""}`;

    // Uçuş yönüne göre başı hafifçe eğ (ama baş aşağı dönmesin)
    const rad = (flightAngleDeg * Math.PI) / 180;
    const headTiltDeg = f.isSuper
      ? Math.max(-16, Math.min(16, Math.sin(rad) * 12 + Math.cos(rad) * 8))
      : 0;

    return (
      <div
        className="mascot-wrap"
        style={{ width: size, height: size }}
        title={showAdminBadge ? adminTooltip : undefined}
      >
        <svg
          ref={ref}
          viewBox="0 0 200 200"
          width={size}
          height={size}
          aria-hidden="true"
          className={svgClass}
        >
          <ellipse
            className="gshadow"
            cx="100"
            cy="186"
            rx="54"
            ry="6"
            fill="rgba(0,0,0,.12)"
          />
          <g className="grass" aria-hidden="true">
            <path
              d="M144 186 Q142 170 138 164 M148 186 Q149 168 152 160 M154 186 Q157 170 162 166 M160 186 Q162 172 168 168 M166 186 Q166 174 172 172"
              stroke="#6FA86A"
              strokeWidth="3"
              strokeLinecap="round"
              fill="none"
            />
            <circle cx="152" cy="160" r="3" fill="#F2D35B" />
          </g>
          {f.isSleep && (
            <g className="zzz" fill="#6B5D4A" fontWeight="800">
              <text x="146" y="62" fontSize="16">
                z
              </text>
              <text className="z2" x="158" y="46" fontSize="20">
                z
              </text>
              <text className="z3" x="172" y="28" fontSize="25">
                Z
              </text>
            </g>
          )}
          <g className="card" aria-hidden="true">
            <rect
              x="150"
              y="20"
              width="34"
              height="24"
              rx="5"
              fill="#FFFFFF"
              stroke="#6B5D4A"
              strokeWidth="1.5"
            />
            <path
              d="M156 28 H178 M156 33 H172 M156 38 H175"
              stroke="var(--vurgu)"
              strokeWidth="2"
              strokeLinecap="round"
            />
          </g>

          {/* Süper Koyun Pelerini, Uçuş Rüzgarı ve Yöne Uzanan Toynaklar (Görsel 2) */}
          {f.isSuper && (
            <g
              className="super-flight-dir"
              style={{
                transform: `rotate(${flightAngleDeg}deg)`,
                transformOrigin: "100px 108px",
              }}
            >
              {/* Arkada dalgalanan kırmızı pelerin */}
              <g className="super-cape-fabric">
                <path
                  d="M92 84 C62 66 28 74 10 92 C24 108 18 124 8 136 C36 144 68 140 94 126 Z"
                  fill="#E03E2D"
                  stroke="#9F1F14"
                  strokeWidth="2.5"
                  strokeLinejoin="round"
                />
                <path
                  d="M84 94 C58 88 38 94 24 106"
                  stroke="#F58272"
                  strokeWidth="2"
                  strokeLinecap="round"
                  fill="none"
                  opacity="0.7"
                />
              </g>

              {/* Hız çizgileri */}
              <path
                d="M14 68 H44 M4 108 H30 M18 148 H48"
                stroke="#94A3B8"
                strokeWidth="2.5"
                strokeLinecap="round"
                opacity="0.7"
              />

              {/* Arkaya uzanan arka toynak */}
              <g>
                <path
                  d="M64 128 L34 146"
                  stroke="#2E2A28"
                  strokeWidth="12"
                  strokeLinecap="round"
                />
                <path
                  d="M35 145 L22 153 L30 158 L38 152 L39 161 L47 156 L40 144 Z"
                  fill="#1F1B19"
                />
              </g>

              {/* Uçuş yönüne (+X eksenine) ileri uzatılmış ön toynak */}
              <g>
                <path
                  d="M126 108 L174 108"
                  stroke="#2E2A28"
                  strokeWidth="13"
                  strokeLinecap="round"
                />
                {/* Çatal toynak ucu */}
                <path
                  d="M170 101 L192 103 L184 108 L192 113 L170 115 Z"
                  fill="#1F1B19"
                  stroke="#2E2A28"
                  strokeWidth="1.5"
                  strokeLinejoin="round"
                />
              </g>
            </g>
          )}

          <g className="sh-all">
            {!f.isSuper && (
              <g className="legs" fill="#2E2A28">
                <rect x="66" y="158" width="18" height="26" rx="9" />
                <rect x="116" y="158" width="18" height="26" rx="9" />
                <rect
                  x="86"
                  y="164"
                  width="13"
                  height="22"
                  rx="6.5"
                  fill="#1F1B19"
                />
                <rect
                  x="101"
                  y="164"
                  width="13"
                  height="22"
                  rx="6.5"
                  fill="#1F1B19"
                />
              </g>
            )}
            <g className="wool">
              <path
                d="M100.0 46.0 A15.4 15.4 0 0 1 124.6 50.2 A15.1 15.1 0 0 1 145.8 62.2 A14.6 14.6 0 0 1 160.9 80.4 A14.2 14.2 0 0 1 167.7 102.3 A14.1 14.1 0 0 1 165.4 125.0 A14.4 14.4 0 0 1 154.3 145.4 A14.9 14.9 0 0 1 135.8 160.7 A15.3 15.3 0 0 1 112.5 168.9 A15.5 15.5 0 0 1 87.5 168.9 A15.3 15.3 0 0 1 64.2 160.7 A14.9 14.9 0 0 1 45.7 145.4 A14.4 14.4 0 0 1 34.6 125.0 A14.1 14.1 0 0 1 32.3 102.3 A14.2 14.2 0 0 1 39.1 80.4 A14.6 14.6 0 0 1 54.2 62.2 A15.1 15.1 0 0 1 75.4 50.2 A15.4 15.4 0 0 1 100.0 46.0 Z"
                fill="#FFFFFF"
                stroke="#2E2A28"
                strokeWidth="3"
                strokeLinejoin="round"
              />
            </g>
            <g
              className="head"
              style={
                f.isSuper
                  ? {
                      transform: `rotate(${headTiltDeg}deg)`,
                      transformOrigin: "100px 112px",
                    }
                  : undefined
              }
            >
              <g className="ear-l">
                <ellipse
                  cx="62"
                  cy="94"
                  rx="17"
                  ry="8.5"
                  fill="#2E2A28"
                  transform="rotate(-24 62 94)"
                />
              </g>
              <g className="ear-r">
                <ellipse
                  cx="138"
                  cy="94"
                  rx="17"
                  ry="8.5"
                  fill="#2E2A28"
                  transform="rotate(24 138 94)"
                />
              </g>
              <path
                d="M100 74 C124 74 134 94 132 116 C130 140 117 154 100 154 C83 154 70 140 68 116 C66 94 76 74 100 74 Z"
                fill="#2E2A28"
              />
              <path
                d="M100.0 60.0 A11.0 11.0 0 0 1 115.4 62.8 A7.6 7.6 0 0 1 123.6 69.9 A6.0 6.0 0 0 1 120.8 78.0 A9.5 9.5 0 0 1 108.2 83.3 A11.5 11.5 0 0 1 91.8 83.3 A9.5 9.5 0 0 1 79.2 78.0 A6.0 6.0 0 0 1 76.4 69.9 A7.6 7.6 0 0 1 84.6 62.8 A11.0 11.0 0 0 1 100.0 60.0 Z"
                fill="#FFFFFF"
                stroke="#2E2A28"
                strokeWidth="3"
                strokeLinejoin="round"
              />
              <ellipse
                cx="78"
                cy="132"
                rx="6"
                ry="3.6"
                fill="#C77B86"
                className="cheek"
                opacity={f.cheekOpacity}
              />
              <ellipse
                cx="122"
                cy="132"
                rx="6"
                ry="3.6"
                fill="#C77B86"
                className="cheek"
                opacity={f.cheekOpacity}
              />

              {f.eyeOpen && (
                <g className="eyes">
                  <g className="eye-w">
                    <ellipse cx="84" cy="108" rx="11.5" ry="13.5" fill="#fff" />
                    <ellipse cx="116" cy="108" rx="11.5" ry="13.5" fill="#fff" />
                  </g>
                  <g
                    className="pupil"
                    style={{
                      transform: `translate(${pupilOffset.x}px, ${pupilOffset.y}px)`,
                    }}
                  >
                    <circle cx="85" cy="110" r="7" fill="#1F1B19" />
                    <circle cx="117" cy="110" r="7" fill="#1F1B19" />
                    <circle cx="87.4" cy="106.5" r="2.5" fill="#fff" />
                    <circle cx="119.4" cy="106.5" r="2.5" fill="#fff" />
                    <circle cx="83" cy="113" r="1.1" fill="#fff" />
                    <circle cx="115" cy="113" r="1.1" fill="#fff" />
                  </g>
                </g>
              )}

              {/* Süper koyun kararlı kaşları */}
              {f.isSuper && (
                <path
                  d="M72 91 L93 97 M128 91 L107 97"
                  stroke="#EDE6DF"
                  strokeWidth="2.8"
                  strokeLinecap="round"
                />
              )}

              {f.eyeClosed && (
                <path
                  d="M72.5 109 Q84 116 95.5 109 M104.5 109 Q116 116 127.5 109"
                  stroke="#EDE6DF"
                  strokeWidth="3"
                  fill="none"
                  strokeLinecap="round"
                />
              )}

              {f.eyeHappy && (
                <path
                  d="M72.5 111 Q84 99 95.5 111 M104.5 111 Q116 99 127.5 111"
                  stroke="#EDE6DF"
                  strokeWidth="3.2"
                  fill="none"
                  strokeLinecap="round"
                />
              )}

              {f.eyeSad && (
                <g>
                  <ellipse cx="84" cy="110" rx="8.6" ry="9.4" fill="#fff" />
                  <ellipse cx="116" cy="110" rx="8.6" ry="9.4" fill="#fff" />
                  <circle cx="84" cy="113" r="5.2" fill="#1F1B19" />
                  <circle cx="116" cy="113" r="5.2" fill="#1F1B19" />
                  <path
                    d="M71.5 92.5 L89.8 96.5 M128.5 92.5 L110.2 96.5"
                    stroke="#EDE6DF"
                    strokeWidth="2.6"
                    strokeLinecap="round"
                  />
                </g>
              )}

              {f.mSmile && (
                <path
                  d="M95.2 138 Q100 142.0 104.8 138"
                  stroke="#A2958E"
                  strokeWidth="2.4"
                  fill="none"
                  strokeLinecap="round"
                />
              )}

              {f.mOpen && (
                <g>
                  <ellipse
                    cx="100"
                    cy="139.6"
                    rx="3.6"
                    ry="4.4"
                    fill="#140F0D"
                    stroke="#A2958E"
                    strokeWidth="1.5"
                  />
                  <ellipse
                    cx="100"
                    cy="141.6"
                    rx="2.08"
                    ry="1.44"
                    fill="#E58C9C"
                  />
                </g>
              )}

              {f.mChew && (
                <ellipse
                  className="chew"
                  cx="100"
                  cy="139"
                  rx="3.6"
                  ry="1.76"
                  fill="#140F0D"
                  stroke="#A2958E"
                  strokeWidth="1.5"
                />
              )}

              {f.mSad && (
                <path
                  d="M95.2 141.2 Q100 137.2 104.8 141.2"
                  stroke="#A2958E"
                  strokeWidth="2.4"
                  fill="none"
                  strokeLinecap="round"
                />
              )}

              {/* Görsel 1: Yanak tarafında özgüvenli dişli sırıtış */}
              {f.mGrin && (
                <g>
                  <path
                    d="M73 121 C63 125 63 139 74 140 C83 141 88 134 83 125 Z"
                    fill="#FFFFFF"
                    stroke="#1F1B19"
                    strokeWidth="2.2"
                    strokeLinejoin="round"
                  />
                  <path
                    d="M66 131 L84 131 M73 122 L73 139 M79 124 L79 138"
                    stroke="#2E2A28"
                    strokeWidth="1.2"
                  />
                </g>
              )}
            </g>

            {/* Görsel 1: İki yanda yukarı kalkık toynak-başparmaklar */}
            {f.isThumbsUp && (
              <g className="thumbs-hands">
                {/* Sol kol + başparmak */}
                <path
                  d="M42 134 Q26 134 26 118"
                  stroke="#2E2A28"
                  strokeWidth="12"
                  strokeLinecap="round"
                  fill="none"
                />
                <rect x="16" y="102" width="22" height="22" rx="10" fill="#2E2A28" />
                <rect
                  x="23"
                  y="83"
                  width="11"
                  height="24"
                  rx="5.5"
                  fill="#2E2A28"
                  stroke="#1F1B19"
                  strokeWidth="1.5"
                />
                <path
                  d="M17 113 H28"
                  stroke="#1F1B19"
                  strokeWidth="1.8"
                  strokeLinecap="round"
                />

                {/* Sağ kol + başparmak */}
                <path
                  d="M158 134 Q174 134 174 118"
                  stroke="#2E2A28"
                  strokeWidth="12"
                  strokeLinecap="round"
                  fill="none"
                />
                <rect x="162" y="102" width="22" height="22" rx="10" fill="#2E2A28" />
                <rect
                  x="166"
                  y="83"
                  width="11"
                  height="24"
                  rx="5.5"
                  fill="#2E2A28"
                  stroke="#1F1B19"
                  strokeWidth="1.5"
                />
                <path
                  d="M172 113 H183"
                  stroke="#1F1B19"
                  strokeWidth="1.8"
                  strokeLinecap="round"
                />
              </g>
            )}
          </g>
        </svg>

        {f.isBaa && <span className="mascot-baa-bubble">{baaText}</span>}
        {showUpdateDot && <span className="mascot-badge-update" />}
        {showAdminBadge && (
          <span
            className="mascot-badge-admin"
            aria-label={adminTooltip}
            title={adminTooltip}
          >
            🛡
          </span>
        )}
      </div>
    );
  },
);
