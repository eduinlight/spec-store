import { type LoaderFunctionArgs, redirect } from "react-router";
import { api, unwrap } from "~/api/client";
import { guard } from "~/lib/session";

export async function loader({ request }: LoaderFunctionArgs) {
  const organizations = await guard(
    request,
    async () => unwrap(await api.GET("/api/v1/orgs")).organizations,
  );
  const first = organizations[0];
  return first ? redirect(`/o/${first.slug}`) : redirect("/onboarding");
}

export function Home() {
  return null;
}
