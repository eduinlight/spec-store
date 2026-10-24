export type Card = { title: string; body: string };

export type Plan = {
  name: string;
  price: string;
  who: string;
  items: string[];
  cta: string;
};

export type FaqItem = { q: string; a: string };

export type LandingCopy = {
  meta: { title: string; description: string };
  cta: { startFree: string; signIn: string; bookDemo: string };
  nav: {
    logoAlt: string;
    label: string;
    howItWorks: string;
    features: string;
    security: string;
    pricing: string;
    faq: string;
    language: string;
    languageHref: string;
  };
  hero: {
    badge: string;
    headline: string;
    body: string;
    worksWith: string;
    agents: string[];
    terminal: {
      prompt: string;
      approved: string;
      phase: string;
      nextTask: string;
      noMarkers: string;
    };
    card: {
      title: string;
      status: string;
      phase: string;
      tasks: string;
      actor: string;
      action: string;
      done: string;
      ago: string;
    };
  };
  problem: {
    eyebrow: string;
    title: string;
    cards: { tag: string; title: string; body: string }[];
  };
  how: {
    eyebrow: string;
    title: string;
    steps: Card[];
  };
  features: {
    eyebrow: string;
    title: string;
    cards: [Card, Card, Card, Card, Card, Card];
    states: [string, string, string, string];
    revisions: [string, string, string];
    parallelNote: string;
    roles: [string, string, string];
    tokens: {
      readWrite: string;
      readOnly: string;
      org: string;
      expires: string;
      revoked: string;
    };
  };
  guardrails: {
    eyebrow: string;
    title: string;
    body: string;
    emphasis: string;
    refusal: string;
    audit: { denied: string; detail: string };
  };
  concurrency: {
    eyebrow: string;
    title: string;
    body: string;
    agentA: string;
    agentB: string;
    readPlan: string;
    writeOk: string;
    writeConflict: string;
    rereadMerge: string;
    writeRetry: string;
  };
  teams: {
    eyebrow: string;
    title: string;
    body: string;
    overview: string;
    items: Card[];
  };
  portable: {
    eyebrow: string;
    title: string;
    body: string;
    terminal: { documents: string; summary: string };
  };
  pricing: {
    eyebrow: string;
    title: string;
    featured: string;
    plans: Plan[];
  };
  faq: { title: string; items: FaqItem[] };
  closing: { title: string };
  footer: {
    label: string;
    security: string;
    docs: string;
    dpa: string;
    disclosure: string;
  };
};
