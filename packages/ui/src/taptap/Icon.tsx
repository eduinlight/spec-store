import type { SVGProps } from "react";
import { glyphs, type IconName } from "./icon-data";

export type { IconName } from "./icon-data";

export interface IconProps
  extends Omit<SVGProps<SVGSVGElement>, "name" | "dangerouslySetInnerHTML"> {
  name: IconName;
  size?: number;
  title?: string;
}

export const iconNames = Object.keys(glyphs) as IconName[];

export function Icon({ name, size = 24, title, style, ...rest }: IconProps) {
  const glyph = glyphs[name];
  if (!glyph) {
    return null;
  }
  const labelled = title !== undefined;
  return (
    <svg
      width={size}
      height={size}
      viewBox={glyph.viewBox}
      fill="none"
      role={labelled ? "img" : undefined}
      aria-label={title}
      aria-hidden={labelled ? undefined : true}
      focusable="false"
      style={{ flexShrink: 0, ...style }}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: glyph markup is the static icon table of the design system
      dangerouslySetInnerHTML={{ __html: glyph.body }}
      {...rest}
    />
  );
}

export function LockDocument({
  size = 16,
  title,
  style,
}: {
  size?: number;
  title?: string;
  style?: SVGProps<SVGSVGElement>["style"];
}) {
  const labelled = title !== undefined;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      role={labelled ? "img" : undefined}
      aria-label={title}
      aria-hidden={labelled ? undefined : true}
      focusable="false"
      style={{ flexShrink: 0, ...style }}
    >
      <path d="M13 21H6.5A1.5 1.5 0 0 1 5 19.5v-15A1.5 1.5 0 0 1 6.5 3h8L19 7.5V11" />
      <path d="M14.5 3v4.5H19" />
      <path d="M8.5 11h6M8.5 14.5h3" />
      <rect x="15" y="16.5" width="6" height="4.5" rx="1" />
      <path d="M16.5 16.5v-1.25a1.5 1.5 0 0 1 3 0v1.25" />
    </svg>
  );
}
