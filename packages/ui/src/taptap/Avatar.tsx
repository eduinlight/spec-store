import type { ComponentPropsWithRef, CSSProperties } from "react";

export type AvatarShape = "circle" | "square";
export type AvatarState = "default" | "hover" | "selected" | "disabled";

const RING: Record<AvatarState, string> = {
  default: "none",
  hover: "0 0 0 2px var(--primary-500)",
  selected: "0 0 0 2px var(--primary-600)",
  disabled: "none",
};

const initialsOf = (name: string): string =>
  name
    .split(/\s+/)
    .map((word) => word[0] ?? "")
    .join("")
    .slice(0, 2)
    .toUpperCase();

const frameStyle = (
  size: number,
  shape: AvatarShape,
  filled: boolean,
  state: AvatarState,
): CSSProperties => ({
  width: size,
  height: size,
  flexShrink: 0,
  borderRadius: shape === "circle" ? "50%" : 4,
  overflow: "hidden",
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  background: filled ? "var(--neutral-100)" : "var(--primary-200)",
  color: "var(--primary-700)",
  font: `500 ${Math.round(size * 0.4)}px/1 var(--font-sans)`,
  boxShadow: RING[state],
  opacity: state === "disabled" ? 0.4 : 1,
});

export interface AvatarProps extends ComponentPropsWithRef<"span"> {
  src?: string;
  name?: string;
  size?: number;
  shape?: AvatarShape;
  state?: AvatarState;
}

export function Avatar({
  src,
  name = "",
  size = 32,
  shape = "circle",
  state = "default",
  style,
  ...rest
}: AvatarProps) {
  const frame = { ...frameStyle(size, shape, Boolean(src), state), ...style };
  if (src) {
    return (
      <span {...rest} style={frame}>
        <img src={src} alt={name} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
      </span>
    );
  }
  return (
    <span role="img" aria-label={name} {...rest} style={frame}>
      {initialsOf(name)}
    </span>
  );
}

export interface AvatarGroupProps extends ComponentPropsWithRef<"span"> {
  srcs?: ReadonlyArray<string | undefined>;
  names?: ReadonlyArray<string>;
  size?: number;
  max?: number;
}

export function AvatarGroup({
  srcs = [],
  names = [],
  size = 28,
  max = 3,
  style,
  ...rest
}: AvatarGroupProps) {
  const visible = srcs.slice(0, max).map((src, index) => ({
    id: `avatar-${index}`,
    src,
    name: names[index],
    overlap: index > 0,
  }));
  const hidden = Math.max(0, srcs.length - visible.length);
  return (
    // biome-ignore lint/a11y/useSemanticElements: an avatar stack is a visual group of people, not a form fieldset
    <span role="group" {...rest} style={{ display: "inline-flex", ...style }}>
      {visible.map((item) => (
        <Avatar
          key={item.id}
          src={item.src}
          name={item.name}
          size={size}
          style={{ marginLeft: item.overlap ? -8 : 0, boxShadow: "0 0 0 1px #fff" }}
        />
      ))}
      {hidden > 0 ? (
        <span
          role="img"
          aria-label={`${hidden} more`}
          style={{
            ...frameStyle(size, "circle", false, "default"),
            marginLeft: visible.length ? -8 : 0,
            boxShadow: "0 0 0 1px #fff",
          }}
        >
          {`+${hidden}`}
        </span>
      ) : null}
    </span>
  );
}
