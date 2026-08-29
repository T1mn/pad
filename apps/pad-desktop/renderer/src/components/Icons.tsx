import type { ReactNode, SVGProps } from "react";

export type IconName =
  | "archive"
  | "arrow-left"
  | "attachment"
  | "check"
  | "chevron-down"
  | "chevron-right"
  | "code"
  | "file"
  | "folder"
  | "layout"
  | "menu"
  | "more"
  | "panel-bottom"
  | "panel-right"
  | "plus"
  | "search"
  | "send"
  | "settings"
  | "sparkles"
  | "terminal"
  | "x";

const paths: Record<IconName, ReactNode> = {
  archive: <><path d="M4 7h16"/><path d="M6 7v12h12V7"/><path d="M9 11h6"/><path d="M5 3h14v4H5z"/></>,
  "arrow-left": <><path d="m15 18-6-6 6-6"/></>,
  attachment: <path d="m20 11-8.3 8.3a5 5 0 0 1-7-7L14 3a3.5 3.5 0 0 1 5 5l-9.2 9.2a2 2 0 0 1-2.8-2.8l8.5-8.5"/>,
  check: <path d="m5 12 4 4L19 6"/>,
  "chevron-down": <path d="m8 10 4 4 4-4"/>,
  "chevron-right": <path d="m10 8 4 4-4 4"/>,
  code: <><path d="m8 9-3 3 3 3"/><path d="m16 9 3 3-3 3"/><path d="m14 5-4 14"/></>,
  file: <><path d="M6 3h8l4 4v14H6z"/><path d="M14 3v5h5"/></>,
  folder: <path d="M3 6h7l2 2h9v11H3z"/>,
  layout: <><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M8 4v16"/></>,
  menu: <><path d="M4 7h16"/><path d="M4 12h16"/><path d="M4 17h16"/></>,
  more: <><circle cx="5" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="19" cy="12" r="1" fill="currentColor" stroke="none"/></>,
  "panel-bottom": <><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 15h18"/></>,
  "panel-right": <><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/></>,
  plus: <><path d="M12 5v14"/><path d="M5 12h14"/></>,
  search: <><circle cx="11" cy="11" r="6"/><path d="m16 16 4 4"/></>,
  send: <><path d="m5 12 7-7 7 7"/><path d="M12 19V5"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.9 4.9 7 7M17 17l2.1 2.1M2 12h3M19 12h3M4.9 19.1 7 17M17 7l2.1-2.1"/></>,
  sparkles: <><path d="m12 3 1.2 3.8L17 8l-3.8 1.2L12 13l-1.2-3.8L7 8l3.8-1.2z"/><path d="m18 14 .7 2.3L21 17l-2.3.7L18 20l-.7-2.3L15 17l2.3-.7z"/></>,
  terminal: <><path d="m5 7 4 4-4 4"/><path d="M11 17h7"/><rect x="2.5" y="3.5" width="19" height="17" rx="2"/></>,
  x: <><path d="m6 6 12 12"/><path d="M18 6 6 18"/></>,
};

export function Icon({ name, ...props }: { name: IconName } & SVGProps<SVGSVGElement>) {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      {...props}
    >
      {paths[name]}
    </svg>
  );
}
