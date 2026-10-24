export function commitUrl(repositoryUrl: string | null | undefined, commit: string): string | null {
  return repositoryUrl ? `${repositoryUrl.replace(/\/$/, "")}/commit/${commit}` : null;
}
