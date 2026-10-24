export function projectParams(params: Record<string, string | undefined>) {
  return { org: params.org ?? "", project: params.project ?? "" };
}
