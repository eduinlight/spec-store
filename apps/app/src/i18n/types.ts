import type { en } from "./en";

type Shape<T> = { [K in keyof T]: T[K] extends string ? string : Shape<T[K]> };

export type Translation = Shape<typeof en>;
