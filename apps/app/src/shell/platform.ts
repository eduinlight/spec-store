export const isApplePlatform = (): boolean =>
  typeof navigator !== "undefined" && /mac|iphone|ipad/i.test(navigator.platform);

export const paletteKeys = (): string[] => (isApplePlatform() ? ["⌘", "K"] : ["Ctrl", "K"]);

export const specLabel = (number: number): string => String(number).padStart(3, "0");
