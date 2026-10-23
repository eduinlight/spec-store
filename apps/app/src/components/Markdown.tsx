import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

export function Markdown({ body }: { body: string }) {
  return (
    <div className="markdown">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
    </div>
  );
}
