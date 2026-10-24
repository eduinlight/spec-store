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

export function Markdown({ body, requirementTips }: MarkdownProps) {
  const components: Components = {
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
    <div className="markdown">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {body}
      </ReactMarkdown>
    </div>
  );
}
