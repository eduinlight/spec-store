import { ApiFailure, api, succeeded, unwrap } from "~/api/client";
import type { ImportTree } from "./sdd-tree";

export interface NewProjectInput {
  slug: string;
  name: string;
  repository: string;
}

export const VALIDATION_STATUS = 422;

export async function createEmptyProject(org: string, input: NewProjectInput): Promise<void> {
  unwrap(
    await api.POST("/api/v1/orgs/{org}/projects", {
      params: { path: { org } },
      body: { slug: input.slug, name: input.name },
    }),
  );
  if (input.repository) {
    succeeded(
      await api.PUT("/api/v1/projects/{org}/{project}/repository-url", {
        params: { path: { org, project: input.slug } },
        body: { url: input.repository },
      }),
    );
  }
}

export async function copyProject(org: string, source: string, input: NewProjectInput) {
  unwrap(
    await api.POST("/api/v1/projects/{org}/{project}/copy", {
      params: { path: { org, project: source } },
      body: {
        slug: input.slug,
        name: input.name,
        repository_url: input.repository || null,
      },
    }),
  );
}

export const problemLines = (message: string): string[] =>
  message.split("\n").filter((line) => line.trim().length > 0);

export async function importTree(
  org: string,
  project: string,
  tree: ImportTree,
  dryRun: boolean,
): Promise<string[]> {
  try {
    succeeded(
      await api.POST("/api/v1/projects/{org}/{project}/import", {
        params: { path: { org, project } },
        body: { tree, dry_run: dryRun } as unknown as Record<string, never>,
      }),
    );
    return [];
  } catch (error) {
    if (error instanceof ApiFailure && error.status === VALIDATION_STATUS) {
      return problemLines(error.message);
    }
    throw error;
  }
}
