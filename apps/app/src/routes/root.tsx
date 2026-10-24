import { Button } from "@workspace/ui/components/button";
import { useTranslation } from "react-i18next";
import {
  isRouteErrorResponse,
  Link,
  type LoaderFunctionArgs,
  Outlet,
  useLoaderData,
  useNavigate,
  useParams,
  useRouteError,
} from "react-router";
import { ApiFailure, api, unwrap } from "~/api/client";
import { switchLanguage } from "~/lib/i18n";
import type { Language } from "~/lib/language";
import { guard, requireSession } from "~/lib/session";

export async function loader({ request }: LoaderFunctionArgs) {
  const me = await requireSession(request);
  const organizations = await guard(
    request,
    async () => unwrap(await api.GET("/api/v1/orgs")).organizations,
  );
  return { me, organizations };
}

export type RootData = Awaited<ReturnType<typeof loader>>;

async function signOut() {
  const result = await api.POST("/auth/logout");
  if (result.data?.logout_url) {
    window.location.assign(result.data.logout_url);
  } else {
    window.location.assign("/");
  }
}

export function Root() {
  const { me, organizations } = useLoaderData() as RootData;
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const params = useParams();
  const current = params.org ?? "";
  return (
    <div className="min-h-screen bg-muted/40">
      <header className="border-b bg-background">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-4 px-6 py-3">
          <Link to="/" className="flex items-center gap-2 font-semibold">
            <img src="/logo-horizontal.png" alt={t("app.name")} className="h-7 w-auto" />
          </Link>
          {organizations.length > 0 && (
            <label className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{t("app.switchOrganization")}</span>
              <select
                aria-label={t("app.switchOrganization")}
                className="rounded-md border bg-background px-2 py-1"
                value={current}
                onChange={(event) => navigate(`/o/${event.target.value}`)}
              >
                {current === "" && <option value="">—</option>}
                {organizations.map((organization) => (
                  <option key={organization.slug} value={organization.slug}>
                    {organization.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          <div className="ml-auto flex items-center gap-3 text-sm">
            <label className="flex items-center gap-2">
              <span className="sr-only">{t("app.language")}</span>
              <select
                aria-label={t("app.language")}
                className="rounded-md border bg-background px-2 py-1"
                value={i18n.language}
                onChange={(event) => {
                  const language = event.target.value as Language;
                  void switchLanguage(i18n, language);
                  void api.PUT("/api/v1/me/language", { body: { language } });
                }}
              >
                <option value="en">{t("app.languages.en")}</option>
                <option value="es">{t("app.languages.es")}</option>
              </select>
            </label>
            <Link to="/profile" className="hover:underline">
              {me.handle}
              {me.is_operator && (
                <span className="ml-1 text-muted-foreground">({t("app.operator")})</span>
              )}
            </Link>
            <Button variant="outline" size="sm" onClick={() => void signOut()}>
              {t("app.signOut")}
            </Button>
          </div>
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-6 py-8">
        <Outlet />
      </main>
    </div>
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
