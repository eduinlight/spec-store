import { alternates, Landing, metaFor } from "~/landing";

export function meta() {
  return metaFor("es");
}

export function links() {
  return alternates();
}

export default function HomeEs() {
  return <Landing language="es" />;
}
