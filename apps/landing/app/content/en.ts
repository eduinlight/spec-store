import type { LandingCopy } from "./types";

export const en: LandingCopy = {
  meta: {
    title: "SDD Vault — Specs your AI agents follow, and can't rewrite",
    description:
      "Store your constitution, specs, plans and tasks in one secure, versioned place. Serve them to every AI agent through MCP. Work in parallel without conflicts.",
  },
  cta: { startFree: "Start free", signIn: "Sign in", bookDemo: "Book a demo" },
  nav: {
    logoAlt: "SDD Vault",
    label: "Sections",
    howItWorks: "How it works",
    features: "Features",
    security: "Security",
    pricing: "Pricing",
    faq: "FAQ",
    language: "Español",
    languageHref: "/es",
  },
  hero: {
    badge: "MCP server for Spec Driven Development",
    headline: "Specs your AI agents follow, and can't rewrite.",
    body: "Store your constitution, specs, plans and tasks in one secure, versioned place. Serve them to every AI agent through MCP. Work in parallel without conflicts.",
    worksWith: "Works with any MCP-compatible agent",
    agents: ["Claude Code", "Cursor", "Windsurf", "VS Code", "JetBrains", "Custom agents"],
    terminal: {
      prompt: "show the project status",
      approved: "approved",
      phase: "phase 3",
      nextTask: "Migrate refunds table",
      noMarkers: "0 open",
    },
    card: {
      title: "Diesel ORM migration",
      status: "Approved",
      phase: "Phase 3 · Implementation",
      tasks: "12 / 20 tasks",
      actor: "Marco",
      action: "Claude Code marked",
      done: "done",
      ago: "2 min ago",
    },
  },
  problem: {
    eyebrow: "The problem",
    title: "Specs became the contract with your agents. They still live as loose files.",
    cards: [
      {
        tag: "constitution.md · modified",
        title: "Agents edit the rules",
        body: "Any agent with file access can rewrite a principle in the same commit as the code that breaks it. Asked to make a test pass, it can change the spec instead of the code.",
      },
      {
        tag: "CONFLICT (content): tasks.md",
        title: "Specs conflict in parallel branches",
        body: 'Two agents mark different tasks done on two branches. The merge conflicts, or merges silently and loses one change. "The current spec" depends on your branch.',
      },
      {
        tag: "Status: Approved (?)",
        title: "Nobody knows what's approved",
        body: '"Approved" is a word in a markdown header. Nothing prevents implementing a draft, and product managers need git access just to read it.',
      },
    ],
  },
  how: {
    eyebrow: "How it works",
    title: "From sign-up to first agent call in under three minutes.",
    steps: [
      {
        title: "Connect your agent",
        body: "Paste one command and approve access in your browser. Claude Code, Cursor or any MCP client.",
      },
      {
        title: "Write specs with your agent",
        body: "The server assigns the next spec number. Open questions become markers, never forgotten.",
      },
      {
        title: "Approve as a team",
        body: "Product, design and QA read the rendered spec, answer markers and approve without git.",
      },
      {
        title: "Implement in parallel",
        body: "Agents pick parallel tasks. Each completion is recorded with its commit, tracked to the FR.",
      },
    ],
  },
  features: {
    eyebrow: "Features",
    title: "One source of truth for every branch, every developer, every agent.",
    cards: [
      {
        title: "Governed",
        body: "A protected constitution only owners can change. Status transitions follow SDD, and approvals are recorded with name and time.",
      },
      {
        title: "Versioned",
        body: "Every write is an immutable revision with author, agent and message. Compare any two, restore any one. Nothing is lost.",
      },
      {
        title: "Parallel",
        body: "No merge conflicts on specs. Optimistic locking on every document and atomic task completion. Ten agents, one spec.",
      },
      {
        title: "AI-native",
        body: "Ask for the next task, not the whole file. Structured answers, fewer tokens, and guardrails returned as errors agents can act on.",
      },
      {
        title: "Multi-tenant",
        body: "Organizations, projects and roles per project. Invite guests to one project without exposing the rest of the organization.",
      },
      {
        title: "Secure",
        body: "SSO and SCIM, scoped tokens shown once, an immutable audit trail, encryption in transit and at rest, strict tenant isolation.",
      },
    ],
    states: ["Draft", "Clarified", "Approved", "Archived"],
    revisions: ["Clarify refund window", "Add FR-18 for partial refunds", "Initial plan"],
    parallelNote: "Both completed at 14:02:11. No conflict.",
    roles: ["Owner", "Editor", "Reader"],
    tokens: {
      readWrite: "read-write",
      readOnly: "read-only",
      org: "org",
      expires: "exp. 30d",
      revoked: "Revoked",
    },
  },
  guardrails: {
    eyebrow: "Guardrails",
    title: "Your agent can't rewrite the rules. Not even by accident.",
    body: "While fixing a failing test, an editor's agent tries to relax a constitution principle. The server refuses, the agent reports back to the developer, and the attempt lands in the audit trail.",
    emphasis: "Prompts are suggestions. SDD Vault rules are enforced.",
    refusal: 'Only project owners can write the constitution. Your role in "payments" is editor.',
    audit: {
      denied: "Denied",
      detail: "payments · origin 81.2.69.x · audit entry is immutable",
    },
  },
  concurrency: {
    eyebrow: "Concurrency",
    title: "Two agents, one document, zero lost changes.",
    body: "Every write declares the revision it is based on. When two agents write from the same revision, exactly one wins. The other is told the current revision, re-reads, merges and retries. Both changes are kept, both attributed.",
    agentA: "Agent A · Claude Code",
    agentB: "Agent B · Cursor",
    readPlan: "read plan",
    writeOk: "write → r13 ✓",
    writeConflict: "write on r12 → conflict",
    rereadMerge: "re-read r13, merge",
    writeRetry: "write → r14 ✓",
  },
  teams: {
    eyebrow: "For teams and companies",
    title: "Your constitution is locked. Your history is permanent. Your agents are guarded.",
    body: "Define company principles once in an organization constitution; every project inherits them and can add, never weaken. Prove who approved what, and offboard people in one step.",
    overview: "Read the security overview →",
    items: [
      {
        title: "SSO and SCIM",
        body: "SAML or OIDC. People who leave lose access automatically.",
      },
      {
        title: "Scoped agent tokens",
        body: "Shown once, stored as a hash, expiring and revocable.",
      },
      {
        title: "Immutable audit trail",
        body: "Every write and security event. Exportable, streamable to your SIEM.",
      },
      {
        title: "Approvals and policies",
        body: "Required approvers, token lifetime, IP allowlists.",
      },
      {
        title: "Encryption and isolation",
        body: "TLS everywhere, encrypted at rest, authorized on every call.",
      },
      {
        title: "Data residency",
        body: "EU or US regions. Dedicated deployment available.",
      },
    ],
  },
  portable: {
    eyebrow: "Open and portable",
    title: "Markdown in, markdown out. Export anytime.",
    body: "Import an existing SDD tree in one command, with a dry run first. Export a project or a single spec back to files, byte for byte. Open protocol, documented API and CLI.",
    terminal: {
      documents: "documents",
      summary: "0 problems · run without --dry-run to import",
    },
  },
  pricing: {
    eyebrow: "Pricing",
    title: "Readers and reviewers are free on every paid plan.",
    featured: "Most teams",
    plans: [
      {
        name: "Free",
        price: "$0",
        who: "For trying SDD with your agent.",
        items: ["1 user organization", "Up to 3 projects", "Full MCP server", "30-day history"],
        cta: "Start free",
      },
      {
        name: "Pro",
        price: "Per user",
        who: "For individual developers.",
        items: ["Unlimited projects", "Unlimited history", "Guests"],
        cta: "Start free",
      },
      {
        name: "Team",
        price: "Per seat",
        who: "For teams running agents in parallel.",
        items: [
          "Organizations and roles",
          "Approvals",
          "Audit trail",
          "Integrations",
          "Priority support",
        ],
        cta: "Start free",
      },
      {
        name: "Enterprise",
        price: "Custom",
        who: "For companies with compliance needs.",
        items: [
          "SSO and SCIM",
          "Org constitution and policies",
          "SIEM export",
          "Data residency",
          "Dedicated deployment, SLA",
        ],
        cta: "Book a demo",
      },
    ],
  },
  faq: {
    title: "Questions",
    items: [
      {
        q: "Do I need to change my SDD workflow?",
        a: "No. Constitution, spec, plan, tasks, research and validation keep their shape and stay markdown. Your agent reads and writes them through MCP instead of the file system.",
      },
      {
        q: "Which agents are supported?",
        a: "Any MCP-compatible agent: Claude Code, Cursor, Windsurf, VS Code agents, JetBrains and custom agents built on agent SDKs. Teams can mix agents.",
      },
      {
        q: "Can I leave and take my specs?",
        a: "Yes. Export a project or a single spec to a standard SDD folder tree at any time, byte for byte identical to what was imported.",
      },
      {
        q: "Do you train AI on my specs?",
        a: "No. AI features never train on customer content, and content is never shared across organizations.",
      },
      {
        q: "What happens if two agents write at the same time?",
        a: "Every write names the revision it is based on. One write wins; the other receives a conflict with the current revision, re-reads, merges and retries. Nothing is silently overwritten.",
      },
      {
        q: "Can I still keep specs in git?",
        a: "Yes. The upcoming GitHub and GitLab app can mirror specs back to a branch for teams that also want them in the repository.",
      },
      {
        q: "Where is my data stored?",
        a: "In the region you choose, EU or US. Enterprise plans can use a dedicated isolated deployment with customer-managed keys.",
      },
    ],
  },
  closing: { title: "Give your agents a contract they can't rewrite." },
  footer: {
    label: "Footer",
    security: "Security",
    docs: "Docs",
    dpa: "DPA",
    disclosure: "Responsible disclosure",
  },
};
