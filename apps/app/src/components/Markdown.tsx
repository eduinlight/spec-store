import { Tooltip } from "@workspace/ui/taptap";
import { Children, type ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { slugify } from "~/lib/markdown";

interface MarkdownProps {
  body: string;
  requirementTips?: ReadonlyMap<string, ReactNode>;
}

const textOf = (node: ReactNode): string =>
  Children.toArray(node)
    .map((child) => {
      if (typeof child === "string" || typeof child === "number") return String(child);
      if (typeof child === "object" && "props" in child) {
        return textOf((child.props as { children?: ReactNode }).children);
      }
      return "";
    })
    .join("");

const PROSE = [
  "markdown",
  "[&_h2]:mb-3 [&_h2]:mt-8 [&_h2]:text-2xl [&_h2]:font-medium [&_h2]:leading-8 [&_h2:first-child]:mt-0",
  "[&_h3]:mb-2 [&_h3]:mt-6 [&_h3]:text-lg [&_h3]:font-medium [&_h3]:leading-[26px]",
  "[&_h4]:mb-2 [&_h4]:mt-4 [&_h4]:text-base [&_h4]:font-medium",
  "[&_p]:mb-5 [&_p]:text-pretty",
  "[&_ul]:mb-5 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:mb-5 [&_ol]:list-decimal [&_ol]:pl-5 [&_li]:my-1.5",
  "[&_a]:text-[var(--primary-700)] [&_a]:underline",
  "[&_code]:rounded-sm [&_code]:bg-[var(--neutral-100)] [&_code]:px-1.5 [&_code]:py-px [&_code]:font-mono [&_code]:text-sm",
  "[&_pre]:mb-5 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-[var(--neutral-50)] [&_pre]:p-4 [&_pre_code]:bg-transparent [&_pre_code]:p-0",
  "[&_blockquote]:mb-5 [&_blockquote]:border-l-4 [&_blockquote]:border-[var(--neutral-300)] [&_blockquote]:pl-4 [&_blockquote]:text-[var(--neutral-600)]",
  "[&_table]:mb-5 [&_table]:w-full [&_table]:border-collapse [&_table]:text-sm [&_th]:border-b [&_th]:border-[var(--neutral-300)] [&_th]:px-2 [&_th]:py-1.5 [&_th]:text-left [&_td]:border-b [&_td]:border-[var(--neutral-200)] [&_td]:px-2 [&_td]:py-1.5",
  "[&_hr]:my-6 [&_hr]:border-[var(--neutral-200)]",
].join(" ");

export function Markdown({ body, requirementTips }: MarkdownProps) {
  const components: Components = {
    h1: ({ children }) => <h2>{children}</h2>,
    h2: ({ children }) => <h2 id={slugify(textOf(children))}>{children}</h2>,
    h3: ({ children }) => <h3 id={slugify(textOf(children))}>{children}</h3>,
  };
  if (requirementTips && requirementTips.size > 0) {
    components.strong = ({ children }) => {
      const tip = requirementTips.get(textOf(children).trim());
      if (tip === undefined) return <strong>{children}</strong>;
      return (
        <Tooltip title={tip} placement="right">
          <strong
            // biome-ignore lint/a11y/noNoninteractiveTabindex: the requirement id reveals its tasks on focus as well as on hover
            tabIndex={0}
            style={{ cursor: "help" }}
          >
            {children}
          </strong>
        </Tooltip>
      );
    };
  }
  return (
    <div className={PROSE}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {body}
      </ReactMarkdown>
    </div>
  );
}

export function InlineMarkdown({ body }: { body: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{ p: ({ children }) => <>{children}</> }}
    >
      {body}
    </ReactMarkdown>
  );
}
