import type { SVGProps } from "react";

export type IconName =
  | "activity"
  | "arrow-down"
  | "arrow-up"
  | "chevron"
  | "close"
  | "command"
  | "database"
  | "download"
  | "globe"
  | "history"
  | "map"
  | "pause"
  | "play"
  | "process"
  | "record"
  | "search"
  | "settings"
  | "shield"
  | "stop"
  | "terminal";

const paths: Record<IconName, string[]> = {
  activity: ["M3 12h4l2.5-7 5 14 2.5-7H21"],
  "arrow-down": ["M12 3v18", "m18 15-6 6-6-6"],
  "arrow-up": ["M12 21V3", "m6 9-6-6-6 6"],
  chevron: ["m9 18 6-6-6-6"],
  close: ["M18 6 6 18", "m6 6 12 12"],
  command: [
    "M18 9a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12Z",
  ],
  database: [
    "M4 6c0 2 3.6 3 8 3s8-1 8-3-3.6-3-8-3-8 1-8 3Z",
    "M4 6v6c0 2 3.6 3 8 3s8-1 8-3V6",
    "M4 12v6c0 2 3.6 3 8 3s8-1 8-3v-6",
  ],
  download: ["M12 3v12", "m7 10 5 5 5-5", "M5 21h14"],
  globe: [
    "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20Z",
    "M2 12h20",
    "M12 2c2.5 2.7 3.8 6 3.8 10S14.5 19.3 12 22c-2.5-2.7-3.8-6-3.8-10S9.5 4.7 12 2Z",
  ],
  history: ["M3 12a9 9 0 1 0 3-6.7L3 8", "M3 3v5h5", "M12 7v5l3 2"],
  map: ["m3 6 6-3 6 3 6-3v15l-6 3-6-3-6 3V6Z", "M9 3v15", "M15 6v15"],
  pause: ["M8 5v14", "M16 5v14"],
  play: ["m8 5 11 7-11 7V5Z"],
  process: [
    "M9 3h6v6H9V3Z",
    "M3 15h6v6H3v-6Z",
    "M15 15h6v6h-6v-6Z",
    "M12 9v3",
    "M6 15v-3h12v3",
  ],
  record: [
    "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z",
    "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z",
  ],
  search: ["m21 21-4.3-4.3", "M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16Z"],
  settings: [
    "M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z",
    "M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2 3.4-.2-.1a1.7 1.7 0 0 0-1.8-.3l-1 .6a1.7 1.7 0 0 0-.8 1.5v.3h-4v-.3a1.7 1.7 0 0 0-.8-1.5l-1-.6a1.7 1.7 0 0 0-1.8.3l-.2.1-2-3.4.1-.1a1.7 1.7 0 0 0 .3-1.9l-.5-1a1.7 1.7 0 0 0-1.5-1H2V9h.2a1.7 1.7 0 0 0 1.5-1l.5-1a1.7 1.7 0 0 0-.3-1.9l-.1-.1 2-3.4.2.1A1.7 1.7 0 0 0 7.8 2l1-.6a1.7 1.7 0 0 0 .8-1.5V0h4v.3a1.7 1.7 0 0 0 .8 1.5l1 .6a1.7 1.7 0 0 0 1.8-.3l.2-.1 2 3.4-.1.1a1.7 1.7 0 0 0-.3 1.9l.5 1a1.7 1.7 0 0 0 1.5 1h.2v4H21a1.7 1.7 0 0 0-1.5 1l-.1.6Z",
  ],
  shield: ["M12 22s8-4 8-11V5l-8-3-8 3v6c0 7 8 11 8 11Z", "m9 12 2 2 4-5"],
  stop: ["M6 6h12v12H6z"],
  terminal: ["m4 17 6-6-6-6", "M12 19h8"],
};

export function Icon({
  name,
  ...props
}: { name: IconName } & SVGProps<SVGSVGElement>) {
  return (
    <svg
      aria-hidden="true"
      fill="none"
      height="18"
      viewBox="0 0 24 24"
      width="18"
      {...props}
    >
      {paths[name].map((path, index) => (
        <path
          d={path}
          key={`${name}-${index}`}
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="1.7"
        />
      ))}
    </svg>
  );
}
