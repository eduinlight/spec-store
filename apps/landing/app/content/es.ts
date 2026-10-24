import type { LandingCopy } from "./types";

export const es: LandingCopy = {
  meta: {
    title: "SDD Vault — Specs que tus agentes de IA siguen y no pueden reescribir",
    description:
      "Guarda tu constitución, specs, planes y tareas en un solo lugar seguro y versionado. Sírvelos a todos tus agentes de IA mediante MCP. Trabaja en paralelo sin conflictos.",
  },
  cta: { startFree: "Empieza gratis", signIn: "Iniciar sesión", bookDemo: "Reserva una demo" },
  nav: {
    logoAlt: "SDD Vault",
    label: "Secciones",
    howItWorks: "Cómo funciona",
    features: "Funciones",
    security: "Seguridad",
    pricing: "Precios",
    faq: "Preguntas",
    language: "English",
    languageHref: "/",
  },
  hero: {
    badge: "Servidor MCP para Spec Driven Development",
    headline: "Specs que tus agentes de IA siguen y no pueden reescribir.",
    body: "Guarda tu constitución, specs, planes y tareas en un solo lugar seguro y versionado. Sírvelos a todos tus agentes de IA mediante MCP. Trabaja en paralelo sin conflictos.",
    worksWith: "Funciona con cualquier agente compatible con MCP",
    agents: ["Claude Code", "Cursor", "Windsurf", "VS Code", "JetBrains", "Agentes propios"],
    terminal: {
      prompt: "muestra el estado del proyecto",
      approved: "aprobada",
      phase: "fase 3",
      nextTask: "Migrar la tabla de reembolsos",
      noMarkers: "0 abiertos",
    },
    card: {
      title: "Migración a Diesel ORM",
      status: "Aprobada",
      phase: "Fase 3 · Implementación",
      tasks: "12 / 20 tareas",
      actor: "Marco",
      action: "Claude Code marcó",
      done: "como hecha",
      ago: "hace 2 min",
    },
  },
  problem: {
    eyebrow: "El problema",
    title: "Las specs se volvieron el contrato con tus agentes. Y siguen siendo archivos sueltos.",
    cards: [
      {
        tag: "constitution.md · modificado",
        title: "Los agentes editan las reglas",
        body: "Cualquier agente con acceso a archivos puede reescribir un principio en el mismo commit que el código que lo rompe. Si le pides que haga pasar una prueba, puede cambiar la spec en lugar del código.",
      },
      {
        tag: "CONFLICT (content): tasks.md",
        title: "Las specs entran en conflicto en ramas paralelas",
        body: "Dos agentes marcan tareas distintas como hechas en dos ramas. El merge entra en conflicto, o se fusiona en silencio y pierde un cambio. «La spec actual» depende de tu rama.",
      },
      {
        tag: "Status: Approved (?)",
        title: "Nadie sabe qué está aprobado",
        body: "«Aprobado» es una palabra en el encabezado de un markdown. Nada impide implementar un borrador, y el equipo de producto necesita acceso a git solo para leerlo.",
      },
    ],
  },
  how: {
    eyebrow: "Cómo funciona",
    title: "Del registro a la primera llamada de un agente en menos de tres minutos.",
    steps: [
      {
        title: "Conecta tu agente",
        body: "Pega un comando y aprueba el acceso en tu navegador. Claude Code, Cursor o cualquier cliente MCP.",
      },
      {
        title: "Escribe specs con tu agente",
        body: "El servidor asigna el siguiente número de spec. Las preguntas abiertas se vuelven marcadores y nunca se olvidan.",
      },
      {
        title: "Aprueben en equipo",
        body: "Producto, diseño y QA leen la spec renderizada, responden los marcadores y aprueban sin git.",
      },
      {
        title: "Implementen en paralelo",
        body: "Los agentes toman tareas paralelas. Cada finalización queda registrada con su commit y vinculada al FR.",
      },
    ],
  },
  features: {
    eyebrow: "Funciones",
    title: "Una sola fuente de verdad para cada rama, cada desarrollador y cada agente.",
    cards: [
      {
        title: "Gobernado",
        body: "Una constitución protegida que solo los propietarios pueden cambiar. Las transiciones de estado siguen SDD y las aprobaciones quedan registradas con nombre y hora.",
      },
      {
        title: "Versionado",
        body: "Cada escritura es una revisión inmutable con autor, agente y mensaje. Compara dos cualesquiera, restaura la que quieras. No se pierde nada.",
      },
      {
        title: "Paralelo",
        body: "Sin conflictos de merge en las specs. Bloqueo optimista en cada documento y finalización atómica de tareas. Diez agentes, una spec.",
      },
      {
        title: "Nativo para IA",
        body: "Pide la siguiente tarea, no el archivo completo. Respuestas estructuradas, menos tokens y restricciones devueltas como errores que los agentes pueden atender.",
      },
      {
        title: "Multiorganización",
        body: "Organizaciones, proyectos y roles por proyecto. Invita a colaboradores externos a un proyecto sin exponer el resto de la organización.",
      },
      {
        title: "Seguro",
        body: "SSO y SCIM, tokens con alcance que se muestran una sola vez, un registro de auditoría inmutable, cifrado en tránsito y en reposo, y aislamiento estricto entre organizaciones.",
      },
    ],
    states: ["Borrador", "Aclarada", "Aprobada", "Archivada"],
    revisions: [
      "Aclarar el plazo de reembolso",
      "Agregar FR-18 para reembolsos parciales",
      "Plan inicial",
    ],
    parallelNote: "Ambas terminaron a las 14:02:11. Sin conflicto.",
    roles: ["Propietario", "Editor", "Lector"],
    tokens: {
      readWrite: "lectura y escritura",
      readOnly: "solo lectura",
      org: "org",
      expires: "vence en 30 d",
      revoked: "Revocado",
    },
  },
  guardrails: {
    eyebrow: "Restricciones",
    title: "Tu agente no puede reescribir las reglas. Ni siquiera por accidente.",
    body: "Mientras arregla una prueba que falla, el agente de un editor intenta relajar un principio de la constitución. El servidor lo rechaza, el agente se lo informa al desarrollador y el intento queda en el registro de auditoría.",
    emphasis: "Los prompts son sugerencias. Las reglas de SDD Vault se cumplen.",
    refusal:
      'Solo los propietarios del proyecto pueden escribir la constitución. Tu rol en "payments" es editor.',
    audit: {
      denied: "Denegado",
      detail: "payments · origen 81.2.69.x · la entrada de auditoría es inmutable",
    },
  },
  concurrency: {
    eyebrow: "Concurrencia",
    title: "Dos agentes, un documento, ningún cambio perdido.",
    body: "Cada escritura declara la revisión en la que se basa. Cuando dos agentes escriben desde la misma revisión, gana exactamente uno. Al otro se le informa la revisión actual, vuelve a leer, fusiona y reintenta. Se conservan ambos cambios, cada uno con su autor.",
    agentA: "Agente A · Claude Code",
    agentB: "Agente B · Cursor",
    readPlan: "lee el plan",
    writeOk: "escribe → r13 ✓",
    writeConflict: "escribe sobre r12 → conflicto",
    rereadMerge: "relee r13 y fusiona",
    writeRetry: "escribe → r14 ✓",
  },
  teams: {
    eyebrow: "Para equipos y empresas",
    title:
      "Tu constitución está protegida. Tu historial es permanente. Tus agentes están vigilados.",
    body: "Define los principios de la empresa una sola vez en una constitución de la organización; cada proyecto los hereda y puede añadir, nunca debilitar. Demuestra quién aprobó qué y da de baja a una persona en un solo paso.",
    overview: "Lee la descripción de seguridad →",
    items: [
      {
        title: "SSO y SCIM",
        body: "SAML u OIDC. Quien se va pierde el acceso automáticamente.",
      },
      {
        title: "Tokens de agente con alcance",
        body: "Se muestran una sola vez, se guardan como hash, vencen y se pueden revocar.",
      },
      {
        title: "Auditoría inmutable",
        body: "Cada escritura y cada evento de seguridad. Exportable y transmisible a tu SIEM.",
      },
      {
        title: "Aprobaciones y políticas",
        body: "Aprobadores obligatorios, vigencia de tokens y listas de IP permitidas.",
      },
      {
        title: "Cifrado y aislamiento",
        body: "TLS en todas partes, cifrado en reposo y autorización en cada llamada.",
      },
      {
        title: "Residencia de datos",
        body: "Regiones UE o EE. UU. Despliegue dedicado disponible.",
      },
    ],
  },
  portable: {
    eyebrow: "Abierto y portable",
    title: "Markdown de entrada, markdown de salida. Exporta cuando quieras.",
    body: "Importa un árbol SDD existente con un solo comando, con una simulación previa. Exporta un proyecto o una sola spec de vuelta a archivos, byte por byte. Protocolo abierto, API y CLI documentados.",
    terminal: {
      documents: "documentos",
      summary: "0 problemas · ejecuta sin --dry-run para importar",
    },
  },
  pricing: {
    eyebrow: "Precios",
    title: "Los lectores y revisores son gratis en todos los planes de pago.",
    featured: "Para la mayoría de equipos",
    plans: [
      {
        name: "Free",
        price: "$0",
        who: "Para probar SDD con tu agente.",
        items: [
          "1 organización de un usuario",
          "Hasta 3 proyectos",
          "Servidor MCP completo",
          "Historial de 30 días",
        ],
        cta: "Empieza gratis",
      },
      {
        name: "Pro",
        price: "Por usuario",
        who: "Para desarrolladores individuales.",
        items: ["Proyectos ilimitados", "Historial ilimitado", "Invitados"],
        cta: "Empieza gratis",
      },
      {
        name: "Team",
        price: "Por puesto",
        who: "Para equipos que ejecutan agentes en paralelo.",
        items: [
          "Organizaciones y roles",
          "Aprobaciones",
          "Registro de auditoría",
          "Integraciones",
          "Soporte prioritario",
        ],
        cta: "Empieza gratis",
      },
      {
        name: "Enterprise",
        price: "A medida",
        who: "Para empresas con requisitos de cumplimiento.",
        items: [
          "SSO y SCIM",
          "Constitución de la organización y políticas",
          "Exportación a SIEM",
          "Residencia de datos",
          "Despliegue dedicado, SLA",
        ],
        cta: "Reserva una demo",
      },
    ],
  },
  faq: {
    title: "Preguntas",
    items: [
      {
        q: "¿Tengo que cambiar mi flujo de SDD?",
        a: "No. La constitución, la spec, el plan, las tareas, la investigación y la validación conservan su forma y siguen siendo markdown. Tu agente los lee y escribe mediante MCP en lugar del sistema de archivos.",
      },
      {
        q: "¿Qué agentes son compatibles?",
        a: "Cualquier agente compatible con MCP: Claude Code, Cursor, Windsurf, agentes de VS Code, JetBrains y agentes propios creados con SDK de agentes. Los equipos pueden combinar agentes.",
      },
      {
        q: "¿Puedo irme y llevarme mis specs?",
        a: "Sí. Exporta un proyecto o una sola spec a un árbol de carpetas SDD estándar en cualquier momento, byte por byte idéntico a lo que se importó.",
      },
      {
        q: "¿Entrenan IA con mis specs?",
        a: "No. Las funciones de IA nunca se entrenan con el contenido de los clientes, y el contenido jamás se comparte entre organizaciones.",
      },
      {
        q: "¿Qué pasa si dos agentes escriben al mismo tiempo?",
        a: "Cada escritura indica la revisión en la que se basa. Una escritura gana; la otra recibe un conflicto con la revisión actual, vuelve a leer, fusiona y reintenta. Nada se sobrescribe en silencio.",
      },
      {
        q: "¿Puedo seguir guardando las specs en git?",
        a: "Sí. La próxima app de GitHub y GitLab podrá reflejar las specs en una rama para los equipos que también las quieran en el repositorio.",
      },
      {
        q: "¿Dónde se almacenan mis datos?",
        a: "En la región que elijas, UE o EE. UU. Los planes Enterprise pueden usar un despliegue dedicado y aislado con claves administradas por el cliente.",
      },
    ],
  },
  closing: { title: "Dale a tus agentes un contrato que no puedan reescribir." },
  footer: {
    label: "Pie de página",
    security: "Seguridad",
    docs: "Documentación",
    dpa: "DPA",
    disclosure: "Divulgación responsable",
  },
};
