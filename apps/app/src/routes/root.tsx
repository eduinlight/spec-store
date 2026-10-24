import { useTranslation } from "react-i18next";
import {
  isRouteErrorResponse,
  Link,
  type LoaderFunctionArgs,
  Outlet,
  useLoaderData,
  useRouteError,
} from "react-router";
import { ApiFailure, api, unwrap } from "~/api/client";
import { guard, requireSession } from "~/lib/session";
import { ConfirmProvider } from "~/shell/confirm";
import { type RawShellData, toShellData } from "~/shell/data";
import { Shell } from "~/shell/Shell";

async function lenient<T>(request: Request, load: () => Promise<T>): Promise<T | null> {
  try {
    return await guard(request, load);
  } catch (error) {
    if (error instanceof ApiFailure) return null;
    throw error;
  }
}

export async function loader({ request, params }: LoaderFunctionArgs) {
  const me = await requireSession(request);
  const organizations = await guard(
    request,
    async () => unwrap(await api.GET("/api/v1/orgs")).organizations,
  );
  const { org, project } = params;
  const [projects, status] = await Promise.all([
    org
      ? lenient(
          request,
          async () =>
            unwrap(await api.GET("/api/v1/orgs/{org}/projects", { params: { path: { org } } }))
              .projects,
        )
      : null,
    org && project
      ? lenient(request, async () =>
          unwrap(
            await api.GET("/api/v1/projects/{org}/{project}/status", {
              params: { path: { org, project } },
            }),
          ),
        )
      : null,
  ]);
  const shell: RawShellData = { me, organizations, projects: projects ?? [], status };
  return shell;
}

export type RootData = Awaited<ReturnType<typeof loader>>;

export function Root() {
  const data = useLoaderData() as RootData;
  return (
    <ConfirmProvider>
      <Shell data={toShellData(data)}>
        <Outlet />
      </Shell>
    </ConfirmProvider>
  );
}

export function RouteError() {
  const error = useRouteError();
  const { t } = useTranslation();
  const status =
    error instanceof ApiFailure ? error.status : isRouteErrorResponse(error) ? error.status : 500;
  const message =
    status === 404
      ? t("errors.notFound")
      : status === 403
        ? t("errors.forbidden")
        : status === 503
          ? t("errors.unavailable")
          : t("errors.unexpected");
  return (
    <div className="mx-auto max-w-xl px-6 py-16 text-center">
      <p role="alert" className="text-lg">
        {message}
      </p>
      <Link to="/" className="mt-6 inline-block text-primary hover:underline">
        {t("app.back")}
      </Link>
    </div>
  );
}
