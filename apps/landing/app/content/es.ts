import type { en } from "./en";

type Shape<T> = { [K in keyof T]: T[K] extends string ? string : Shape<T[K]> };

export const es: Shape<typeof en> = {
  meta: {
    title: "spec-store — Specs en las que tus agentes pueden confiar",
    description:
      "Guarda tus documentos de Spec Driven Development en un solo lugar que comparten tu equipo y tus agentes de IA.",
  },
  nav: { signIn: "Iniciar sesión", language: "English", languageHref: "/" },
  hero: {
    headline: "Specs en las que tus agentes pueden confiar",
    body: "spec-store guarda tus documentos de Spec Driven Development — constitución, specs, planes y tareas — en un solo lugar que comparten tu equipo y tus agentes de IA.",
    signUp: "Crea tu cuenta",
    signIn: "Iniciar sesión",
  },
  features: {
    title: "Por qué spec-store",
    agents: {
      title: "Pensado para agentes",
      body: "Conecta Claude Code o cualquier cliente MCP aprobándolo en tu navegador. Lee y escribe specs con exactamente tu acceso.",
    },
    history: {
      title: "Nada se pierde",
      body: "Cada cambio es una revisión con su autor y su fecha. Compara dos versiones y mira quién cambió qué.",
    },
    teams: {
      title: "Hecho para equipos",
      body: "Las organizaciones son dueñas de sus proyectos. Invita personas, dales roles y mantén cada organización separada.",
    },
  },
  footer: { rights: "spec-store" },
};
