import { Button } from "@workspace/ui/components/button";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Markdown } from "~/components/Markdown";

export function DocumentView({ body }: { body: string }) {
  const { t } = useTranslation();
  const [raw, setRaw] = useState(false);
  return (
    <div className="grid gap-3">
      <div className="flex gap-2">
        <Button size="sm" variant={raw ? "outline" : "default"} onClick={() => setRaw(false)}>
          {t("spec.rendered")}
        </Button>
        <Button size="sm" variant={raw ? "default" : "outline"} onClick={() => setRaw(true)}>
          {t("spec.raw")}
        </Button>
      </div>
      {raw ? (
        <pre
          data-testid="raw"
          className="overflow-x-auto whitespace-pre-wrap rounded-md bg-muted p-4 text-sm"
        >
          {body}
        </pre>
      ) : (
        <Markdown body={body} />
      )}
    </div>
  );
}
